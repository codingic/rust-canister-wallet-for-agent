use candid::Nat;
use ic_cdk::management_canister::{
    self, EcdsaCurve, EcdsaKeyId, HttpHeader, HttpMethod, HttpRequestArgs, SignWithEcdsaArgs,
};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::PublicKey;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha3::{Digest, Keccak256};

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{BalanceRequest, BalanceResponse, TransferRequest, TransferResponse};

const EVM_NATIVE_DECIMALS: usize = 18;
const EVM_NATIVE_GAS_LIMIT: u64 = 21_000;
const EVM_ERC20_GAS_LIMIT_DEFAULT: u64 = 120_000;

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u32,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

pub async fn get_native_eth_balance(
    network: &str,
    req: BalanceRequest,
) -> WalletResult<BalanceResponse> {
    let account = normalize_and_validate_hex_address(&req.account)?;

    let result_hex =
        rpc_call_hex_string(network, "eth_getBalance", json!([account, "latest"])).await?;
    let wei = parse_hex_quantity(&result_hex)?;
    let eth_text = format_units(&wei, EVM_NATIVE_DECIMALS);

    Ok(BalanceResponse {
        network: network.to_string(),
        account,
        token: None,
        amount: Some(eth_text),
        decimals: Some(EVM_NATIVE_DECIMALS as u8),
        block_ref: Some("latest".to_string()),
        pending: false,
        message: Some("RPC eth_getBalance (formatted ETH)".to_string()),
    })
}

pub async fn get_erc20_balance(
    network: &str,
    req: BalanceRequest,
) -> WalletResult<BalanceResponse> {
    let account = normalize_and_validate_hex_address(&req.account)?;
    let token_contract = req
        .token
        .as_deref()
        .ok_or_else(|| WalletError::invalid_input("token is required for ERC20 balance query"))?;
    let token_contract = normalize_and_validate_hex_address(token_contract)?;
    let account_bytes = hex_address_to_20_bytes(&account)?;

    let decimals = fetch_erc20_decimals(network, &token_contract).await?;
    let data = encode_erc20_balance_of_call(&account_bytes);
    let result_hex = rpc_call_hex_string(
        network,
        "eth_call",
        json!([
            {
                "to": token_contract,
                "data": format!("0x{}", addressing::hex_encode(&data))
            },
            "latest"
        ]),
    )
    .await?;
    let raw = parse_hex_data(&result_hex)?;
    let amount = BigUint::from_bytes_be(&raw);

    Ok(BalanceResponse {
        network: network.to_string(),
        account,
        token: Some(token_contract),
        amount: Some(format_units(&amount, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: Some("latest".to_string()),
        pending: false,
        message: Some("RPC eth_call balanceOf(address)".to_string()),
    })
}

pub async fn transfer_native_eth(
    network: &str,
    req: TransferRequest,
) -> WalletResult<TransferResponse> {
    if req.token.is_some() {
        return Err(WalletError::invalid_input(
            "native ETH transfer does not accept token parameter",
        ));
    }
    let to = normalize_and_validate_hex_address(&req.to)?;
    let value_wei = parse_decimal_units(req.amount.trim(), EVM_NATIVE_DECIMALS)?;
    if value_wei == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }
    let to_bytes = hex_address_to_20_bytes(&to)?;

    let tx_id = send_legacy_transaction(
        network,
        req.from.as_deref(),
        &to_bytes,
        &value_wei,
        &[],
        &BigUint::from(EVM_NATIVE_GAS_LIMIT),
    )
    .await?;

    Ok(TransferResponse {
        network: network.to_string(),
        accepted: true,
        tx_id: Some(tx_id.clone()),
        message: format!("broadcasted raw transaction via eth_sendRawTransaction: {tx_id}"),
    })
}

pub async fn transfer_erc20(network: &str, req: TransferRequest) -> WalletResult<TransferResponse> {
    let token_contract = req
        .token
        .as_deref()
        .ok_or_else(|| WalletError::invalid_input("token is required for ERC20 transfer"))?;
    let token_contract = normalize_and_validate_hex_address(token_contract)?;
    let token_contract_bytes = hex_address_to_20_bytes(&token_contract)?;

    let to = normalize_and_validate_hex_address(&req.to)?;
    let to_bytes = hex_address_to_20_bytes(&to)?;

    let decimals = fetch_erc20_decimals(network, &token_contract).await?;
    let amount_units = parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
    if amount_units == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }

    let data = encode_erc20_transfer_call(&to_bytes, &amount_units)?;
    let tx_id = send_legacy_transaction(
        network,
        req.from.as_deref(),
        &token_contract_bytes,
        &BigUint::from(0u8),
        &data,
        &BigUint::from(EVM_ERC20_GAS_LIMIT_DEFAULT),
    )
    .await?;

    Ok(TransferResponse {
        network: network.to_string(),
        accepted: true,
        tx_id: Some(tx_id.clone()),
        message: format!("broadcasted ERC20 transfer via eth_sendRawTransaction: {tx_id}"),
    })
}

