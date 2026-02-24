use candid::Nat;
use ic_cdk::management_canister::{
    self, HttpHeader, HttpMethod, HttpRequestArgs, SchnorrAlgorithm, SchnorrKeyId,
    SignWithSchnorrArgs,
};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    self, AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest,
    TransferResponse,
};

const NETWORK_NAME: &str = types::networks::SOLANA;
const SOL_DECIMALS: u8 = 9;
const SOLANA_SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];
const SPL_TOKEN_PROGRAM_ID_BASE58: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Serialize)]
struct SolanaJsonRpcRequest {
    jsonrpc: &'static str,
    method: &'static str,
    params: Value,
    id: u32,
}

#[derive(Deserialize)]
struct SolanaJsonRpcResponse {
    result: Option<Value>,
    error: Option<SolanaJsonRpcError>,
}

#[derive(Deserialize)]
struct SolanaJsonRpcError {
    code: i64,
    message: String,
}

pub async fn request_address(req: AddressRequest) -> WalletResult<AddressResponse> {
    request_address_for_network(NETWORK_NAME, req).await
}

pub async fn request_address_for_network(
    network_name: &str,
    req: AddressRequest,
) -> WalletResult<AddressResponse> {
    let resolved = addressing::resolve_address_request(network_name, req)?;
    let (public_key, key_name) = addressing::fetch_schnorr_public_key(
        ic_cdk::management_canister::SchnorrAlgorithm::Ed25519,
    )
    .await?;

    if public_key.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for sol address: {}",
            public_key.len()
        )));
    }

    Ok(AddressResponse {
        network: network_name.to_string(),
        address: addressing::base58_encode(&public_key),
        public_key_hex: addressing::hex_encode(&public_key),
        key_name,
        index: resolved.index,
        account_tag: resolved.account_tag,
        message: Some("Derived from management canister Schnorr(ed25519) public key".into()),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    get_balance_for_network(NETWORK_NAME, req).await
}

pub async fn get_balance_for_network(
    network_name: &str,
    req: BalanceRequest,
) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    if req.token.as_deref().map(str::trim).is_some_and(|t| !t.is_empty()) {
        return Ok(BalanceResponse {
            network: network_name.to_string(),
            account: req.account,
            token: req.token,
            amount: None,
            decimals: Some(SOL_DECIMALS),
            block_ref: None,
            pending: true,
            message: Some("Solana SPL token balance query not implemented yet".to_string()),
        });
    }

    let rpc_result = solana_rpc_call(
        network_name,
        "getBalance",
        json!([req.account, { "commitment": "confirmed" }]),
    )
    .await?;
    let slot = rpc_result
        .get("context")
        .and_then(|v| v.get("slot"))
        .and_then(Value::as_u64);
    let lamports = rpc_result
        .get("value")
        .and_then(Value::as_u64)
        .ok_or_else(|| WalletError::Internal("solana rpc getBalance missing value".into()))?;

    Ok(BalanceResponse {
        network: network_name.to_string(),
        account: req.account,
        token: None,
        amount: Some(format_lamports(lamports)),
        decimals: Some(SOL_DECIMALS),
        block_ref: slot.map(|s| s.to_string()),
        pending: false,
        message: Some("RPC getBalance (formatted SOL)".to_string()),
    })
}

pub fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    transfer_for_network(NETWORK_NAME, req)
}

pub async fn transfer_sol(req: TransferRequest) -> WalletResult<TransferResponse> {
    transfer_sol_for_network(NETWORK_NAME, req).await
}

pub async fn transfer_spl(req: TransferRequest) -> WalletResult<TransferResponse> {
    transfer_spl_for_network(NETWORK_NAME, req).await
}

