use blake2::digest::{Update, VariableOutput};
use blake2::Blake2bVar;
use candid::Nat;
use ic_cdk::management_canister::{self, SchnorrAlgorithm, SchnorrKeyId, SignWithSchnorrArgs};
use num_bigint::BigUint;
use serde_json::{json, Value};

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::sdk::evm_tx;
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, ConfiguredTokenResponse,
    TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::SUI_MAINNET;
const SUI_DECIMALS: u8 = 9;
const SUI_COIN_TYPE: &str = "0x2::sui::SUI";
const SUI_ED25519_FLAG: u8 = 0x00;
const SUI_DEFAULT_GAS_BUDGET_NATIVE: u64 = 2_000_000;
const SUI_DEFAULT_GAS_BUDGET_TOKEN: u64 = 5_000_000;

pub async fn request_address() -> WalletResult<AddressResponse> {
    let (pubkey, key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for Sui address: {}",
            pubkey.len()
        )));
    }
    let address = sui_address_from_pubkey(&pubkey)?;
    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address,
        public_key_hex: addressing::hex_encode(&pubkey),
        key_name,
        message: Some("Sui address from blake2b(flag||ed25519_pubkey)".into()),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    let address = normalize_sui_address(req.account.trim())?;
    let coin_type = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut params = vec![Value::String(address.clone())];
    if let Some(t) = coin_type.clone() {
        params.push(Value::String(t));
    }
    let result = sui_rpc_call("suix_getBalance", Value::Array(params)).await?;
    let total = result
        .get("totalBalance")
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("Sui balance missing totalBalance".into()))?;
    let total_raw = BigUint::parse_bytes(total.as_bytes(), 10)
        .ok_or_else(|| WalletError::Internal("Sui totalBalance parse failed".into()))?;
    let decimals = if let Some(t) = coin_type.as_deref() {
        fetch_sui_coin_decimals(t).await.unwrap_or(SUI_DECIMALS)
    } else {
        SUI_DECIMALS
    };
    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: address,
        token: coin_type,
        amount: Some(evm_tx::format_units(&total_raw, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some("Sui JSON-RPC suix_getBalance".into()),
    })
}

pub async fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    let managed = fetch_managed_sui_identity().await?;
    if let Some(from) = req.from.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if normalize_sui_address(from)? != managed.address {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed Sui address",
            ));
        }
    }
    let to = normalize_sui_address(req.to.trim())?;
    let coin_type = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let decimals = if let Some(ref t) = coin_type {
        fetch_sui_coin_decimals(t).await.unwrap_or(SUI_DECIMALS)
    } else {
        SUI_DECIMALS
    };
    let amount = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
    if amount == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }
    let amount_u64 = biguint_to_u64(&amount)?;

    let gas_price = sui_rpc_call("suix_getReferenceGasPrice", Value::Array(vec![]))
        .await?
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1000);

    let gas_budget = if coin_type.is_some() {
        SUI_DEFAULT_GAS_BUDGET_TOKEN
    } else {
        SUI_DEFAULT_GAS_BUDGET_NATIVE
    };

    let tx_bytes_b64 = if let Some(coin) = coin_type {
        let token_coins = fetch_sui_coin_ids(&managed.address, &coin, amount_u64).await?;
        let gas_coin = select_sui_gas_coin(&managed.address, gas_budget, gas_price).await?;
        let params = json!([
            managed.address,
            token_coins,
            [to],
            [amount_u64.to_string()],
            gas_coin,
            gas_budget.to_string()
        ]);
        let built = sui_rpc_call("unsafe_pay", params).await?;
        extract_sui_tx_bytes(&built)?
    } else {
        let needed = amount_u64.saturating_add(gas_budget.saturating_mul(gas_price));
        let sui_coins = fetch_sui_coin_ids(&managed.address, SUI_COIN_TYPE, needed).await?;
        let params = json!([
            managed.address,
            sui_coins,
            [to],
            [amount_u64.to_string()],
            gas_budget.to_string()
        ]);
        let built = sui_rpc_call("unsafe_paySui", params).await?;
        extract_sui_tx_bytes(&built)?
    };

    let tx_bytes = base64_decode_std(&tx_bytes_b64)?;
    let digest = sui_intent_digest(&tx_bytes)?;
    let sig = sign_sui_digest(&digest).await?;
    let mut sui_sig = Vec::with_capacity(1 + 64 + 32);
    sui_sig.push(SUI_ED25519_FLAG);
    sui_sig.extend_from_slice(&sig);
    sui_sig.extend_from_slice(&managed.pubkey);
    let sui_sig_b64 = base64_encode_std(&sui_sig);

    let exec = sui_rpc_call(
        "sui_executeTransactionBlock",
        json!([
            tx_bytes_b64,
            [sui_sig_b64],
            { "showEffects": true },
            "WaitForLocalExecution"
        ]),
    )
    .await?;
    if let Some(status) = exec
        .get("effects")
        .and_then(|e| e.get("status"))
        .and_then(|s| s.get("status"))
        .and_then(Value::as_str)
    {
        if status != "success" {
            let err_text = exec
                .get("effects")
                .and_then(|e| e.get("status"))
                .and_then(|s| s.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(WalletError::Internal(format!(
                "Sui executeTransactionBlock failed: {err_text}"
            )));
        }
    }
    let tx_id = exec
        .get("digest")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: tx_id.clone(),
        message: tx_id
            .map(|h| format!("Sui executeTransactionBlock accepted: {h}"))
            .unwrap_or_else(|| "Sui executeTransactionBlock accepted".to_string()),
    })
}

