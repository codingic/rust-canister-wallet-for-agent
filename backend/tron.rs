use candid::Nat;
use ic_cdk::management_canister::{self, EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgs};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::PublicKey;
use num_bigint::BigUint;
use serde_json::{json, Value};
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::Keccak256;

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::sdk::evm_tx;
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, ConfiguredTokenResponse,
    TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::TRON;
const TRX_DECIMALS: u8 = 6;
const TRON_PREFIX: u8 = 0x41;
const TRON_FEE_LIMIT_SUN_DEFAULT: u64 = 100_000_000;

#[derive(Clone, Debug)]
struct TronAddress {
    base58: String,
    evm20: [u8; 20],
}

pub async fn request_address() -> WalletResult<AddressResponse> {
    let (public_key, key_name) = addressing::fetch_ecdsa_secp256k1_public_key().await?;
    let addr = tron_address_from_sec1_public_key(&public_key)?;
    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: addr.base58,
        public_key_hex: addressing::hex_encode(&public_key),
        key_name,
        message: Some("Derived TRON address from management canister tECDSA public key".into()),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    let account = parse_tron_address(&req.account)?;
    let token_opt = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if token_opt.is_none() {
        let v = tron_post_json(
            "wallet/getaccount",
            json!({
                "address": account.base58,
                "visible": true
            }),
        )
        .await?;
        let balance_sun = v.get("balance").and_then(Value::as_u64).unwrap_or(0);
        return Ok(BalanceResponse {
            network: NETWORK_NAME.to_string(),
            account: account.base58,
            token: None,
            amount: Some(evm_tx::format_units(
                &BigUint::from(balance_sun),
                usize::from(TRX_DECIMALS),
            )),
            decimals: Some(TRX_DECIMALS),
            block_ref: None,
            pending: false,
            message: Some("TRON RPC wallet/getaccount".to_string()),
        });
    }

    let token = parse_tron_address(token_opt.unwrap_or_default())?;
    let decimals = fetch_trc20_decimals(&account, &token).await?;
    let amount_raw = fetch_trc20_balance_raw(&account, &token).await?;

    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: account.base58,
        token: Some(token.base58),
        amount: Some(evm_tx::format_units(&amount_raw, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some("TRON RPC triggerconstantcontract balanceOf(address)".to_string()),
    })
}

pub async fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    let to = parse_tron_address(&req.to)?;
    let token_opt = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let (pubkey, key_name) = addressing::fetch_ecdsa_secp256k1_public_key().await?;
    let managed = tron_address_from_sec1_public_key(&pubkey)?;
    if let Some(from) = req.from.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let from_addr = parse_tron_address(from)?;
        if from_addr.base58 != managed.base58 {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed TRON address",
            ));
        }
    }

    let mut tx = if token_opt.is_none() {
        let amount_sun = parse_u64_amount(&req.amount, TRX_DECIMALS)?;
        if amount_sun == 0 {
            return Err(WalletError::invalid_input("amount must be > 0"));
        }
        tron_create_trx_transfer(&managed, &to, amount_sun).await?
    } else {
        let token = parse_tron_address(token_opt.unwrap_or_default())?;
        let decimals = fetch_trc20_decimals(&managed, &token).await?;
        let amount_units = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
        if amount_units == BigUint::from(0u8) {
            return Err(WalletError::invalid_input("amount must be > 0"));
        }
        tron_create_trc20_transfer(&managed, &to, &token, &amount_units, req.memo.as_deref())
            .await?
    };

    let txid_hex = tx
        .get("txID")
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("TRON transaction response missing txID".into()))?
        .to_string();
    let txid_bytes = decode_hex(&txid_hex)?;
    if txid_bytes.len() != 32 {
        return Err(WalletError::Internal("TRON txID must be 32 bytes".into()));
    }
    let signature_hex = sign_tron_txid(&txid_bytes, &pubkey, &key_name).await?;
    tx["signature"] = Value::Array(vec![Value::String(signature_hex)]);

    let broadcast = tron_post_json("wallet/broadcasttransaction", tx).await?;
    let accepted = broadcast
        .get("result")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !accepted {
        let msg = broadcast
            .get("message")
            .and_then(Value::as_str)
            .map(|s| decode_hex_or_passthrough(s))
            .unwrap_or_else(|| broadcast.to_string());
        return Err(WalletError::Internal(format!(
            "TRON broadcasttransaction rejected: {msg}"
        )));
    }

    let txid = broadcast
        .get("txid")
        .and_then(Value::as_str)
        .unwrap_or(&txid_hex)
        .to_string();

    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: Some(txid.clone()),
        message: format!("TRON broadcasttransaction accepted: {txid}"),
    })
}