pub async fn transfer_sol_for_network(
    network_name: &str,
    req: TransferRequest,
) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    if req.token.as_deref().map(str::trim).is_some_and(|t| !t.is_empty()) {
        return Err(WalletError::invalid_input(
            "native SOL transfer does not accept token parameter",
        ));
    }

    let amount_lamports = parse_decimal_lamports(&req.amount)?;
    if amount_lamports == 0 {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }

    let (public_key, _key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if public_key.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for sol transfer: {}",
            public_key.len()
        )));
    }
    let from_pubkey: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::Internal("invalid ed25519 pubkey length".into()))?;
    let from_address = addressing::base58_encode(&from_pubkey);
    if let Some(from_override) = req.from.as_deref() {
        let normalized = from_override.trim();
        if !normalized.is_empty() && normalized != from_address {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed Solana address",
            ));
        }
    }

    let to_pubkey = decode_solana_pubkey(&req.to)?;
    let recent_blockhash = fetch_recent_blockhash(network_name).await?;
    let message = encode_system_transfer_message(&from_pubkey, &to_pubkey, &recent_blockhash, amount_lamports);
    let signature = sign_solana_message(&message).await?;
    if signature.len() != 64 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 signature length: {}",
            signature.len()
        )));
    }

    let raw_tx = encode_signed_transaction(&signature, &message);
    let tx_sig = send_raw_transaction(network_name, &raw_tx).await?;

    Ok(TransferResponse {
        network: network_name.to_string(),
        accepted: true,
        tx_id: Some(tx_sig.clone()),
        message: format!("broadcasted raw transaction via sendTransaction: {tx_sig}"),
    })
}

pub async fn transfer_spl_for_network(
    network_name: &str,
    req: TransferRequest,
) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    let mint_text = req
        .token
        .as_deref()
        .ok_or_else(|| WalletError::invalid_input("token (SPL mint) is required"))?;
    let mint = decode_solana_pubkey(mint_text)?;
    let destination_owner = decode_solana_pubkey(&req.to)?;
    let amount_decimals = fetch_spl_decimals(network_name, &mint).await?;
    let amount_raw = parse_decimal_u64_units(&req.amount, amount_decimals)?;
    if amount_raw == 0 {
        return Err(WalletError::invalid_input("amount must be > 0"));
    }

    let (public_key, _key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if public_key.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for spl transfer: {}",
            public_key.len()
        )));
    }
    let owner_pubkey: [u8; 32] = public_key
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::Internal("invalid ed25519 pubkey length".into()))?;
    let owner_address = addressing::base58_encode(&owner_pubkey);
    if let Some(from_override) = req.from.as_deref() {
        let normalized = from_override.trim();
        if !normalized.is_empty() && normalized != owner_address {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed Solana address",
            ));
        }
    }

    let source_token_account = fetch_token_account_for_owner(network_name, &owner_pubkey, &mint).await?;
    let dest_token_account =
        fetch_token_account_for_owner(network_name, &destination_owner, &mint).await?;
    let recent_blockhash = fetch_recent_blockhash(network_name).await?;
    let message = encode_spl_transfer_checked_message(
        &owner_pubkey,
        &source_token_account,
        &dest_token_account,
        &mint,
        &recent_blockhash,
        amount_raw,
        amount_decimals,
    )?;
    let signature = sign_solana_message(&message).await?;
    if signature.len() != 64 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 signature length: {}",
            signature.len()
        )));
    }
    let raw_tx = encode_signed_transaction(&signature, &message);
    let tx_sig = send_raw_transaction(network_name, &raw_tx).await?;

    Ok(TransferResponse {
        network: network_name.to_string(),
        accepted: true,
        tx_id: Some(tx_sig.clone()),
        message: format!("broadcasted SPL transfer via sendTransaction: {tx_sig}"),
    })
}

pub fn transfer_for_network(
    network_name: &str,
    req: TransferRequest,
) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    Ok(TransferResponse {
        network: network_name.to_string(),
        accepted: false,
        tx_id: None,
        message: format!(
            "{network_name} transfer scaffold received request; signing/execution not implemented"
        ),
    })
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

