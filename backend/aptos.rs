use candid::Nat;
use ic_cdk::management_canister::{
    self, HttpMethod, SchnorrAlgorithm, SchnorrKeyId, SignWithSchnorrArgs,
};
use num_bigint::BigUint;
use serde_json::{json, Value};
use sha3::{Digest as Sha3Digest, Sha3_256};

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::sdk::evm_tx;
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::APTOS_MAINNET;
const APT_DECIMALS: u8 = 8;
const APTOS_COIN_TYPE: &str = "0x1::aptos_coin::AptosCoin";
const APTOS_TRANSFER_COINS_FN: &str = "0x1::aptos_account::transfer_coins";

pub async fn request_address() -> WalletResult<AddressResponse> {
    let (pubkey, key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for Aptos address: {}",
            pubkey.len()
        )));
    }
    let mut hasher = Sha3_256::new();
    hasher.update(&pubkey);
    hasher.update([0u8]); // Ed25519 scheme byte
    let auth_key = hasher.finalize();
    let address = format!("0x{}", addressing::hex_encode(&auth_key));
    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address,
        public_key_hex: addressing::hex_encode(&pubkey),
        key_name,
        message: Some("Aptos account address (auth key from ed25519 pubkey)".into()),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    let address = normalize_aptos_address(req.account.trim())?;
    let coin_type = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(APTOS_COIN_TYPE);

    let amount_raw = fetch_coin_balance_raw(&address, coin_type).await?;
    let decimals = if coin_type == APTOS_COIN_TYPE {
        APT_DECIMALS
    } else {
        fetch_coin_decimals(coin_type).await.unwrap_or(APT_DECIMALS)
    };

    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: address,
        token: if coin_type == APTOS_COIN_TYPE {
            None
        } else {
            Some(coin_type.to_string())
        },
        amount: Some(evm_tx::format_units(&amount_raw, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some("Aptos REST coin store resource".into()),
    })
}

pub async fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    let managed = fetch_managed_aptos_identity().await?;
    if let Some(from) = req.from.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let from_norm = normalize_aptos_address(from)?;
        if from_norm != managed.address {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed Aptos address",
            ));
        }
    }

    let to = normalize_aptos_address(req.to.trim())?;
    let coin_type = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(APTOS_COIN_TYPE)
        .to_string();
    let decimals = if coin_type == APTOS_COIN_TYPE {
        APT_DECIMALS
    } else {
        fetch_coin_decimals(&coin_type)
            .await
            .unwrap_or(APT_DECIMALS)
    };
    let amount_units = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
    if amount_units == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }
    let amount_u64 = biguint_to_u64(&amount_units)?;

    let account_info =
        aptos_get_json(&format!("/accounts/{}", path_encode(&managed.address))).await?;
    let sequence_number = account_info
        .get("sequence_number")
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("Aptos account missing sequence_number".into()))?
        .to_string();

    let ledger_info = aptos_get_json("/").await?;
    let chain_id = ledger_info
        .get("chain_id")
        .and_then(parse_u8_json)
        .ok_or_else(|| WalletError::Internal("Aptos ledger info missing chain_id".into()))?;

    let gas_info = aptos_get_json("/estimate_gas_price").await?;
    let gas_unit_price = gas_info
        .get("gas_estimate")
        .and_then(Value::as_u64)
        .or_else(|| {
            gas_info
                .get("deprioritized_gas_estimate")
                .and_then(Value::as_u64)
        })
        .unwrap_or(100);
    let max_gas_amount = if coin_type == APTOS_COIN_TYPE {
        20_000u64
    } else {
        80_000u64
    };
    let expiration = ((ic_cdk::api::time() / 1_000_000_000) + 600).to_string();

    let payload = json!({
        "type": "entry_function_payload",
        "function": APTOS_TRANSFER_COINS_FN,
        "type_arguments": [coin_type],
        "arguments": [to, amount_u64.to_string()]
    });

    let raw_tx_req = json!({
        "sender": managed.address,
        "sequence_number": sequence_number,
        "max_gas_amount": max_gas_amount.to_string(),
        "gas_unit_price": gas_unit_price.to_string(),
        "expiration_timestamp_secs": expiration,
        "payload": payload,
        "chain_id": chain_id
    });

    let signing_message_resp =
        aptos_post_json("/transactions/signing_message", raw_tx_req.clone()).await?;
    let signing_message_hex = signing_message_resp
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("Aptos signing_message missing message".into()))?;
    let signing_message_bytes = decode_hex_prefixed(signing_message_hex)?;
    let signature = sign_aptos_message(&signing_message_bytes).await?;

    let mut submit_req = raw_tx_req;
    submit_req["signature"] = json!({
        "type": "ed25519_signature",
        "public_key": format!("0x{}", addressing::hex_encode(&managed.pubkey)),
        "signature": format!("0x{}", addressing::hex_encode(&signature)),
    });

    let submit_resp = aptos_post_json("/transactions", submit_req).await?;
    let tx_hash = submit_resp
        .get("hash")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: tx_hash.clone(),
        message: tx_hash
            .map(|h| format!("Aptos submit accepted: {h}"))
            .unwrap_or_else(|| "Aptos submit accepted".to_string()),
    })
}

struct ManagedAptosIdentity {
    address: String,
    pubkey: [u8; 32],
}

async fn fetch_managed_aptos_identity() -> WalletResult<ManagedAptosIdentity> {
    let (pubkey, _key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for Aptos signer: {}",
            pubkey.len()
        )));
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&pubkey);
    let mut hasher = Sha3_256::new();
    hasher.update(pubkey);
    hasher.update([0u8]);
    let auth_key = hasher.finalize();
    Ok(ManagedAptosIdentity {
        address: format!("0x{}", addressing::hex_encode(&auth_key)),
        pubkey: pk,
    })
}