async fn send_legacy_transaction(
    network: &str,
    from_override: Option<&str>,
    to_bytes: &[u8; 20],
    value: &BigUint,
    data: &[u8],
    gas_limit: &BigUint,
) -> WalletResult<String> {
    let (public_key_bytes, _key_name) = addressing::fetch_ecdsa_secp256k1_public_key().await?;
    let from_address = evm_address_from_sec1_public_key(&public_key_bytes)?;
    if let Some(from) = from_override {
        let normalized_from = normalize_and_validate_hex_address(from)?;
        if !eq_hex_address(&normalized_from, &from_address) {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed EVM address",
            ));
        }
    }

    let chain_id = config::rpc_config::chain_id(network).ok_or_else(|| {
        WalletError::Internal(format!("missing chain_id config for network: {network}"))
    })?;

    let nonce = parse_hex_quantity(
        &rpc_call_hex_string(
            network,
            "eth_getTransactionCount",
            json!([from_address.clone(), "pending"]),
        )
        .await?,
    )?;

    let gas_price =
        parse_hex_quantity(&rpc_call_hex_string(network, "eth_gasPrice", json!([])).await?)?;

    let signing_payload = rlp_encode_legacy_unsigned(
        &nonce, &gas_price, gas_limit, &to_bytes, value, data, chain_id,
    );
    let signing_hash = keccak256(&signing_payload);
    let signature_bytes = sign_prehash_with_management(&signing_hash).await?;

    let signature = Signature::try_from(signature_bytes.as_slice()).map_err(|err| {
        WalletError::Internal(format!("invalid secp256k1 signature from tECDSA: {err}"))
    })?;
    let expected_vk = VerifyingKey::from_sec1_bytes(&public_key_bytes)
        .map_err(|err| WalletError::Internal(format!("invalid secp256k1 public key: {err}")))?;
    let recovery = detect_recovery_id(&signing_hash, &signature, &expected_vk)?;
    if recovery.is_x_reduced() {
        return Err(WalletError::Internal(
            "unsupported ECDSA recovery id (x_reduced=true) for Ethereum encoding".into(),
        ));
    }

    let y_parity = if recovery.is_y_odd() { 1u64 } else { 0u64 };
    let v = chain_id
        .checked_mul(2)
        .and_then(|x| x.checked_add(35 + y_parity))
        .ok_or_else(|| WalletError::Internal("v overflow".into()))?;

    let r = BigUint::from_bytes_be(&signature_bytes[..32]);
    let s = BigUint::from_bytes_be(&signature_bytes[32..]);
    let signed_raw = rlp_encode_legacy_signed(
        &nonce,
        &gas_price,
        gas_limit,
        &to_bytes,
        value,
        data,
        &BigUint::from(v),
        &r,
        &s,
    );
    let raw_tx_hex = format!("0x{}", addressing::hex_encode(&signed_raw));

    rpc_call_hex_string(network, "eth_sendRawTransaction", json!([raw_tx_hex])).await
}