pub async fn discover_coin_type_token(coin_type: &str) -> WalletResult<ConfiguredTokenResponse> {
    let coin_type = coin_type.trim().to_string();
    if coin_type.is_empty() {
        return Err(WalletError::invalid_input("coin type is required"));
    }
    let v = fetch_sui_coin_metadata_json(&coin_type).await?;
    let decimals = v
        .get("decimals")
        .and_then(|x| x.as_u64().and_then(|n| u8::try_from(n).ok()))
        .ok_or_else(|| WalletError::Internal("Sui coin metadata missing decimals".into()))?;
    let symbol = v
        .get("symbol")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| WalletError::Internal("Sui coin metadata missing symbol".into()))?
        .to_string();
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| symbol.clone());
    Ok(ConfiguredTokenResponse {
        network: NETWORK_NAME.to_string(),
        symbol,
        name,
        token_address: coin_type,
        decimals: u64::from(decimals),
    })
}

struct ManagedSuiIdentity {
    address: String,
    pubkey: [u8; 32],
}

async fn fetch_managed_sui_identity() -> WalletResult<ManagedSuiIdentity> {
    let (pubkey, _key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for Sui signer: {}",
            pubkey.len()
        )));
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&pubkey);
    Ok(ManagedSuiIdentity {
        address: sui_address_from_pubkey(&pubkey)?,
        pubkey: pk,
    })
}

async fn fetch_sui_coin_decimals(coin_type: &str) -> WalletResult<u8> {
    let v = fetch_sui_coin_metadata_json(coin_type).await?;
    v.get("decimals")
        .and_then(|x| x.as_u64().and_then(|n| u8::try_from(n).ok()))
        .ok_or_else(|| WalletError::Internal("Sui coin metadata missing decimals".into()))
}

async fn fetch_sui_coin_metadata_json(coin_type: &str) -> WalletResult<Value> {
    sui_rpc_call(
        "suix_getCoinMetadata",
        Value::Array(vec![Value::String(coin_type.trim().to_string())]),
    )
    .await
}