async fn fetch_recent_blockhash(network_name: &str) -> WalletResult<[u8; 32]> {
    let rpc_result = solana_rpc_call(
        network_name,
        "getLatestBlockhash",
        json!([{ "commitment": "confirmed" }]),
    )
    .await?;
    let blockhash = rpc_result
        .get("value")
        .and_then(|v| v.get("blockhash"))
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("solana rpc getLatestBlockhash missing blockhash".into()))?;
    decode_solana_pubkey(blockhash)
}

async fn sign_solana_message(message: &[u8]) -> WalletResult<Vec<u8>> {
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
    Ok(res.signature)
}

async fn send_raw_transaction(network_name: &str, raw_tx: &[u8]) -> WalletResult<String> {
    let raw_base64 = base64_encode(raw_tx);
    let tx_sig_value = solana_rpc_call(
        network_name,
        "sendTransaction",
        json!([
            raw_base64,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed"
            }
        ]),
    )
    .await?;
    let tx_sig = tx_sig_value
        .as_str()
        .ok_or_else(|| WalletError::Internal("solana rpc sendTransaction result is not string".into()))?;
    Ok(tx_sig.to_string())
}

async fn fetch_spl_decimals(network_name: &str, mint: &[u8; 32]) -> WalletResult<u8> {
    let mint_b58 = addressing::base58_encode(mint);
    let rpc_result = solana_rpc_call(
        network_name,
        "getTokenSupply",
        json!([mint_b58, { "commitment": "confirmed" }]),
    )
    .await?;
    let decimals = rpc_result
        .get("value")
        .and_then(|v| v.get("decimals"))
        .and_then(Value::as_u64)
        .ok_or_else(|| WalletError::Internal("solana rpc getTokenSupply missing decimals".into()))?;
    u8::try_from(decimals)
        .map_err(|_| WalletError::Internal("solana rpc token decimals out of range".into()))
}

async fn fetch_token_account_for_owner(
    network_name: &str,
    owner: &[u8; 32],
    mint: &[u8; 32],
) -> WalletResult<[u8; 32]> {
    let owner_b58 = addressing::base58_encode(owner);
    let mint_b58 = addressing::base58_encode(mint);
    let rpc_result = solana_rpc_call(
        network_name,
        "getTokenAccountsByOwner",
        json!([
            owner_b58,
            { "mint": mint_b58 },
            { "encoding": "jsonParsed", "commitment": "confirmed" }
        ]),
    )
    .await?;

    let first_pubkey = rpc_result
        .get("value")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("pubkey"))
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::invalid_input("destination/source token account not found for this mint"))?;
    decode_solana_pubkey(first_pubkey)
}

async fn solana_rpc_call(
    network_name: &str,
    method: &'static str,
    params: Value,
) -> WalletResult<Value> {
    let rpc_url = config::rpc_config::resolve_rpc_url(network_name, None)
        .map_err(|err| WalletError::Internal(format!("rpc url resolution failed: {err}")))?;

    let body = serde_json::to_vec(&SolanaJsonRpcRequest {
        jsonrpc: "2.0",
        method,
        params,
        id: 1,
    })
    .map_err(|err| WalletError::Internal(format!("serialize solana rpc request failed: {err}")))?;

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
        .map_err(|err| WalletError::Internal(format!("solana http outcall failed: {err}")))?;

    if http_res.status != Nat::from(200u16) {
        let body_text = String::from_utf8_lossy(&http_res.body);
        let snippet: String = body_text.chars().take(240).collect();
        return Err(WalletError::Internal(format!(
            "solana rpc http status {}: {}",
            http_res.status, snippet
        )));
    }

    let rpc_body: SolanaJsonRpcResponse = serde_json::from_slice(&http_res.body)
        .map_err(|err| WalletError::Internal(format!("parse solana rpc response failed: {err}")))?;
    if let Some(err) = rpc_body.error {
        return Err(WalletError::Internal(format!(
            "solana rpc error {}: {}",
            err.code, err.message
        )));
    }
    rpc_body
        .result
        .ok_or_else(|| WalletError::Internal("solana rpc response missing result".to_string()))
}

