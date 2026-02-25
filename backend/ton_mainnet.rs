use candid::Nat;
use ic_cdk::management_canister::{
    self, HttpMethod, SchnorrAlgorithm, SchnorrKeyId, SignWithSchnorrArgs,
};
use num_bigint::BigUint;
use serde_json::{json, Value};

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::sdk::{evm_tx, ton_tx};
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, ConfiguredTokenResponse,
    TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::TON_MAINNET;
const TON_DECIMALS: u8 = 9;
const TON_WALLET_MODE_DEFAULT: u8 = 3;
const TON_VALID_UNTIL_SECS_AHEAD: u64 = 300;
const TON_JETTON_FORWARD_AMOUNT_NANOTON: u64 = 1;
const TON_JETTON_ATTACHED_NANOTON_DEFAULT: u64 = 100_000_000; // 0.1 TON

pub async fn request_address() -> WalletResult<AddressResponse> {
    let (pubkey, key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for TON address: {}",
            pubkey.len()
        )));
    }
    let mut pubkey32 = [0u8; 32];
    pubkey32.copy_from_slice(&pubkey);
    let code = ton_tx::wallet_v4r2_code_cell()?;
    let data = ton_tx::wallet_v4r2_data_cell(&pubkey32, ton_tx::TON_WALLET_V4R2_WALLET_ID)?;
    let state_init = ton_tx::state_init_cell(code, data)?;
    let raw_addr =
        ton_tx::contract_address_from_state_init(&state_init, ton_tx::TON_WORKCHAIN_BASECHAIN);
    let address = ton_tx::format_user_friendly_address(&raw_addr, false, false); // non-bounceable for receiving
    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: address.clone(),
        public_key_hex: addressing::hex_encode(&pubkey),
        key_name,
        message: Some(format!(
            "TON wallet v4r2 address (raw {})",
            ton_tx::format_raw_ton_address(&raw_addr)
        )),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    let owner_addr = ton_tx::parse_ton_address(&req.account)?;
    let owner_text = ton_tx::format_user_friendly_address(&owner_addr, false, false);
    let token_opt = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if token_opt.is_none() {
        let payload = ton_v2_get_json(&format!(
            "/getAddressBalance?address={}",
            percent_encode(&owner_text)
        ))
        .await?;
        let result = payload
            .get("result")
            .ok_or_else(|| WalletError::Internal("TON RPC missing result field".into()))?;
        let nanotons = parse_biguint_json_decimal(result)
            .ok_or_else(|| WalletError::Internal("TON balance parse failed".into()))?;
        return Ok(BalanceResponse {
            network: NETWORK_NAME.to_string(),
            account: owner_text,
            token: None,
            amount: Some(evm_tx::format_units(&nanotons, usize::from(TON_DECIMALS))),
            decimals: Some(TON_DECIMALS),
            block_ref: None,
            pending: false,
            message: Some("TON RPC getAddressBalance".to_string()),
        });
    }

    let token_master = ton_tx::parse_ton_address(token_opt.unwrap_or_default())?;
    let jetton_wallet = fetch_jetton_wallet_for_owner(&owner_addr, &token_master).await?;
    let decimals = resolve_jetton_decimals(&token_master).await.unwrap_or(9);

    let (amount_raw, jetton_wallet_addr_text) = match jetton_wallet {
        Some(info) => (info.balance, info.address_text),
        None => (BigUint::from(0u8), String::from("")),
    };

    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: owner_text,
        token: Some(ton_tx::format_user_friendly_address(
            &token_master,
            false,
            false,
        )),
        amount: Some(evm_tx::format_units(&amount_raw, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some(if jetton_wallet_addr_text.is_empty() {
            "TON v3 jetton/wallets (no wallet yet => balance 0)".to_string()
        } else {
            format!("TON v3 jetton/wallets ({jetton_wallet_addr_text})")
        }),
    })
}