async fn fetch_sui_coin_ids(
    owner: &str,
    coin_type: &str,
    needed: u64,
) -> WalletResult<Vec<String>> {
    let v = sui_rpc_call("suix_getCoins", json!([owner, coin_type, Value::Null, 100])).await?;
    let data = v
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| WalletError::Internal("Sui suix_getCoins missing data".into()))?;
    let mut selected = Vec::new();
    let mut total = 0u128;
    for item in data {
        let id = item
            .get("coinObjectId")
            .and_then(Value::as_str)
            .ok_or_else(|| WalletError::Internal("Sui coin item missing coinObjectId".into()))?;
        let bal = item
            .get("balance")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);
        selected.push(id.to_string());
        total = total.saturating_add(bal);
        if total >= u128::from(needed) {
            return Ok(selected);
        }
    }
    Err(WalletError::Internal(format!(
        "insufficient Sui coin objects for {coin_type}: need {needed}, found {total}"
    )))
}

async fn select_sui_gas_coin(owner: &str, gas_budget: u64, gas_price: u64) -> WalletResult<String> {
    let need = u128::from(gas_budget).saturating_mul(u128::from(gas_price));
    let v = sui_rpc_call(
        "suix_getCoins",
        json!([owner, SUI_COIN_TYPE, Value::Null, 50]),
    )
    .await?;
    let data = v
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| WalletError::Internal("Sui suix_getCoins missing data".into()))?;
    let mut best: Option<(String, u128)> = None;
    for item in data {
        let id = item
            .get("coinObjectId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let bal = item
            .get("balance")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);
        if bal >= need {
            match &best {
                Some((_, best_bal)) if *best_bal >= bal => {}
                _ => best = Some((id.to_string(), bal)),
            }
        }
    }
    best.map(|x| x.0).ok_or_else(|| {
        WalletError::Internal(format!(
            "no SUI gas coin covers required gas budget {} @ price {}",
            gas_budget, gas_price
        ))
    })
}

fn extract_sui_tx_bytes(v: &Value) -> WalletResult<String> {
    v.get("txBytes")
        .or_else(|| v.get("tx_bytes"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| WalletError::Internal("Sui unsafe_* response missing txBytes".into()))
}

fn sui_address_from_pubkey(pubkey32: &[u8]) -> WalletResult<String> {
    if pubkey32.len() != 32 {
        return Err(WalletError::invalid_input("Sui pubkey must be 32 bytes"));
    }
    let mut h =
        Blake2bVar::new(32).map_err(|_| WalletError::Internal("Blake2b init failed".into()))?;
    h.update(&[SUI_ED25519_FLAG]);
    h.update(pubkey32);
    let mut out = [0u8; 32];
    h.finalize_variable(&mut out)
        .map_err(|_| WalletError::Internal("Blake2b finalize failed".into()))?;
    Ok(format!("0x{}", addressing::hex_encode(&out)))
}

fn sui_intent_digest(tx_bytes: &[u8]) -> WalletResult<[u8; 32]> {
    let mut h =
        Blake2bVar::new(32).map_err(|_| WalletError::Internal("Blake2b init failed".into()))?;
    h.update(&[0u8, 0u8, 0u8]); // IntentScope::TransactionData, IntentVersion::V0, AppId::Sui
    h.update(tx_bytes);
    let mut out = [0u8; 32];
    h.finalize_variable(&mut out)
        .map_err(|_| WalletError::Internal("Blake2b finalize failed".into()))?;
    Ok(out)
}

async fn sign_sui_digest(digest32: &[u8; 32]) -> WalletResult<Vec<u8>> {
    let key_name = config::app_config::default_schnorr_key_name().to_string();
    let args = SignWithSchnorrArgs {
        message: digest32.to_vec(),
        derivation_path: vec![],
        key_id: SchnorrKeyId {
            algorithm: SchnorrAlgorithm::Ed25519,
            name: key_name,
        },
        aux: None,
    };
    let res = management_canister::sign_with_schnorr(&args)
        .await
        .map_err(|err| WalletError::Internal(format!("sign_with_schnorr failed: {err}")))?;
    if res.signature.len() != 64 {
        return Err(WalletError::Internal(format!(
            "unexpected Sui signature length: {}",
            res.signature.len()
        )));
    }
    Ok(res.signature)
}

async fn sui_rpc_call(method: &str, params: Value) -> WalletResult<Value> {
    let base = config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|e| WalletError::Internal(format!("sui rpc url resolution failed: {e}")))?;
    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    }))
    .map_err(|e| WalletError::Internal(format!("serialize sui rpc body failed: {e}")))?;
    let res = crate::outcall::post_json(base, body, 1024 * 1024, "sui rpc").await?;
    if res.status != Nat::from(200u16) {
        let snippet: String = String::from_utf8_lossy(&res.body)
            .chars()
            .take(300)
            .collect();
        return Err(WalletError::Internal(format!(
            "sui rpc http status {}: {}",
            res.status, snippet
        )));
    }
    let payload: Value = serde_json::from_slice(&res.body)
        .map_err(|e| WalletError::Internal(format!("parse sui rpc response failed: {e}")))?;
    if let Some(err) = payload.get("error") {
        return Err(WalletError::Internal(format!("Sui RPC error: {err}")));
    }
    payload
        .get("result")
        .cloned()
        .ok_or_else(|| WalletError::Internal("Sui RPC missing result".into()))
}