fn format_lamports(lamports: u64) -> String {
    let whole = lamports / 1_000_000_000;
    let frac = lamports % 1_000_000_000;
    if frac == 0 {
        return whole.to_string();
    }
    let mut frac_text = format!("{frac:09}");
    while frac_text.ends_with('0') {
        frac_text.pop();
    }
    format!("{whole}.{frac_text}")
}

fn decode_solana_pubkey(value: &str) -> WalletResult<[u8; 32]> {
    let bytes = base58_decode(value.trim())?;
    bytes.try_into().map_err(|_| {
        WalletError::invalid_input("solana pubkey/blockhash must decode to 32 bytes (base58)")
    })
}

fn base58_decode(input: &str) -> WalletResult<Vec<u8>> {
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
        let digit = base58_digit(ch).ok_or_else(|| WalletError::invalid_input("invalid base58 character"))?;
        acc = acc * 58u8 + BigUint::from(digit);
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

fn base58_digit(ch: u8) -> Option<u8> {
    BASE58_ALPHABET
        .iter()
        .position(|c| *c == ch)
        .map(|i| i as u8)
}

fn encode_system_transfer_message(
    from_pubkey: &[u8; 32],
    to_pubkey: &[u8; 32],
    recent_blockhash: &[u8; 32],
    lamports: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);

    // Legacy message header
    out.push(1); // num_required_signatures
    out.push(0); // num_readonly_signed_accounts
    out.push(1); // num_readonly_unsigned_accounts (system program)

    // account keys: payer, recipient, system program
    encode_shortvec_len(3, &mut out);
    out.extend_from_slice(from_pubkey);
    out.extend_from_slice(to_pubkey);
    out.extend_from_slice(&SOLANA_SYSTEM_PROGRAM_ID);

    // recent blockhash
    out.extend_from_slice(recent_blockhash);

    // instructions vec len = 1
    encode_shortvec_len(1, &mut out);

    // system_program::transfer instruction
    out.push(2); // program_id_index (system program)
    encode_shortvec_len(2, &mut out); // account indices len
    out.push(0); // from
    out.push(1); // to

    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes()); // SystemInstruction::Transfer
    data.extend_from_slice(&lamports.to_le_bytes());
    encode_shortvec_len(data.len(), &mut out);
    out.extend_from_slice(&data);

    out
}

fn encode_signed_transaction(signature: &[u8], message: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + signature.len() + message.len());
    encode_shortvec_len(1, &mut out);
    out.extend_from_slice(signature);
    out.extend_from_slice(message);
    out
}

fn encode_spl_transfer_checked_message(
    owner_pubkey: &[u8; 32],
    source_token_account: &[u8; 32],
    dest_token_account: &[u8; 32],
    mint: &[u8; 32],
    recent_blockhash: &[u8; 32],
    amount_raw: u64,
    decimals: u8,
) -> WalletResult<Vec<u8>> {
    let token_program_id = decode_solana_pubkey(SPL_TOKEN_PROGRAM_ID_BASE58)?;
    let mut out = Vec::with_capacity(320);

    // Legacy message header
    out.push(1); // num_required_signatures
    out.push(0); // num_readonly_signed_accounts
    out.push(2); // readonly unsigned: mint + token program

    // account keys: owner(payer/authority), source token acct, dest token acct, mint, token program
    encode_shortvec_len(5, &mut out);
    out.extend_from_slice(owner_pubkey);
    out.extend_from_slice(source_token_account);
    out.extend_from_slice(dest_token_account);
    out.extend_from_slice(mint);
    out.extend_from_slice(&token_program_id);

    out.extend_from_slice(recent_blockhash);

    encode_shortvec_len(1, &mut out); // instruction count
    out.push(4); // program_id_index = token program
    encode_shortvec_len(4, &mut out); // accounts len
    out.push(1); // source
    out.push(3); // mint
    out.push(2); // destination
    out.push(0); // authority

    let mut data = Vec::with_capacity(10);
    data.push(12); // TokenInstruction::TransferChecked
    data.extend_from_slice(&amount_raw.to_le_bytes());
    data.push(decimals);
    encode_shortvec_len(data.len(), &mut out);
    out.extend_from_slice(&data);
    Ok(out)
}