pub async fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;

    let managed = fetch_managed_ton_wallet().await?;
    if let Some(from) = req.from.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let from_addr = ton_tx::parse_ton_address(from)?;
        if from_addr.workchain != managed.address.workchain
            || from_addr.hash != managed.address.hash
        {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed TON wallet address",
            ));
        }
    }

    let token_opt = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let out_msg = if token_opt.is_none() {
        build_ton_native_transfer_message(&managed, &req).await?
    } else {
        build_ton_jetton_transfer_message(&managed, &req).await?
    };

    let wallet_state = fetch_wallet_state(&managed.address).await?;
    let valid_until = ((ic_cdk::api::time() / 1_000_000_000) + TON_VALID_UNTIL_SECS_AHEAD) as u32;
    let signing_body = ton_tx::build_wallet_v4r2_signing_body(
        ton_tx::TON_WALLET_V4R2_WALLET_ID,
        valid_until,
        wallet_state.seqno,
        TON_WALLET_MODE_DEFAULT,
        out_msg,
    )?;
    let signing_hash = ton_tx::cell_hash(&signing_body);
    let signature = sign_ton_hash(&signing_hash).await?;
    let body = ton_tx::build_wallet_v4r2_body_with_signature(&signature, &signing_body)?;

    let state_init = if wallet_state.active {
        None
    } else {
        Some(managed.state_init.clone())
    };
    let ext_message = ton_tx::build_external_message(&managed.address, body, state_init)?;
    let boc_b64 = ton_tx::cell_to_boc_base64(&ext_message)?;
    let send_res = ton_v2_post_json("/sendBocReturnHash", json!({ "boc": boc_b64 })).await;
    let (tx_id, message) = match send_res {
        Ok(v) => {
            let hash = extract_send_boc_hash(&v).unwrap_or_else(|| String::from("accepted"));
            (
                Some(hash.clone()),
                format!("TON sendBocReturnHash accepted: {hash}"),
            )
        }
        Err(primary_err) => {
            // Fallback for providers that don't expose sendBocReturnHash.
            ton_v2_post_json("/sendBoc", json!({ "boc": boc_b64 }))
                .await
                .map_err(|fallback_err| {
                    WalletError::Internal(format!(
                        "TON sendBocReturnHash failed: {:?}; sendBoc failed: {:?}",
                        primary_err, fallback_err
                    ))
                })?;
            (None, "TON sendBoc accepted".to_string())
        }
    };

    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id,
        message,
    })
}

pub async fn discover_jetton_token(token_address: &str) -> WalletResult<ConfiguredTokenResponse> {
    let master = ton_tx::parse_ton_address(token_address)?;
    let canonical = ton_tx::format_user_friendly_address(&master, false, false);
    let decimals = resolve_jetton_decimals(&master).await.unwrap_or(9);
    let (symbol, name) = fetch_jetton_name_symbol_from_v3(&master)
        .await
        .unwrap_or_else(|_| {
            let suffix = canonical
                .get(canonical.len().saturating_sub(6)..)
                .unwrap_or(&canonical);
            (format!("JET{}", suffix), format!("Jetton {}", suffix))
        });
    Ok(ConfiguredTokenResponse {
        network: NETWORK_NAME.to_string(),
        symbol,
        name,
        token_address: canonical,
        decimals: u64::from(decimals),
    })
}

struct ManagedTonWallet {
    address: ton_tx::TonAddress,
    state_init: ton_tx::Cell,
}

#[derive(Clone, Debug)]
struct TonWalletState {
    seqno: u32,
    active: bool,
}

#[derive(Clone, Debug)]
struct JettonWalletInfo {
    address: ton_tx::TonAddress,
    address_text: String,
    balance: BigUint,
}

async fn fetch_managed_ton_wallet() -> WalletResult<ManagedTonWallet> {
    let (pubkey, _key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for TON wallet: {}",
            pubkey.len()
        )));
    }
    let mut pubkey32 = [0u8; 32];
    pubkey32.copy_from_slice(&pubkey);
    let code = ton_tx::wallet_v4r2_code_cell()?;
    let data = ton_tx::wallet_v4r2_data_cell(&pubkey32, ton_tx::TON_WALLET_V4R2_WALLET_ID)?;
    let state_init = ton_tx::state_init_cell(code, data)?;
    let address =
        ton_tx::contract_address_from_state_init(&state_init, ton_tx::TON_WORKCHAIN_BASECHAIN);
    Ok(ManagedTonWallet {
        address,
        state_init,
    })
}