async fn sign_prehash_with_management(prehash: &[u8; 32]) -> WalletResult<Vec<u8>> {
    let key_name = config::app_config::default_ecdsa_key_name().to_string();
    let args = SignWithEcdsaArgs {
        message_hash: prehash.to_vec(),
        derivation_path: vec![],
        key_id: EcdsaKeyId {
            curve: EcdsaCurve::Secp256k1,
            name: key_name,
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
    Ok(res.signature)
}

fn detect_recovery_id(
    prehash: &[u8; 32],
    signature: &Signature,
    expected_vk: &VerifyingKey,
) -> WalletResult<RecoveryId> {
    for byte in 0u8..=3u8 {
        let Ok(recid) = RecoveryId::try_from(byte) else {
            continue;
        };
        if let Ok(vk) = VerifyingKey::recover_from_prehash(prehash, signature, recid) {
            if &vk == expected_vk {
                return Ok(recid);
            }
        }
    }
    Err(WalletError::Internal(
        "failed to recover matching secp256k1 public key for signature".into(),
    ))
}

async fn rpc_call_hex_string(network: &str, method: &str, params: Value) -> WalletResult<String> {
    let value = rpc_call(network, method, params).await?;
    let s = value
        .as_str()
        .ok_or_else(|| WalletError::Internal(format!("rpc {method} result is not string")))?;
    Ok(s.to_string())
}

async fn rpc_call(network: &str, method: &str, params: Value) -> WalletResult<Value> {
    let rpc_url = config::rpc_config::resolve_rpc_url(network, None)
        .map_err(|err| WalletError::Internal(format!("rpc url resolution failed: {err}")))?;

    let body = serde_json::to_vec(&JsonRpcRequest {
        jsonrpc: "2.0",
        method: method.to_string(),
        params,
        id: 1,
    })
    .map_err(|err| {
        WalletError::Internal(format!("serialize rpc request failed ({method}): {err}"))
    })?;

    let http_args = HttpRequestArgs {
        url: rpc_url,
        max_response_bytes: Some(32 * 1024),
        method: HttpMethod::POST,
        headers: vec![
            HttpHeader {
                name: "content-type".to_string(),
                value: "application/json".to_string(),
            },
            HttpHeader {
                name: "accept".to_string(),
                value: "application/json".to_string(),
            },
        ],
        body: Some(body),
        transform: None,
    };

    let http_res = management_canister::http_request(&http_args)
        .await
        .map_err(|err| WalletError::Internal(format!("http outcall failed: {err}")))?;

    if http_res.status != Nat::from(200u16) {
        let body_text = String::from_utf8_lossy(&http_res.body);
        let snippet: String = body_text.chars().take(240).collect();
        return Err(WalletError::Internal(format!(
            "rpc http status {}: {}",
            http_res.status, snippet
        )));
    }

    let rpc_body: JsonRpcResponse = serde_json::from_slice(&http_res.body)
        .map_err(|err| WalletError::Internal(format!("parse rpc response failed: {err}")))?;

    if let Some(err) = rpc_body.error {
        return Err(WalletError::Internal(format!(
            "rpc error {}: {}",
            err.code, err.message
        )));
    }

    rpc_body
        .result
        .ok_or_else(|| WalletError::Internal("rpc response missing result".to_string()))
}

fn normalize_and_validate_hex_address(value: &str) -> WalletResult<String> {
    let s = value.trim();
    if !is_hex_address(s) {
        return Err(WalletError::invalid_input(
            "EVM account must be a 0x-prefixed 20-byte hex address",
        ));
    }
    Ok(format!("0x{}", &s[2..].to_lowercase()))
}

fn eq_hex_address(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn is_hex_address(value: &str) -> bool {
    let s = value.trim();
    let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"));
    let Some(hex) = hex else {
        return false;
    };
    hex.len() == 40 && hex.as_bytes().iter().all(|b| b.is_ascii_hexdigit())
}

fn hex_address_to_20_bytes(value: &str) -> WalletResult<[u8; 20]> {
    let normalized = normalize_and_validate_hex_address(value)?;
    let hex = &normalized[2..];
    let mut out = [0u8; 20];
    for i in 0..20 {
        let hi = from_hex_nibble(hex.as_bytes()[i * 2])?;
        let lo = from_hex_nibble(hex.as_bytes()[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
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

fn evm_address_from_sec1_public_key(public_key: &[u8]) -> WalletResult<String> {
    let secp_pubkey = PublicKey::from_sec1_bytes(public_key)
        .map_err(|err| WalletError::Internal(format!("invalid secp256k1 public key: {err}")))?;
    let uncompressed = secp_pubkey.to_encoded_point(false);
    let uncompressed_bytes = uncompressed.as_bytes();
    if uncompressed_bytes.len() != 65 || uncompressed_bytes[0] != 0x04 {
        return Err(WalletError::Internal(
            "unexpected secp256k1 uncompressed public key length".into(),
        ));
    }
    let hash = Keccak256::digest(&uncompressed_bytes[1..]);
    Ok(format!("0x{}", addressing::hex_encode(&hash[12..])))
}

async fn fetch_erc20_decimals(network: &str, token_contract: &str) -> WalletResult<u8> {
    let result_hex = rpc_call_hex_string(
        network,
        "eth_call",
        json!([
            {
                "to": token_contract,
                "data": "0x313ce567"
            },
            "latest"
        ]),
    )
    .await?;

    let bytes = parse_hex_data(&result_hex)?;
    if bytes.is_empty() {
        return Err(WalletError::Internal(
            "ERC20 decimals() returned empty data".into(),
        ));
    }
    let value = BigUint::from_bytes_be(&bytes);
    if value > BigUint::from(u8::MAX) {
        return Err(WalletError::Internal(
            "ERC20 decimals() value out of range".into(),
        ));
    }
    let raw = value.to_bytes_be();
    Ok(*raw.last().unwrap_or(&0))
}

fn encode_erc20_transfer_call(to: &[u8; 20], amount: &BigUint) -> WalletResult<Vec<u8>> {
    let amount_bytes = amount.to_bytes_be();
    if amount_bytes.len() > 32 {
        return Err(WalletError::invalid_input("token amount is too large"));
    }

    let mut out = Vec::with_capacity(4 + 32 + 32);
    out.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]); // transfer(address,uint256)
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(to);
    out.extend_from_slice(&vec![0u8; 32 - amount_bytes.len()]);
    out.extend_from_slice(&amount_bytes);
    Ok(out)
}

fn encode_erc20_balance_of_call(account: &[u8; 20]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 32);
    out.extend_from_slice(&[0x70, 0xa0, 0x82, 0x31]); // balanceOf(address)
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(account);
    out
}

fn parse_hex_quantity(hex: &str) -> WalletResult<BigUint> {
    let trimmed = hex.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .ok_or_else(|| WalletError::Internal("rpc result is not a hex quantity".to_string()))?;
    if digits.is_empty() {
        return Err(WalletError::Internal(
            "rpc result hex quantity is empty".to_string(),
        ));
    }
    BigUint::parse_bytes(digits.as_bytes(), 16)
        .ok_or_else(|| WalletError::Internal("rpc result hex quantity parse failed".to_string()))
}

fn parse_hex_data(hex: &str) -> WalletResult<Vec<u8>> {
    let trimmed = hex.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .ok_or_else(|| WalletError::Internal("rpc result is not hex data".to_string()))?;
    if digits.is_empty() {
        return Ok(Vec::new());
    }
    if digits.len() % 2 != 0 {
        return Err(WalletError::Internal(
            "rpc hex data length is odd".to_string(),
        ));
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

fn parse_decimal_units(value: &str, decimals: usize) -> WalletResult<BigUint> {
    let v = value.trim();
    if v.is_empty() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    if v.starts_with('-') {
        return Err(WalletError::invalid_input("amount must be positive"));
    }
    let mut parts = v.split('.');
    let whole = parts.next().unwrap_or_default();
    let frac = parts.next();
    if parts.next().is_some() {
        return Err(WalletError::invalid_input("invalid decimal amount format"));
    }
    if whole.is_empty() && frac.is_none() {
        return Err(WalletError::invalid_input("amount is required"));
    }
    if !whole.is_empty() && !whole.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input(
            "amount has non-digit characters",
        ));
    }
    let frac = frac.unwrap_or("");
    if !frac.is_empty() && !frac.as_bytes().iter().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input(
            "amount has non-digit characters",
        ));
    }
    if frac.len() > decimals {
        return Err(WalletError::invalid_input(format!(
            "amount supports at most {decimals} decimal places"
        )));
    }

    let whole_num = if whole.is_empty() {
        BigUint::from(0u8)
    } else {
        BigUint::parse_bytes(whole.as_bytes(), 10)
            .ok_or_else(|| WalletError::invalid_input("invalid whole amount"))?
    };
    let scale = BigUint::from(10u8).pow(decimals as u32);
    let mut out = &whole_num * &scale;

    if !frac.is_empty() {
        let mut frac_padded = frac.to_string();
        frac_padded.push_str(&"0".repeat(decimals - frac.len()));
        let frac_num = BigUint::parse_bytes(frac_padded.as_bytes(), 10)
            .ok_or_else(|| WalletError::invalid_input("invalid fractional amount"))?;
        out += frac_num;
    }
    Ok(out)
}

fn format_units(value: &BigUint, decimals: usize) -> String {
    if decimals == 0 {
        return value.to_str_radix(10);
    }

    let raw = value.to_str_radix(10);
    if raw == "0" {
        return "0".to_string();
    }

    if raw.len() <= decimals {
        let mut out = String::from("0.");
        out.push_str(&"0".repeat(decimals - raw.len()));
        out.push_str(&raw);
        trim_decimal_zeros(out)
    } else {
        let split = raw.len() - decimals;
        let mut out = String::with_capacity(raw.len() + 1);
        out.push_str(&raw[..split]);
        out.push('.');
        out.push_str(&raw[split..]);
        trim_decimal_zeros(out)
    }
}

fn trim_decimal_zeros(mut s: String) -> String {
    if let Some(dot) = s.find('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.len() == dot + 1 {
            s.pop();
        }
    }
    s
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let digest = Keccak256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn rlp_encode_legacy_unsigned(
    nonce: &BigUint,
    gas_price: &BigUint,
    gas_limit: &BigUint,
    to: &[u8; 20],
    value: &BigUint,
    data: &[u8],
    chain_id: u64,
) -> Vec<u8> {
    rlp_encode_list(&[
        rlp_encode_biguint(nonce),
        rlp_encode_biguint(gas_price),
        rlp_encode_biguint(gas_limit),
        rlp_encode_bytes(to),
        rlp_encode_biguint(value),
        rlp_encode_bytes(data),
        rlp_encode_u64(chain_id),
        rlp_encode_u64(0),
        rlp_encode_u64(0),
    ])
}

fn rlp_encode_legacy_signed(
    nonce: &BigUint,
    gas_price: &BigUint,
    gas_limit: &BigUint,
    to: &[u8; 20],
    value: &BigUint,
    data: &[u8],
    v: &BigUint,
    r: &BigUint,
    s: &BigUint,
) -> Vec<u8> {
    rlp_encode_list(&[
        rlp_encode_biguint(nonce),
        rlp_encode_biguint(gas_price),
        rlp_encode_biguint(gas_limit),
        rlp_encode_bytes(to),
        rlp_encode_biguint(value),
        rlp_encode_bytes(data),
        rlp_encode_biguint(v),
        rlp_encode_biguint(r),
        rlp_encode_biguint(s),
    ])
}

fn rlp_encode_u64(v: u64) -> Vec<u8> {
    rlp_encode_biguint(&BigUint::from(v))
}

fn rlp_encode_biguint(v: &BigUint) -> Vec<u8> {
    if *v == BigUint::from(0u8) {
        return rlp_encode_bytes(&[]);
    }
    rlp_encode_bytes(&v.to_bytes_be())
}

fn rlp_encode_bytes(data: &[u8]) -> Vec<u8> {
    match data.len() {
        0 => vec![0x80],
        1 if data[0] < 0x80 => vec![data[0]],
        len if len <= 55 => {
            let mut out = Vec::with_capacity(1 + len);
            out.push(0x80 + len as u8);
            out.extend_from_slice(data);
            out
        }
        len => {
            let len_bytes = usize_to_be_bytes(len);
            let mut out = Vec::with_capacity(1 + len_bytes.len() + len);
            out.push(0xb7 + len_bytes.len() as u8);
            out.extend_from_slice(&len_bytes);
            out.extend_from_slice(data);
            out
        }
    }
}

fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload_len: usize = items.iter().map(Vec::len).sum();
    let mut payload = Vec::with_capacity(payload_len);
    for item in items {
        payload.extend_from_slice(item);
    }

    if payload_len <= 55 {
        let mut out = Vec::with_capacity(1 + payload_len);
        out.push(0xc0 + payload_len as u8);
        out.extend_from_slice(&payload);
        out
    } else {
        let len_bytes = usize_to_be_bytes(payload_len);
        let mut out = Vec::with_capacity(1 + len_bytes.len() + payload_len);
        out.push(0xf7 + len_bytes.len() as u8);
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(&payload);
        out
    }
}

fn usize_to_be_bytes(value: usize) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first_non_zero = bytes
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(bytes.len() - 1);
    bytes[first_non_zero..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_units_trims_trailing_zeros() {
        let v = BigUint::parse_bytes(b"1234500000000000000", 10).unwrap();
        assert_eq!(format_units(&v, 18), "1.2345");
    }

    #[test]
    fn format_units_handles_small_values() {
        let v = BigUint::parse_bytes(b"1000000000", 10).unwrap();
        assert_eq!(format_units(&v, 18), "0.000000001");
    }

    #[test]
    fn parse_decimal_units_works() {
        let v = parse_decimal_units("0.001", 18).unwrap();
        assert_eq!(v.to_str_radix(10), "1000000000000000");
    }

    #[test]
    fn rlp_encodes_zero_as_empty_string() {
        assert_eq!(rlp_encode_u64(0), vec![0x80]);
    }
}