fn encode_shortvec_len(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let mut elem = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(elem);
            break;
        }
        elem |= 0x80;
        out.push(elem);
    }
}

fn parse_decimal_lamports(value: &str) -> WalletResult<u64> {
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
        return Err(WalletError::invalid_input("amount format is invalid"));
    }
    if whole.is_empty() && frac.is_none() {
        return Err(WalletError::invalid_input("amount format is invalid"));
    }
    if !whole.is_empty() && !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input("amount must be decimal"));
    }
    let whole_num = if whole.is_empty() {
        0u64
    } else {
        whole
            .parse::<u64>()
            .map_err(|_| WalletError::invalid_input("amount is too large"))?
    };
    let mut lamports = whole_num
        .checked_mul(1_000_000_000)
        .ok_or_else(|| WalletError::invalid_input("amount is too large"))?;

    if let Some(frac_part) = frac {
        if !frac_part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(WalletError::invalid_input("amount must be decimal"));
        }
        if frac_part.len() > SOL_DECIMALS as usize {
            return Err(WalletError::invalid_input(
                "too many decimal places for SOL (max 9)",
            ));
        }
        let mut frac_text = frac_part.to_string();
        while frac_text.len() < SOL_DECIMALS as usize {
            frac_text.push('0');
        }
        if !frac_text.is_empty() {
            let frac_num = frac_text
                .parse::<u64>()
                .map_err(|_| WalletError::invalid_input("amount is too large"))?;
            lamports = lamports
                .checked_add(frac_num)
                .ok_or_else(|| WalletError::invalid_input("amount is too large"))?;
        }
    }

    Ok(lamports)
}

fn parse_decimal_u64_units(value: &str, decimals: u8) -> WalletResult<u64> {
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
        return Err(WalletError::invalid_input("amount format is invalid"));
    }
    if !whole.is_empty() && !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(WalletError::invalid_input("amount must be decimal"));
    }
    let whole_num = if whole.is_empty() {
        0u64
    } else {
        whole
            .parse::<u64>()
            .map_err(|_| WalletError::invalid_input("amount is too large"))?
    };
    let scale = 10u64
        .checked_pow(u32::from(decimals))
        .ok_or_else(|| WalletError::invalid_input("decimals out of range"))?;
    let mut units = whole_num
        .checked_mul(scale)
        .ok_or_else(|| WalletError::invalid_input("amount is too large"))?;

    if let Some(frac_part) = frac {
        if !frac_part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(WalletError::invalid_input("amount must be decimal"));
        }
        if frac_part.len() > usize::from(decimals) {
            return Err(WalletError::invalid_input("too many decimal places"));
        }
        let mut frac_text = frac_part.to_string();
        while frac_text.len() < usize::from(decimals) {
            frac_text.push('0');
        }
        if !frac_text.is_empty() {
            let frac_num = frac_text
                .parse::<u64>()
                .map_err(|_| WalletError::invalid_input("amount is too large"))?;
            units = units
                .checked_add(frac_num)
                .ok_or_else(|| WalletError::invalid_input("amount is too large"))?;
        }
    }
    Ok(units)
}

fn base64_encode(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0usize;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(BASE64_ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = (data[i] as u32) << 16;
        out.push(BASE64_ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(BASE64_ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}