async fn fetch_wallet_state(address: &ton_tx::TonAddress) -> WalletResult<TonWalletState> {
    let address_text = ton_tx::format_user_friendly_address(address, false, false);
    let path = format!(
        "/getWalletInformation?address={}",
        percent_encode(&address_text)
    );
    let payload = ton_v2_get_json_allow_rpc_not_ok(&path).await?;
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    if !ok {
        let msg = payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
            .to_lowercase();
        if msg.contains("not initialized")
            || msg.contains("cannot get seqno")
            || msg.contains("failed to execute get methods")
        {
            return Ok(TonWalletState {
                seqno: 0,
                active: false,
            });
        }
        return Err(WalletError::Internal(format!(
            "TON getWalletInformation failed: {}",
            payload
        )));
    }
    let result = payload.get("result").unwrap_or(&payload);
    let seqno = result.get("seqno").and_then(parse_u32_json).unwrap_or(0);
    let account_state = result
        .get("account_state")
        .and_then(Value::as_str)
        .or_else(|| result.get("state").and_then(Value::as_str))
        .unwrap_or("");
    let active = matches!(account_state, "active") || result.get("wallet").is_some();
    Ok(TonWalletState { seqno, active })
}

async fn build_ton_native_transfer_message(
    _managed: &ManagedTonWallet,
    req: &TransferRequest,
) -> WalletResult<ton_tx::Cell> {
    let to_addr = ton_tx::parse_ton_address(&req.to)?;
    let amount = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(TON_DECIMALS))?;
    if amount == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }
    let body = req
        .memo
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ton_tx::build_comment_body)
        .transpose()?;
    let bounce = to_addr.bounceable.unwrap_or(true);
    ton_tx::build_internal_message(&to_addr, &amount, bounce, body)
}

async fn build_ton_jetton_transfer_message(
    managed: &ManagedTonWallet,
    req: &TransferRequest,
) -> WalletResult<ton_tx::Cell> {
    let to_owner = ton_tx::parse_ton_address(&req.to)?;
    let token_master = ton_tx::parse_ton_address(
        req.token
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| WalletError::invalid_input("token is required for jetton transfer"))?,
    )?;
    let sender_jetton_wallet = fetch_jetton_wallet_for_owner(&managed.address, &token_master)
        .await?
        .ok_or_else(|| {
            WalletError::Internal(
                "sender jetton wallet not found (fund token first so wallet is created)".into(),
            )
        })?;
    let decimals = resolve_jetton_decimals(&token_master).await.unwrap_or(9);
    let amount_units = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
    if amount_units == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }

    let forward_amount = BigUint::from(TON_JETTON_FORWARD_AMOUNT_NANOTON);
    let body = ton_tx::build_jetton_transfer_body(
        &amount_units,
        &to_owner,
        &managed.address,
        &forward_amount,
        req.memo.as_deref(),
    )?;

    let attached = req
        .metadata
        .iter()
        .find_map(|(k, v)| {
            (k.eq_ignore_ascii_case("jetton_attached_ton")
                || k.eq_ignore_ascii_case("ton_attached"))
            .then_some(v.as_str())
        })
        .map(|v| evm_tx::parse_decimal_units(v.trim(), usize::from(TON_DECIMALS)))
        .transpose()?
        .unwrap_or_else(|| BigUint::from(TON_JETTON_ATTACHED_NANOTON_DEFAULT));
    if attached == BigUint::from(0u8) {
        return Err(WalletError::invalid_input(
            "attached TON for jetton transfer must be > 0",
        ));
    }

    ton_tx::build_internal_message(&sender_jetton_wallet.address, &attached, true, Some(body))
}