pub async fn discover_trc20_token(token_address: &str) -> WalletResult<ConfiguredTokenResponse> {
    let token = parse_tron_address(token_address)?;
    let (public_key, _key_name) = addressing::fetch_ecdsa_secp256k1_public_key().await?;
    let owner = tron_address_from_sec1_public_key(&public_key)?;
    let decimals = fetch_trc20_decimals(&owner, &token).await?;
    let symbol = fetch_trc20_string_property(&owner, &token, "symbol()")
        .await
        .unwrap_or_else(|_| format!("TRC{}", short_suffix(&token.base58)));
    let name = fetch_trc20_string_property(&owner, &token, "name()")
        .await
        .unwrap_or_else(|_| format!("TRC20 {}", short_suffix(&token.base58)));
    Ok(ConfiguredTokenResponse {
        network: NETWORK_NAME.to_string(),
        symbol,
        name,
        token_address: token.base58,
        decimals: u64::from(decimals),
    })
}

async fn tron_create_trx_transfer(
    from: &TronAddress,
    to: &TronAddress,
    amount_sun: u64,
) -> WalletResult<Value> {
    let tx = tron_post_json(
        "wallet/createtransaction",
        json!({
            "owner_address": from.base58,
            "to_address": to.base58,
            "amount": amount_sun,
            "visible": true
        }),
    )
    .await?;
    ensure_tron_tx_build_ok(&tx, "createtransaction")?;
    Ok(tx)
}

