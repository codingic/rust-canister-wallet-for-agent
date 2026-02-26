use candid::Nat;
use ic_cdk::management_canister::{self, EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgs};
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
use crate::sdk::evm_tx;
use crate::types::{
    BalanceRequest, BalanceResponse, BroadcastHttpRequest, ConfiguredTokenResponse,
    TransferRequest, TransferResponse,
};

const EVM_NATIVE_DECIMALS: usize = 18;
const EVM_NATIVE_GAS_LIMIT: u64 = 21_000;
const EVM_ERC20_GAS_LIMIT_DEFAULT: u64 = 120_000;

struct PreparedEvmBroadcast {
    tx_id: String,
    raw_tx_hex: String,
    broadcast_request: BroadcastHttpRequest,
}

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
    let wei = evm_tx::parse_hex_quantity(&result_hex)?;
    let eth_text = evm_tx::format_units(&wei, EVM_NATIVE_DECIMALS);

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
    let data = evm_tx::encode_erc20_balance_of_call(&account_bytes);
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
    let raw = evm_tx::parse_hex_data(&result_hex)?;
    let amount = BigUint::from_bytes_be(&raw);

    Ok(BalanceResponse {
        network: network.to_string(),
        account,
        token: Some(token_contract),
        amount: Some(evm_tx::format_units(&amount, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: Some("latest".to_string()),
        pending: false,
        message: Some("RPC eth_call balanceOf(address)".to_string()),
    })
}

pub async fn discover_erc20_token(
    network: &str,
    token_contract: &str,
) -> WalletResult<ConfiguredTokenResponse> {
    let token_contract = normalize_and_validate_hex_address(token_contract)?;
    let decimals = fetch_erc20_decimals(network, &token_contract).await?;
    let symbol = fetch_erc20_text_property(network, &token_contract, [0x95, 0xd8, 0x9b, 0x41])
        .await
        .unwrap_or_else(|_| fallback_token_symbol(&token_contract));
    let name = fetch_erc20_text_property(network, &token_contract, [0x06, 0xfd, 0xde, 0x03])
        .await
        .unwrap_or_else(|_| format!("ERC20 {}", short_address_suffix(&token_contract)));

    Ok(ConfiguredTokenResponse {
        network: network.to_string(),
        symbol,
        name,
        token_address: token_contract,
        decimals: u64::from(decimals),
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
    let value_wei = evm_tx::parse_decimal_units(req.amount.trim(), EVM_NATIVE_DECIMALS)?;
    if value_wei == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }
    let to_bytes = hex_address_to_20_bytes(&to)?;

    let prepared = prepare_eip1559_transaction(
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
        accepted: false,
        tx_id: Some(prepared.tx_id.clone()),
        signed_tx: Some(prepared.raw_tx_hex),
        signed_tx_encoding: Some("hex".to_string()),
        broadcast_request: Some(prepared.broadcast_request),
        message: format!(
            "signed EIP-1559 transaction prepared; frontend should broadcast via eth_sendRawTransaction: {}",
            prepared.tx_id
        ),
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
    let amount_units = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
    if amount_units == BigUint::from(0u8) {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }

    let data = evm_tx::encode_erc20_transfer_call(&to_bytes, &amount_units)?;
    let prepared = prepare_eip1559_transaction(
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
        accepted: false,
        tx_id: Some(prepared.tx_id.clone()),
        signed_tx: Some(prepared.raw_tx_hex),
        signed_tx_encoding: Some("hex".to_string()),
        broadcast_request: Some(prepared.broadcast_request),
        message: format!(
            "signed ERC20 transfer prepared; frontend should broadcast via eth_sendRawTransaction: {}",
            prepared.tx_id
        ),
    })
}

async fn prepare_eip1559_transaction(
    network: &str,
    from_override: Option<&str>,
    to_bytes: &[u8; 20],
    value: &BigUint,
    data: &[u8],
    gas_limit: &BigUint,
) -> WalletResult<PreparedEvmBroadcast> {
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

    let nonce = evm_tx::parse_hex_quantity(
        &rpc_call_hex_string(
            network,
            "eth_getTransactionCount",
            json!([from_address.clone(), "pending"]),
        )
        .await?,
    )?;

    let (max_priority_fee_per_gas, max_fee_per_gas) = fetch_eip1559_fees(network).await?;

    let signing_payload = evm_tx::rlp_encode_eip1559_unsigned(
        chain_id,
        &nonce,
        &max_priority_fee_per_gas,
        &max_fee_per_gas,
        gas_limit,
        to_bytes,
        value,
        data,
    );
    let signing_hash = evm_tx::keccak256(&signing_payload);
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

    let y_parity = if recovery.is_y_odd() { 1u8 } else { 0u8 };

    let r = BigUint::from_bytes_be(&signature_bytes[..32]);
    let s = BigUint::from_bytes_be(&signature_bytes[32..]);
    let signed_raw = evm_tx::rlp_encode_eip1559_signed(
        chain_id,
        &nonce,
        &max_priority_fee_per_gas,
        &max_fee_per_gas,
        gas_limit,
        to_bytes,
        value,
        data,
        y_parity,
        &r,
        &s,
    );
    let raw_tx_hex = format!("0x{}", addressing::hex_encode(&signed_raw));
    let tx_hash = format!(
        "0x{}",
        addressing::hex_encode(&evm_tx::keccak256(&signed_raw))
    );
    let rpc_url = config::rpc_config::resolve_rpc_url(network, None)
        .map_err(|err| WalletError::Internal(format!("rpc url resolution failed: {err}")))?;
    let broadcast_body = serde_json::to_string(&JsonRpcRequest {
        jsonrpc: "2.0",
        method: "eth_sendRawTransaction".to_string(),
        params: json!([raw_tx_hex.clone()]),
        id: 1,
    })
    .map_err(|err| WalletError::Internal(format!("serialize rpc request failed: {err}")))?;

    Ok(PreparedEvmBroadcast {
        tx_id: tx_hash,
        raw_tx_hex,
        broadcast_request: BroadcastHttpRequest {
            url: rpc_url,
            method: "POST".to_string(),
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("accept".to_string(), "application/json".to_string()),
            ],
            body: Some(broadcast_body),
        },
    })
}

async fn fetch_eip1559_fees(network: &str) -> WalletResult<(BigUint, BigUint)> {
    let priority_fee =
        match rpc_call_hex_string(network, "eth_maxPriorityFeePerGas", json!([])).await {
            Ok(v) => evm_tx::parse_hex_quantity(&v)?,
            Err(_) => {
                // Fallback for providers that do not support eth_maxPriorityFeePerGas.
                let gas_price = evm_tx::parse_hex_quantity(
                    &rpc_call_hex_string(network, "eth_gasPrice", json!([])).await?,
                )?;
                gas_price
            }
        };

    let base_fee = fetch_latest_base_fee_per_gas(network)
        .await
        .unwrap_or_else(|_| BigUint::from(0u8));
    let max_fee = (&base_fee * BigUint::from(2u8)) + &priority_fee;
    let max_fee = if max_fee < priority_fee {
        priority_fee.clone()
    } else {
        max_fee
    };
    Ok((priority_fee, max_fee))
}

async fn fetch_latest_base_fee_per_gas(network: &str) -> WalletResult<BigUint> {
    let latest_block = rpc_call(network, "eth_getBlockByNumber", json!(["latest", false])).await?;
    let base_fee_hex = latest_block
        .get("baseFeePerGas")
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("latest block missing baseFeePerGas".into()))?;
    evm_tx::parse_hex_quantity(base_fee_hex)
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

    let http_res = crate::outcall::post_json(rpc_url, body, 32 * 1024, "evm rpc").await?;

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

    let bytes = evm_tx::parse_hex_data(&result_hex)?;
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

async fn fetch_erc20_text_property(
    network: &str,
    token_contract: &str,
    selector: [u8; 4],
) -> WalletResult<String> {
    let result_hex = rpc_call_hex_string(
        network,
        "eth_call",
        json!([
            {
                "to": token_contract,
                "data": format!("0x{}", addressing::hex_encode(&selector))
            },
            "latest"
        ]),
    )
    .await?;
    let bytes = evm_tx::parse_hex_data(&result_hex)?;
    decode_abi_string_or_bytes32(&bytes)
}

fn decode_abi_string_or_bytes32(bytes: &[u8]) -> WalletResult<String> {
    if bytes.is_empty() {
        return Err(WalletError::Internal(
            "ERC20 string property returned empty data".into(),
        ));
    }
    if bytes.len() == 32 {
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        let s = String::from_utf8(bytes[..end].to_vec())
            .map_err(|_| WalletError::Internal("ERC20 bytes32 property is not utf8".into()))?;
        return Ok(s.trim().to_string());
    }
    if bytes.len() >= 96 {
        let offset = u256_be_to_usize(&bytes[0..32])?;
        if offset + 64 > bytes.len() {
            return Err(WalletError::Internal(
                "ERC20 ABI string offset is out of range".into(),
            ));
        }
        let len = u256_be_to_usize(&bytes[offset..offset + 32])?;
        let start = offset + 32;
        let end = start.saturating_add(len);
        if end > bytes.len() {
            return Err(WalletError::Internal(
                "ERC20 ABI string length is out of range".into(),
            ));
        }
        let s = String::from_utf8(bytes[start..end].to_vec())
            .map_err(|_| WalletError::Internal("ERC20 string property is not utf8".into()))?;
        return Ok(s.trim().to_string());
    }
    Err(WalletError::Internal(
        "unsupported ERC20 string property ABI encoding".into(),
    ))
}

fn u256_be_to_usize(bytes: &[u8]) -> WalletResult<usize> {
    if bytes.len() != 32 {
        return Err(WalletError::Internal(
            "u256 ABI word must be 32 bytes".into(),
        ));
    }
    if bytes[..24].iter().any(|b| *b != 0) {
        return Err(WalletError::Internal(
            "u256 value does not fit usize".into(),
        ));
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[24..32]);
    usize::try_from(u64::from_be_bytes(arr))
        .map_err(|_| WalletError::Internal("u256 value does not fit usize".into()))
}

fn short_address_suffix(addr: &str) -> String {
    let s = addr.trim();
    s.get(s.len().saturating_sub(6)..).unwrap_or(s).to_string()
}

fn fallback_token_symbol(addr: &str) -> String {
    format!("TKN{}", short_address_suffix(addr).to_uppercase())
}

#[cfg(test)]
mod tests {
    use crate::sdk::evm_tx;
    use num_bigint::BigUint;

    #[test]
    fn format_units_trims_trailing_zeros() {
        let v = BigUint::parse_bytes(b"1234500000000000000", 10).unwrap();
        assert_eq!(evm_tx::format_units(&v, 18), "1.2345");
    }

    #[test]
    fn format_units_handles_small_values() {
        let v = BigUint::parse_bytes(b"1000000000", 10).unwrap();
        assert_eq!(evm_tx::format_units(&v, 18), "0.000000001");
    }

    #[test]
    fn parse_decimal_units_works() {
        let v = evm_tx::parse_decimal_units("0.001", 18).unwrap();
        assert_eq!(v.to_str_radix(10), "1000000000000000");
    }

    #[test]
    fn rlp_encodes_zero_as_empty_string() {
        assert_eq!(evm_tx::rlp_encode_u64_for_test(0), vec![0x80]);
    }
}