async fn fetch_jetton_wallet_for_owner(
    owner: &ton_tx::TonAddress,
    jetton_master: &ton_tx::TonAddress,
) -> WalletResult<Option<JettonWalletInfo>> {
    let owner_text = ton_tx::format_user_friendly_address(owner, false, false);
    let master_text = ton_tx::format_user_friendly_address(jetton_master, false, false);
    let path = format!(
        "/jetton/wallets?owner_address={}&jetton_address={}&limit=1&offset=0",
        percent_encode(&owner_text),
        percent_encode(&master_text)
    );
    let payload = ton_v3_get_json(&path).await?;
    let list = payload
        .get("jetton_wallets")
        .or_else(|| payload.get("result"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let Some(first) = list.first() else {
        return Ok(None);
    };
    let address_text = first
        .get("address")
        .and_then(Value::as_str)
        .or_else(|| first.get("wallet_address").and_then(Value::as_str))
        .ok_or_else(|| WalletError::Internal("TON v3 jetton wallet missing address".into()))?
        .to_string();
    let address = ton_tx::parse_ton_address(&address_text)?;
    let balance = first
        .get("balance")
        .and_then(parse_biguint_json_decimal)
        .unwrap_or_else(|| BigUint::from(0u8));
    Ok(Some(JettonWalletInfo {
        address,
        address_text,
        balance,
    }))
}

async fn resolve_jetton_decimals(jetton_master: &ton_tx::TonAddress) -> Option<u8> {
    if let Some(v) = configured_jetton_decimals(jetton_master) {
        return Some(v);
    }
    fetch_jetton_decimals_from_v3(jetton_master)
        .await
        .ok()
        .flatten()
}

fn configured_jetton_decimals(jetton_master: &ton_tx::TonAddress) -> Option<u8> {
    let raw = ton_tx::format_raw_ton_address(jetton_master).to_lowercase();
    let friendly = ton_tx::format_user_friendly_address(jetton_master, false, false).to_lowercase();
    config::token_list_config::configured_tokens(NETWORK_NAME)
        .iter()
        .find(|t| {
            let a = t.token_address.trim().to_lowercase();
            a == raw || a == friendly
        })
        .and_then(|t| u8::try_from(t.decimals).ok())
}

async fn fetch_jetton_decimals_from_v3(
    jetton_master: &ton_tx::TonAddress,
) -> WalletResult<Option<u8>> {
    let master_text = ton_tx::format_user_friendly_address(jetton_master, false, false);
    let path = format!("/jetton/masters/{}", percent_encode(&master_text));
    let payload = ton_v3_get_json(&path).await?;
    let candidates = [
        payload.get("metadata").and_then(|m| m.get("decimals")),
        payload
            .get("jetton_content")
            .and_then(|m| m.get("data"))
            .and_then(|m| m.get("decimals")),
        payload.get("decimals"),
    ];
    for c in candidates.into_iter().flatten() {
        if let Some(v) = parse_u8_json(c) {
            return Ok(Some(v));
        }
    }
    Ok(None)
}

async fn fetch_jetton_name_symbol_from_v3(
    jetton_master: &ton_tx::TonAddress,
) -> WalletResult<(String, String)> {
    let master_text = ton_tx::format_user_friendly_address(jetton_master, false, false);
    let path = format!("/jetton/masters/{}", percent_encode(&master_text));
    let payload = ton_v3_get_json(&path).await?;

    let symbol = find_first_non_empty_string(
        [
            payload.get("metadata").and_then(|m| m.get("symbol")),
            payload
                .get("jetton_content")
                .and_then(|m| m.get("data"))
                .and_then(|m| m.get("symbol")),
            payload.get("symbol"),
        ]
        .into_iter(),
    )
    .ok_or_else(|| WalletError::Internal("TON jetton metadata missing symbol".into()))?;

    let name = find_first_non_empty_string(
        [
            payload.get("metadata").and_then(|m| m.get("name")),
            payload
                .get("jetton_content")
                .and_then(|m| m.get("data"))
                .and_then(|m| m.get("name")),
            payload.get("name"),
        ]
        .into_iter(),
    )
    .unwrap_or_else(|| symbol.clone());

    Ok((symbol, name))
}

fn find_first_non_empty_string<'a>(
    values: impl Iterator<Item = Option<&'a Value>>,
) -> Option<String> {
    for value in values.flatten() {
        if let Some(s) = value.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            return Some(s.to_string());
        }
    }
    None
}

async fn sign_ton_hash(message_hash32: &[u8; 32]) -> WalletResult<Vec<u8>> {
    let key_name = config::app_config::default_schnorr_key_name().to_string();
    let args = SignWithSchnorrArgs {
        message: message_hash32.to_vec(),
        derivation_path: vec![],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Ed25519,
            name: key_name,
        },
        aux: None,
    };
    let res = management_canister::sign_with_schnorr(&args)
        .await
        .map_err(|err| WalletError::Internal(format!("TON sign_with_schnorr failed: {err}")))?;
    if res.signature.len() != 64 {
        return Err(WalletError::Internal(format!(
            "unexpected TON ed25519 signature length: {}",
            res.signature.len()
        )));
    }
    Ok(res.signature)
}