async fn tron_create_trc20_transfer(
    from: &TronAddress,
    to: &TronAddress,
    token: &TronAddress,
    amount_units: &BigUint,
    memo: Option<&str>,
) -> WalletResult<Value> {
    let encoded = evm_tx::encode_erc20_transfer_call(&to.evm20, amount_units)?;
    let parameter_hex = addressing::hex_encode(&encoded[4..]);
    let mut body = json!({
        "owner_address": from.base58,
        "contract_address": token.base58,
        "function_selector": "transfer(address,uint256)",
        "parameter": parameter_hex,
        "fee_limit": TRON_FEE_LIMIT_SUN_DEFAULT,
        "call_value": 0u64,
        "visible": true
    });
    if let Some(m) = memo.map(str::trim).filter(|s| !s.is_empty()) {
        body["data"] = Value::String(addressing::hex_encode(m.as_bytes()));
    }
    let resp = tron_post_json("wallet/triggersmartcontract", body).await?;
    let result_ok = resp
        .get("result")
        .and_then(|v| v.get("result"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !result_ok {
        let msg = resp
            .get("result")
            .and_then(|v| v.get("message"))
            .and_then(Value::as_str)
            .map(decode_hex_or_passthrough)
            .unwrap_or_else(|| resp.to_string());
        return Err(WalletError::Internal(format!(
            "TRON triggersmartcontract rejected: {msg}"
        )));
    }
    let tx = resp.get("transaction").cloned().ok_or_else(|| {
        WalletError::Internal("TRON triggersmartcontract missing transaction".into())
    })?;
    ensure_tron_tx_build_ok(&tx, "triggersmartcontract.transaction")?;
    Ok(tx)
}

fn ensure_tron_tx_build_ok(tx: &Value, label: &str) -> WalletResult<()> {
    if tx.get("raw_data_hex").and_then(Value::as_str).is_none() {
        return Err(WalletError::Internal(format!(
            "TRON {label} missing raw_data_hex"
        )));
    }
    if tx.get("txID").and_then(Value::as_str).is_none() {
        return Err(WalletError::Internal(format!("TRON {label} missing txID")));
    }
    Ok(())
}

async fn fetch_trc20_balance_raw(
    owner: &TronAddress,
    token: &TronAddress,
) -> WalletResult<BigUint> {
    let param_hex = tron_abi_encode_address_param(&owner.evm20);
    let resp = tron_post_json(
        "wallet/triggerconstantcontract",
        json!({
            "owner_address": owner.base58,
            "contract_address": token.base58,
            "function_selector": "balanceOf(address)",
            "parameter": param_hex,
            "visible": true
        }),
    )
    .await?;
    tron_constant_result_uint(&resp)
}

async fn fetch_trc20_decimals(owner: &TronAddress, token: &TronAddress) -> WalletResult<u8> {
    let resp = tron_post_json(
        "wallet/triggerconstantcontract",
        json!({
            "owner_address": owner.base58,
            "contract_address": token.base58,
            "function_selector": "decimals()",
            "visible": true
        }),
    )
    .await?;
    let n = tron_constant_result_uint(&resp)?;
    if n > BigUint::from(u8::MAX) {
        return Err(WalletError::Internal("TRC20 decimals out of range".into()));
    }
    Ok(*n.to_bytes_be().last().unwrap_or(&0))
}

async fn fetch_trc20_string_property(
    owner: &TronAddress,
    token: &TronAddress,
    selector: &str,
) -> WalletResult<String> {
    let resp = tron_post_json(
        "wallet/triggerconstantcontract",
        json!({
            "owner_address": owner.base58,
            "contract_address": token.base58,
            "function_selector": selector,
            "visible": true
        }),
    )
    .await?;
    let bytes = tron_constant_result_bytes(&resp)?;
    decode_abi_string_or_bytes32(&bytes)
}

fn tron_constant_result_uint(resp: &Value) -> WalletResult<BigUint> {
    if let Some(msg) = resp
        .get("result")
        .and_then(|v| v.get("message"))
        .and_then(Value::as_str)
    {
        let decoded = decode_hex_or_passthrough(msg);
        if !decoded.is_empty() {
            return Err(WalletError::Internal(format!(
                "TRON constant call error: {decoded}"
            )));
        }
    }
    let hex = resp
        .get("constant_result")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .ok_or_else(|| {
            WalletError::Internal("TRON constant call missing constant_result".into())
        })?;
    let bytes = decode_hex(hex)?;
    Ok(BigUint::from_bytes_be(&bytes))
}

fn tron_constant_result_bytes(resp: &Value) -> WalletResult<Vec<u8>> {
    if let Some(msg) = resp
        .get("result")
        .and_then(|v| v.get("message"))
        .and_then(Value::as_str)
    {
        let decoded = decode_hex_or_passthrough(msg);
        if !decoded.is_empty() {
            return Err(WalletError::Internal(format!(
                "TRON constant call error: {decoded}"
            )));
        }
    }
    let hex = resp
        .get("constant_result")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .ok_or_else(|| {
            WalletError::Internal("TRON constant call missing constant_result".into())
        })?;
    decode_hex(hex)
}

fn decode_abi_string_or_bytes32(bytes: &[u8]) -> WalletResult<String> {
    if bytes.is_empty() {
        return Err(WalletError::Internal(
            "TRC20 string property returned empty data".into(),
        ));
    }
    if bytes.len() == 32 {
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        return String::from_utf8(bytes[..end].to_vec())
            .map(|s| s.trim().to_string())
            .map_err(|_| WalletError::Internal("TRC20 bytes32 property is not utf8".into()));
    }
    if bytes.len() >= 96 {
        let offset = abi_u256_to_usize(&bytes[0..32])?;
        if offset + 64 > bytes.len() {
            return Err(WalletError::Internal(
                "TRC20 ABI string offset out of range".into(),
            ));
        }
        let len = abi_u256_to_usize(&bytes[offset..offset + 32])?;
        let start = offset + 32;
        let end = start.saturating_add(len);
        if end > bytes.len() {
            return Err(WalletError::Internal(
                "TRC20 ABI string length out of range".into(),
            ));
        }
        return String::from_utf8(bytes[start..end].to_vec())
            .map(|s| s.trim().to_string())
            .map_err(|_| WalletError::Internal("TRC20 string property is not utf8".into()));
    }
    Err(WalletError::Internal(
        "unsupported TRC20 string property ABI encoding".into(),
    ))
}

fn abi_u256_to_usize(bytes: &[u8]) -> WalletResult<usize> {
    if bytes.len() != 32 {
        return Err(WalletError::Internal("ABI word must be 32 bytes".into()));
    }
    if bytes[..24].iter().any(|b| *b != 0) {
        return Err(WalletError::Internal("ABI value too large".into()));
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[24..32]);
    usize::try_from(u64::from_be_bytes(arr))
        .map_err(|_| WalletError::Internal("ABI value too large".into()))
}

fn short_suffix(value: &str) -> String {
    value
        .get(value.len().saturating_sub(6)..)
        .unwrap_or(value)
        .to_uppercase()
}

fn tron_abi_encode_address_param(addr20: &[u8; 20]) -> String {
    let mut out = vec![0u8; 12];
    out.extend_from_slice(addr20);
    addressing::hex_encode(&out)
}

async fn sign_tron_txid(
    txid32: &[u8],
    public_key_sec1: &[u8],
    key_name: &str,
) -> WalletResult<String> {
    let txid_arr: [u8; 32] = txid32
        .try_into()
        .map_err(|_| WalletError::Internal("TRON txID must be 32 bytes".into()))?;
    let args = SignWithEcdsaArgs {
        message_hash: txid_arr.to_vec(),
        derivation_path: vec![],
        key_id: EcdsaKeyId {
            curve: EcdsaCurve::Secp256k1,
            name: key_name.to_string(),
        },
    };
    let res = management_canister::sign_with_ecdsa(&args)
        .await
        .map_err(|err| WalletError::Internal(format!("sign_with_ecdsa failed: {err}")))?;
    if res.signature.len() != 64 {
        return Err(WalletError::Internal(format!(
            "unexpected secp256k1 signature length: {}",
            res.signature.len()
        )));
    }

    let signature = Signature::try_from(res.signature.as_slice())
        .map_err(|err| WalletError::Internal(format!("invalid secp256k1 signature: {err}")))?;
    let expected_vk = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|err| WalletError::Internal(format!("invalid secp256k1 pubkey: {err}")))?;
    let recid = detect_recovery_id(&txid_arr, &signature, &expected_vk)?;

    let mut sig65 = res.signature;
    sig65.push(recid.to_byte());
    Ok(addressing::hex_encode(&sig65))
}

fn detect_recovery_id(
    prehash: &[u8; 32],
    signature: &Signature,
    expected_vk: &VerifyingKey,
) -> WalletResult<RecoveryId> {
    for b in 0u8..=3u8 {
        let Ok(recid) = RecoveryId::try_from(b) else {
            continue;
        };
        if let Ok(vk) = VerifyingKey::recover_from_prehash(prehash, signature, recid) {
            if &vk == expected_vk {
                return Ok(recid);
            }
        }
    }
    Err(WalletError::Internal(
        "failed to recover matching secp256k1 public key".into(),
    ))
}

fn tron_address_from_sec1_public_key(public_key: &[u8]) -> WalletResult<TronAddress> {
    let secp_pubkey = PublicKey::from_sec1_bytes(public_key)
        .map_err(|err| WalletError::Internal(format!("invalid secp256k1 public key: {err}")))?;
    let uncompressed = secp_pubkey.to_encoded_point(false);
    let bytes = uncompressed.as_bytes();
    if bytes.len() != 65 || bytes[0] != 0x04 {
        return Err(WalletError::Internal(
            "unexpected secp256k1 uncompressed public key length".into(),
        ));
    }
    let hash = Keccak256::digest(&bytes[1..]);
    let mut evm20 = [0u8; 20];
    evm20.copy_from_slice(&hash[12..]);
    let mut tron21 = [0u8; 21];
    tron21[0] = TRON_PREFIX;
    tron21[1..].copy_from_slice(&evm20);
    let base58 = tron_base58check_encode(&tron21);
    Ok(TronAddress { base58, evm20 })
}

fn parse_tron_address(value: &str) -> WalletResult<TronAddress> {
    let s = value.trim();
    if s.is_empty() {
        return Err(WalletError::invalid_input("TRON address is required"));
    }
    if s.starts_with('T') {
        let payload = tron_base58check_decode(s)?;
        return tron_address_from_payload(payload);
    }

    let hex = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    let bytes = decode_hex(hex)?;
    match bytes.len() {
        21 => {
            let mut payload = [0u8; 21];
            payload.copy_from_slice(&bytes);
            tron_address_from_payload(payload)
        }
        20 => {
            let mut payload = [0u8; 21];
            payload[0] = TRON_PREFIX;
            payload[1..].copy_from_slice(&bytes);
            tron_address_from_payload(payload)
        }
        _ => Err(WalletError::invalid_input(
            "TRON address must be base58check or 20/21-byte hex",
        )),
    }
}

fn tron_address_from_payload(payload: [u8; 21]) -> WalletResult<TronAddress> {
    if payload[0] != TRON_PREFIX {
        return Err(WalletError::invalid_input(
            "TRON address hex payload must start with 0x41",
        ));
    }
    let mut evm20 = [0u8; 20];
    evm20.copy_from_slice(&payload[1..]);
    Ok(TronAddress {
        base58: tron_base58check_encode(&payload),
        evm20,
    })
}

fn tron_base58check_encode(payload21: &[u8; 21]) -> String {
    let checksum = &double_sha256(payload21)[..4];
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(payload21);
    data.extend_from_slice(checksum);
    addressing::base58_encode(&data)
}

fn tron_base58check_decode(s: &str) -> WalletResult<[u8; 21]> {
    let raw = base58_decode(s)?;
    if raw.len() != 25 {
        return Err(WalletError::invalid_input(
            "TRON base58check address length invalid",
        ));
    }
    let mut payload = [0u8; 21];
    payload.copy_from_slice(&raw[..21]);
    let checksum = &raw[21..];
    let expected = &double_sha256(&payload)[..4];
    if checksum != expected {
        return Err(WalletError::invalid_input("TRON address checksum mismatch"));
    }
    Ok(payload)
}

fn base58_decode(input: &str) -> WalletResult<Vec<u8>> {
    const BASE58_ALPHABET: &[u8; 58] =
        b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if input.is_empty() {
        return Err(WalletError::invalid_input("base58 string is required"));
    }

    let mut zeros = 0usize;
    for ch in input.bytes() {
        if ch == BASE58_ALPHABET[0] {
            zeros += 1;
        } else {
            break;
        }
    }

    let mut acc = BigUint::from(0u8);
    for ch in input.bytes() {
        let digit = BASE58_ALPHABET
            .iter()
            .position(|c| *c == ch)
            .ok_or_else(|| WalletError::invalid_input("invalid base58 character"))?;
        acc = acc * 58u8 + BigUint::from(digit as u32);
    }

    let mut decoded = acc.to_bytes_be();
    if zeros > 0 {
        let mut prefixed = vec![0u8; zeros];
        prefixed.append(&mut decoded);
        Ok(prefixed)
    } else {
        Ok(decoded)
    }
}

async fn tron_post_json(path: &str, body: Value) -> WalletResult<Value> {
    let rpc_url = config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|err| WalletError::Internal(format!("trx rpc url resolution failed: {err}")))?;
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|err| WalletError::Internal(format!("serialize trx rpc request failed: {err}")))?;
    let http_res = crate::outcall::post_json(
        format!("{}/{}", rpc_url.trim_end_matches('/'), path),
        body_bytes,
        512 * 1024,
        "trx rpc",
    )
    .await?;
    if http_res.status != Nat::from(200u16) {
        let snippet = String::from_utf8_lossy(&http_res.body)
            .chars()
            .take(240)
            .collect::<String>();
        return Err(WalletError::Internal(format!(
            "trx rpc http status {}: {}",
            http_res.status, snippet
        )));
    }
    serde_json::from_slice::<Value>(&http_res.body)
        .map_err(|err| WalletError::Internal(format!("parse trx rpc response failed: {err}")))
}

fn parse_u64_amount(value: &str, decimals: u8) -> WalletResult<u64> {
    let n = evm_tx::parse_decimal_units(value.trim(), usize::from(decimals))?;
    let bytes = n.to_bytes_be();
    if bytes.len() > 8 {
        return Err(WalletError::invalid_input("amount is too large"));
    }
    let mut out = 0u64;
    for b in bytes {
        out = (out << 8) | u64::from(b);
    }
    Ok(out)
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

fn decode_hex(hex: &str) -> WalletResult<Vec<u8>> {
    let s = hex.trim();
    let digits = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    if digits.is_empty() {
        return Ok(Vec::new());
    }
    if digits.len() % 2 != 0 {
        return Err(WalletError::invalid_input("hex length must be even"));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let bytes = digits.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = from_hex_nibble(bytes[i])?;
        let lo = from_hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn from_hex_nibble(b: u8) -> WalletResult<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(WalletError::invalid_input("invalid hex character")),
    }
}

fn double_sha256(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

fn decode_hex_or_passthrough(value: &str) -> String {
    match decode_hex(value) {
        Ok(bytes) if !bytes.is_empty() => {
            String::from_utf8(bytes).unwrap_or_else(|_| value.to_string())
        }
        _ => value.to_string(),
    }
}