async fn fetch_coin_balance_raw(address: &str, coin_type: &str) -> WalletResult<BigUint> {
    let resource_type = format!("0x1::coin::CoinStore<{}>", normalize_type_tag(coin_type));
    match aptos_get_json(&format!(
        "/accounts/{}/resource/{}",
        path_encode(address),
        path_encode(&resource_type)
    ))
    .await
    {
        Ok(v) => {
            let amount = v
                .get("data")
                .and_then(|d| d.get("coin"))
                .and_then(|c| c.get("value"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    WalletError::Internal("Aptos CoinStore missing data.coin.value".into())
                })?;
            BigUint::parse_bytes(amount.as_bytes(), 10)
                .ok_or_else(|| WalletError::Internal("Aptos coin value parse failed".into()))
        }
        Err(WalletError::Internal(msg))
            if msg.contains("http status 404") || msg.contains("resource_not_found") =>
        {
            Ok(BigUint::from(0u8))
        }
        Err(e) => Err(e),
    }
}

async fn fetch_coin_decimals(coin_type: &str) -> WalletResult<u8> {
    let normalized = normalize_type_tag(coin_type);
    let (owner, _) = normalized
        .split_once("::")
        .ok_or_else(|| WalletError::invalid_input("invalid Aptos coin type"))?;
    let resource_type = format!("0x1::coin::CoinInfo<{normalized}>");
    let v = aptos_get_json(&format!(
        "/accounts/{}/resource/{}",
        path_encode(owner),
        path_encode(&resource_type)
    ))
    .await?;
    v.get("data")
        .and_then(|d| d.get("decimals"))
        .and_then(parse_u8_json)
        .ok_or_else(|| WalletError::Internal("Aptos CoinInfo missing decimals".into()))
}

async fn sign_aptos_message(message: &[u8]) -> WalletResult<Vec<u8>> {
    let key_name = config::app_config::default_schnorr_key_name().to_string();
    let args = SignWithSchnorrArgs {
        message: message.to_vec(),
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
            "unexpected Aptos signature length: {}",
            res.signature.len()
        )));
    }
    Ok(res.signature)
}

async fn aptos_get_json(path: &str) -> WalletResult<Value> {
    aptos_http_json(HttpMethod::GET, path, None).await
}

async fn aptos_post_json(path: &str, body: Value) -> WalletResult<Value> {
    aptos_http_json(HttpMethod::POST, path, Some(body)).await
}

async fn aptos_http_json(
    method: HttpMethod,
    path: &str,
    body: Option<Value>,
) -> WalletResult<Value> {
    let base = config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|e| WalletError::Internal(format!("aptos rpc url resolution failed: {e}")))?;
    let url = format!("{}{}", base.trim_end_matches('/'), path);
    let body_bytes = body
        .map(|v| {
            serde_json::to_vec(&v).map_err(|e| {
                WalletError::Internal(format!("serialize aptos http body failed: {e}"))
            })
        })
        .transpose()?;
    let res =
        crate::outcall::json_request(url, method, body_bytes, 1024 * 1024, "aptos rpc").await?;
    let parsed: Value = serde_json::from_slice(&res.body)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&res.body).to_string() }));
    if res.status != Nat::from(200u16) && res.status != Nat::from(201u16) {
        return Err(WalletError::Internal(format!(
            "aptos http status {}: {}",
            res.status, parsed
        )));
    }
    if parsed.get("error_code").is_some()
        || parsed.get("message").is_some()
            && parsed.get("hash").is_none()
            && parsed.get("sequence_number").is_none()
            && parsed.get("chain_id").is_none()
    {
        // keep real transaction success payloads; error payloads usually include error_code/message
        if parsed.get("error_code").is_some() {
            return Err(WalletError::Internal(format!("Aptos REST error: {parsed}")));
        }
    }
    Ok(parsed)
}

fn normalize_aptos_address(input: &str) -> WalletResult<String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(WalletError::invalid_input("Aptos address is required"));
    }
    let no_prefix = s.strip_prefix("0x").unwrap_or(s);
    if no_prefix.is_empty() || !no_prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(WalletError::invalid_input("invalid Aptos address hex"));
    }
    if no_prefix.len() > 64 {
        return Err(WalletError::invalid_input("Aptos address is too long"));
    }
    let mut padded = String::with_capacity(66);
    padded.push_str("0x");
    for _ in 0..(64 - no_prefix.len()) {
        padded.push('0');
    }
    padded.push_str(&no_prefix.to_lowercase());
    Ok(padded)
}

fn normalize_type_tag(input: &str) -> String {
    input.trim().to_string()
}

fn path_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(nibble((b >> 4) & 0x0f));
            out.push(nibble(b & 0x0f));
        }
    }
    out
}

fn decode_hex_prefixed(input: &str) -> WalletResult<Vec<u8>> {
    let s = input.trim().strip_prefix("0x").unwrap_or(input.trim());
    if !s.len().is_multiple_of(2) {
        return Err(WalletError::invalid_input("hex length must be even"));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> WalletResult<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(WalletError::invalid_input("invalid hex character")),
    }
}

fn nibble(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        10..=15 => (b'A' + (v - 10)) as char,
        _ => '0',
    }
}

fn parse_u8_json(v: &Value) -> Option<u8> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|x| u8::try_from(x).ok()),
        Value::String(s) => s.trim().parse::<u8>().ok(),
        _ => None,
    }
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