fn extract_send_boc_hash(payload: &Value) -> Option<String> {
    payload
        .get("result")
        .and_then(|r| {
            r.as_str().map(ToString::to_string).or_else(|| {
                r.get("hash")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
        })
        .or_else(|| {
            payload
                .get("hash")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
}

async fn ton_v2_get_json(path: &str) -> WalletResult<Value> {
    let payload = ton_http_json(ton_v2_url(path)?, HttpMethod::GET, None).await?;
    ton_check_ok_wrapper(payload)
}

async fn ton_v2_get_json_allow_rpc_not_ok(path: &str) -> WalletResult<Value> {
    ton_http_json(ton_v2_url(path)?, HttpMethod::GET, None).await
}

async fn ton_v2_post_json(path: &str, body: Value) -> WalletResult<Value> {
    let payload = ton_http_json(ton_v2_url(path)?, HttpMethod::POST, Some(body)).await?;
    ton_check_ok_wrapper(payload)
}

async fn ton_v3_get_json(path: &str) -> WalletResult<Value> {
    ton_http_json(ton_v3_url(path)?, HttpMethod::GET, None).await
}

fn ton_v2_url(path: &str) -> WalletResult<String> {
    let base = config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|e| WalletError::Internal(format!("ton rpc url resolution failed: {e}")))?;
    Ok(format!("{}{}", base.trim_end_matches('/'), path))
}

fn ton_v3_url(path: &str) -> WalletResult<String> {
    let base_v2 = config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|e| WalletError::Internal(format!("ton rpc url resolution failed: {e}")))?;
    let base_v3 = if let Some(prefix) = base_v2.strip_suffix("/api/v2") {
        format!("{prefix}/api/v3")
    } else if let Some(prefix) = base_v2.strip_suffix("/v2") {
        format!("{prefix}/v3")
    } else {
        format!("{}/api/v3", base_v2.trim_end_matches('/'))
    };
    Ok(format!("{}{}", base_v3.trim_end_matches('/'), path))
}

async fn ton_http_json(
    url: String,
    method: HttpMethod,
    body: Option<Value>,
) -> WalletResult<Value> {
    let body_bytes = body
        .map(|v| {
            serde_json::to_vec(&v).map_err(|err| {
                WalletError::Internal(format!("serialize TON http body failed: {err}"))
            })
        })
        .transpose()?;
    let http_res =
        crate::outcall::json_request(url, method, body_bytes, 1024 * 1024, "ton rpc").await?;
    if http_res.status != Nat::from(200u16) {
        let snippet: String = String::from_utf8_lossy(&http_res.body)
            .chars()
            .take(300)
            .collect();
        return Err(WalletError::Internal(format!(
            "TON HTTP status {}: {}",
            http_res.status, snippet
        )));
    }
    serde_json::from_slice::<Value>(&http_res.body)
        .map_err(|err| WalletError::Internal(format!("parse TON http json failed: {err}")))
}

fn ton_check_ok_wrapper(payload: Value) -> WalletResult<Value> {
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    if ok {
        Ok(payload)
    } else {
        let code = payload.get("code").cloned().unwrap_or(Value::Null);
        let msg = payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        Err(WalletError::Internal(format!(
            "TON RPC error code={code}: {msg}"
        )))
    }
}

fn parse_biguint_json_decimal(v: &Value) -> Option<BigUint> {
    match v {
        Value::String(s) => BigUint::parse_bytes(s.trim().as_bytes(), 10),
        Value::Number(n) => BigUint::parse_bytes(n.to_string().as_bytes(), 10),
        _ => None,
    }
}

fn parse_u32_json(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|x| u32::try_from(x).ok()),
        Value::String(s) => s.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn parse_u8_json(v: &Value) -> Option<u8> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|x| u8::try_from(x).ok()),
        Value::String(s) => s.trim().parse::<u8>().ok(),
        _ => None,
    }
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(hex_char((b >> 4) & 0x0f));
            out.push(hex_char(b & 0x0f));
        }
    }
    out
}

fn hex_char(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        10..=15 => (b'A' + (v - 10)) as char,
        _ => '0',
    }
}

fn validate_transfer(req: &TransferRequest) -> WalletResult<()> {
    if req.to.trim().is_empty() {
        return Err(WalletError::invalid_input("to is required"));
    }
    if req.amount.trim().is_empty() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    Ok(())
}