fn normalize_sui_address(input: &str) -> WalletResult<String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(WalletError::invalid_input("Sui address is required"));
    }
    let no_prefix = s.strip_prefix("0x").unwrap_or(s);
    if no_prefix.is_empty() || !no_prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(WalletError::invalid_input("invalid Sui address hex"));
    }
    if no_prefix.len() > 64 {
        return Err(WalletError::invalid_input("Sui address is too long"));
    }
    let mut out = String::from("0x");
    for _ in 0..(64 - no_prefix.len()) {
        out.push('0');
    }
    out.push_str(&no_prefix.to_lowercase());
    Ok(out)
}

fn biguint_to_u64(v: &BigUint) -> WalletResult<u64> {
    let bytes = v.to_bytes_le();
    if bytes.len() > 8 {
        return Err(WalletError::invalid_input("amount is too large"));
    }
    let mut arr = [0u8; 8];
    arr[..bytes.len()].copy_from_slice(&bytes);
    Ok(u64::from_le_bytes(arr))
}

fn base64_encode_std(data: &[u8]) -> String {
    crate::sdk::ton_tx::base64_encode_std_nopad(data)
}

fn base64_decode_std(input: &str) -> WalletResult<Vec<u8>> {
    let mut text = input.trim().as_bytes().to_vec();
    text.retain(|b| !b" \n\r\t".contains(b));
    while text.len() % 4 != 0 {
        text.push(b'=');
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut i = 0usize;
    while i < text.len() {
        let c0 = b64val(text[i])?;
        let c1 = b64val(text[i + 1])?;
        let c2 = if text[i + 2] == b'=' {
            0
        } else {
            b64val(text[i + 2])?
        };
        let c3 = if text[i + 3] == b'=' {
            0
        } else {
            b64val(text[i + 3])?
        };
        let n =
            (u32::from(c0) << 18) | (u32::from(c1) << 12) | (u32::from(c2) << 6) | u32::from(c3);
        out.push(((n >> 16) & 0xff) as u8);
        if text[i + 2] != b'=' {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if text[i + 3] != b'=' {
            out.push((n & 0xff) as u8);
        }
        i += 4;
    }
    Ok(out)
}

fn b64val(c: u8) -> WalletResult<u8> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' | b'-' => Ok(62),
        b'/' | b'_' => Ok(63),
        _ => Err(WalletError::invalid_input("invalid base64 character")),
    }
}

fn validate_account(account: &str) -> WalletResult<()> {
    if account.trim().is_empty() {
        return Err(WalletError::invalid_input("account is required"));
    }
    Ok(())
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
