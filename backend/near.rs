use candid::Nat;
use ic_cdk::management_canister::{self, SchnorrAlgorithm, SchnorrKeyId, SignWithSchnorrArgs};
use num_bigint::BigUint;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::addressing;
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::sdk::evm_tx;
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::NEAR_MAINNET;
const NEAR_DECIMALS: u8 = 24;
const NEAR_GAS_FT_TRANSFER: u64 = 50_000_000_000_000; // 50 Tgas
const NEAR_DEPOSIT_ONE_YOCTO: u128 = 1;

pub async fn request_address() -> WalletResult<AddressResponse> {
    let (pubkey, key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for NEAR address: {}",
            pubkey.len()
        )));
    }
    let implicit_account = hex_encode(&pubkey);
    let near_pubkey = format!("ed25519:{}", addressing::base58_encode(&pubkey));
    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: implicit_account,
        public_key_hex: addressing::hex_encode(&pubkey),
        key_name,
        message: Some(format!("NEAR implicit account (public key {near_pubkey})")),
    })
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    let account_id = req.account.trim().to_string();
    let token_opt = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if token_opt.is_none() {
        let result = match near_rpc_call(
            "query",
            json!({
                "request_type": "view_account",
                "finality": "final",
                "account_id": account_id
            }),
        )
        .await
        {
            Ok(v) => v,
            Err(err) if is_near_unknown_account_error(&err) => {
                return Ok(BalanceResponse {
                    network: NETWORK_NAME.to_string(),
                    account: req.account,
                    token: None,
                    amount: Some(evm_tx::format_units(
                        &BigUint::from(0u8),
                        usize::from(NEAR_DECIMALS),
                    )),
                    decimals: Some(NEAR_DECIMALS),
                    block_ref: None,
                    pending: false,
                    message: Some(
                        "NEAR implicit account not initialized on-chain yet; treating balance as 0"
                            .to_string(),
                    ),
                });
            }
            Err(err) => return Err(err),
        };
        let amount = result
            .get("amount")
            .and_then(Value::as_str)
            .ok_or_else(|| WalletError::Internal("NEAR view_account missing amount".into()))?;
        let yocto = BigUint::parse_bytes(amount.as_bytes(), 10)
            .ok_or_else(|| WalletError::Internal("NEAR amount parse failed".into()))?;
        return Ok(BalanceResponse {
            network: NETWORK_NAME.to_string(),
            account: req.account,
            token: None,
            amount: Some(evm_tx::format_units(&yocto, usize::from(NEAR_DECIMALS))),
            decimals: Some(NEAR_DECIMALS),
            block_ref: None,
            pending: false,
            message: Some("NEAR RPC query(view_account)".to_string()),
        });
    }

    let contract_id = token_opt.unwrap_or_default().to_string();
    let balance_bytes = near_call_function(
        &contract_id,
        "ft_balance_of",
        json!({ "account_id": account_id }),
    )
    .await?;
    let balance_text = String::from_utf8(balance_bytes)
        .map_err(|_| WalletError::Internal("NEAR ft_balance_of returned non-utf8 bytes".into()))?;
    let balance_value: Value = serde_json::from_str(balance_text.trim()).map_err(|err| {
        WalletError::Internal(format!("NEAR ft_balance_of json parse failed: {err}"))
    })?;
    let amount_raw = balance_value
        .as_str()
        .and_then(|s| BigUint::parse_bytes(s.trim().as_bytes(), 10))
        .unwrap_or_else(|| BigUint::from(0u8));
    let decimals = fetch_nep141_decimals(&contract_id).await.unwrap_or(24);
    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: Some(contract_id),
        amount: Some(evm_tx::format_units(&amount_raw, usize::from(decimals))),
        decimals: Some(decimals),
        block_ref: None,
        pending: false,
        message: Some("NEAR RPC query(call_function ft_balance_of)".to_string()),
    })
}

pub async fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    let managed = fetch_managed_near_identity().await?;
    if let Some(from) = req.from.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if from != managed.account_id {
            return Err(WalletError::invalid_input(
                "from does not match canister-managed NEAR account",
            ));
        }
    }

    let token_opt = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let access = near_rpc_call(
        "query",
        json!({
            "request_type": "view_access_key",
            "finality": "final",
            "account_id": managed.account_id,
            "public_key": managed.near_public_key
        }),
    )
    .await
    .map_err(|err| {
        if is_near_unknown_account_error(&err) {
            WalletError::invalid_input(
                "managed NEAR account is not initialized on-chain yet; fund the implicit account first",
            )
        } else {
            err
        }
    })?;
    let nonce = access
        .get("nonce")
        .and_then(Value::as_u64)
        .ok_or_else(|| WalletError::Internal("NEAR view_access_key missing nonce".into()))?
        .saturating_add(1);
    let block_hash_b58 = access
        .get("block_hash")
        .and_then(Value::as_str)
        .ok_or_else(|| WalletError::Internal("NEAR view_access_key missing block_hash".into()))?;
    let block_hash = base58_decode(block_hash_b58)?;
    if block_hash.len() != 32 {
        return Err(WalletError::Internal(
            "NEAR block_hash must decode to 32 bytes".into(),
        ));
    }
    let mut block_hash32 = [0u8; 32];
    block_hash32.copy_from_slice(&block_hash);

    let action = if token_opt.is_none() {
        let amount = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(NEAR_DECIMALS))?;
        if amount == BigUint::from(0u8) {
            return Err(WalletError::invalid_input("amount must be > 0"));
        }
        NearAction::Transfer {
            deposit: biguint_to_u128(&amount)?,
        }
    } else {
        let contract_id = token_opt.unwrap_or_default().to_string();
        let decimals = fetch_nep141_decimals(&contract_id).await.unwrap_or(24);
        let amount = evm_tx::parse_decimal_units(req.amount.trim(), usize::from(decimals))?;
        if amount == BigUint::from(0u8) {
            return Err(WalletError::invalid_input("amount must be > 0"));
        }
        let args = json!({
            "receiver_id": req.to.trim(),
            "amount": amount.to_string(),
            "memo": req.memo.as_deref().map(str::trim).filter(|s| !s.is_empty())
        });
        let args_bytes = serde_json::to_vec(&args).map_err(|err| {
            WalletError::Internal(format!("serialize ft_transfer args failed: {err}"))
        })?;
        NearAction::FunctionCall {
            method_name: "ft_transfer".to_string(),
            args: args_bytes,
            gas: NEAR_GAS_FT_TRANSFER,
            deposit: NEAR_DEPOSIT_ONE_YOCTO,
        }
    };

    let receiver_id = token_opt
        .map(ToString::to_string)
        .unwrap_or_else(|| req.to.trim().to_string());
    let tx = NearTransaction {
        signer_id: managed.account_id.clone(),
        public_key: managed.public_key32,
        nonce,
        receiver_id,
        block_hash: block_hash32,
        actions: vec![action],
    };
    let tx_bytes = tx.to_borsh()?;
    let tx_hash = Sha256::digest(&tx_bytes);
    let signature = sign_near_tx_hash(&tx_hash).await?;
    let signed_tx_bytes = NearSignedTransaction {
        transaction: tx,
        signature,
    }
    .to_borsh()?;
    let signed_b64 = base64_encode_std(&signed_tx_bytes);
    let res = near_rpc_call("broadcast_tx_commit", json!([signed_b64])).await?;
    let tx_id = res
        .get("transaction")
        .and_then(|t| t.get("hash"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            res.get("transaction_outcome")
                .and_then(|t| t.get("id"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        });

    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: true,
        tx_id: tx_id.clone(),
        message: tx_id
            .map(|h| format!("NEAR broadcast_tx_commit accepted: {h}"))
            .unwrap_or_else(|| "NEAR broadcast_tx_commit accepted".to_string()),
    })
}

struct NearManagedIdentity {
    account_id: String,
    near_public_key: String,
    public_key32: [u8; 32],
}

async fn fetch_managed_near_identity() -> WalletResult<NearManagedIdentity> {
    let (pubkey, _key_name) =
        addressing::fetch_schnorr_public_key(SchnorrAlgorithm::Ed25519).await?;
    if pubkey.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for NEAR signer: {}",
            pubkey.len()
        )));
    }
    let mut pk32 = [0u8; 32];
    pk32.copy_from_slice(&pubkey);
    Ok(NearManagedIdentity {
        account_id: hex_encode(&pubkey),
        near_public_key: format!("ed25519:{}", addressing::base58_encode(&pubkey)),
        public_key32: pk32,
    })
}

async fn fetch_nep141_decimals(contract_id: &str) -> WalletResult<u8> {
    let bytes = near_call_function(contract_id, "ft_metadata", json!({})).await?;
    let text = String::from_utf8(bytes)
        .map_err(|_| WalletError::Internal("NEAR ft_metadata returned non-utf8 bytes".into()))?;
    let v: Value = serde_json::from_str(text.trim()).map_err(|err| {
        WalletError::Internal(format!("NEAR ft_metadata json parse failed: {err}"))
    })?;
    v.get("decimals")
        .and_then(|x| {
            x.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .or_else(|| x.as_str()?.parse::<u8>().ok())
        })
        .ok_or_else(|| WalletError::Internal("NEAR ft_metadata missing decimals".into()))
}

async fn near_call_function(
    account_id: &str,
    method_name: &str,
    args_json: Value,
) -> WalletResult<Vec<u8>> {
    let args = serde_json::to_vec(&args_json)
        .map_err(|err| WalletError::Internal(format!("serialize NEAR call args failed: {err}")))?;
    let args_b64 = base64_encode_std(&args);
    let result = near_rpc_call(
        "query",
        json!({
            "request_type": "call_function",
            "finality": "final",
            "account_id": account_id,
            "method_name": method_name,
            "args_base64": args_b64
        }),
    )
    .await?;
    let arr = result
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| WalletError::Internal("NEAR call_function missing result bytes".into()))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let v = item.as_u64().ok_or_else(|| {
            WalletError::Internal("NEAR call_function result byte is not number".into())
        })?;
        let b = u8::try_from(v).map_err(|_| {
            WalletError::Internal("NEAR call_function result byte out of range".into())
        })?;
        out.push(b);
    }
    Ok(out)
}

async fn near_rpc_call(method: &str, params: Value) -> WalletResult<Value> {
    let base = config::rpc_config::resolve_rpc_url(NETWORK_NAME, None)
        .map_err(|e| WalletError::Internal(format!("near rpc url resolution failed: {e}")))?;
    let body = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": "near-wallet",
        "method": method,
        "params": params
    }))
    .map_err(|err| WalletError::Internal(format!("serialize near rpc body failed: {err}")))?;
    let http_res = crate::outcall::post_json(base, body, 1024 * 1024, "near rpc").await?;
    if http_res.status != Nat::from(200u16) {
        let snippet: String = String::from_utf8_lossy(&http_res.body)
            .chars()
            .take(300)
            .collect();
        return Err(WalletError::Internal(format!(
            "near rpc http status {}: {}",
            http_res.status, snippet
        )));
    }
    let payload: Value = serde_json::from_slice(&http_res.body)
        .map_err(|err| WalletError::Internal(format!("parse near rpc response failed: {err}")))?;
    if let Some(err) = payload.get("error") {
        return Err(WalletError::Internal(format!("NEAR RPC error: {err}")));
    }
    payload
        .get("result")
        .cloned()
        .ok_or_else(|| WalletError::Internal("NEAR RPC missing result".into()))
}

fn is_near_unknown_account_error(err: &WalletError) -> bool {
    let msg = match err {
        WalletError::Internal(s) => s.as_str(),
        WalletError::InvalidInput(s) => s.as_str(),
        _ => return false,
    };
    msg.contains("UNKNOWN_ACCOUNT") || msg.contains("does not exist while viewing")
}

async fn sign_near_tx_hash(hash32: &[u8]) -> WalletResult<[u8; 64]> {
    let key_name = config::app_config::default_schnorr_key_name().to_string();
    let args = SignWithSchnorrArgs {
        message: hash32.to_vec(),
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
            "unexpected NEAR signature length: {}",
            res.signature.len()
        )));
    }
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&res.signature);
    Ok(sig)
}

#[derive(Clone)]
struct NearTransaction {
    signer_id: String,
    public_key: [u8; 32],
    nonce: u64,
    receiver_id: String,
    block_hash: [u8; 32],
    actions: Vec<NearAction>,
}

struct NearSignedTransaction {
    transaction: NearTransaction,
    signature: [u8; 64],
}

#[derive(Clone)]
enum NearAction {
    Transfer {
        deposit: u128,
    },
    FunctionCall {
        method_name: String,
        args: Vec<u8>,
        gas: u64,
        deposit: u128,
    },
}

impl NearTransaction {
    fn to_borsh(&self) -> WalletResult<Vec<u8>> {
        let mut out = Vec::with_capacity(256);
        borsh_string(&mut out, &self.signer_id)?;
        near_public_key_borsh(&mut out, &self.public_key);
        out.extend_from_slice(&self.nonce.to_le_bytes());
        borsh_string(&mut out, &self.receiver_id)?;
        out.extend_from_slice(&self.block_hash);
        borsh_u32(&mut out, self.actions.len())?;
        for a in &self.actions {
            a.encode_borsh(&mut out)?;
        }
        Ok(out)
    }
}

impl NearSignedTransaction {
    fn to_borsh(&self) -> WalletResult<Vec<u8>> {
        let mut out = self.transaction.to_borsh()?;
        near_signature_borsh(&mut out, &self.signature);
        Ok(out)
    }
}

impl NearAction {
    fn encode_borsh(&self, out: &mut Vec<u8>) -> WalletResult<()> {
        match self {
            NearAction::Transfer { deposit } => {
                out.push(3); // Action::Transfer
                out.extend_from_slice(&deposit.to_le_bytes());
            }
            NearAction::FunctionCall {
                method_name,
                args,
                gas,
                deposit,
            } => {
                out.push(2); // Action::FunctionCall
                borsh_string(out, method_name)?;
                borsh_bytes(out, args)?;
                out.extend_from_slice(&gas.to_le_bytes());
                out.extend_from_slice(&deposit.to_le_bytes());
            }
        }
        Ok(())
    }
}

fn near_public_key_borsh(out: &mut Vec<u8>, pk32: &[u8; 32]) {
    out.push(0); // ED25519
    out.extend_from_slice(pk32);
}

fn near_signature_borsh(out: &mut Vec<u8>, sig64: &[u8; 64]) {
    out.push(0); // ED25519
    out.extend_from_slice(sig64);
}

fn borsh_string(out: &mut Vec<u8>, value: &str) -> WalletResult<()> {
    borsh_bytes(out, value.as_bytes())
}

fn borsh_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> WalletResult<()> {
    borsh_u32(out, bytes.len())?;
    out.extend_from_slice(bytes);
    Ok(())
}

fn borsh_u32(out: &mut Vec<u8>, value: usize) -> WalletResult<()> {
    let v = u32::try_from(value)
        .map_err(|_| WalletError::Internal("borsh length exceeds u32".into()))?;
    out.extend_from_slice(&v.to_le_bytes());
    Ok(())
}

fn base58_decode(input: &str) -> WalletResult<Vec<u8>> {
    if input.trim().is_empty() {
        return Err(WalletError::invalid_input("base58 string is required"));
    }
    let alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut zeros = 0usize;
    for b in input.trim().bytes() {
        if b == alphabet[0] {
            zeros += 1;
        } else {
            break;
        }
    }
    let mut acc = BigUint::from(0u8);
    for b in input.trim().bytes() {
        let pos = alphabet
            .iter()
            .position(|x| *x == b)
            .ok_or_else(|| WalletError::invalid_input("invalid base58 character"))?;
        acc = acc * 58u8 + BigUint::from(pos as u32);
    }
    let mut out = acc.to_bytes_be();
    if zeros > 0 {
        let mut prefixed = vec![0u8; zeros];
        prefixed.append(&mut out);
        Ok(prefixed)
    } else {
        Ok(out)
    }
}

fn base64_encode_std(data: &[u8]) -> String {
    // Reuse TON SDK helper (standard alphabet, no padding is accepted by NEAR).
    crate::sdk::ton_tx::base64_encode_std_nopad(data)
}

fn hex_encode(data: &[u8]) -> String {
    crate::sdk::ton_tx::hex_encode(data)
}

fn biguint_to_u128(v: &BigUint) -> WalletResult<u128> {
    let bytes = v.to_bytes_le();
    if bytes.len() > 16 {
        return Err(WalletError::invalid_input("amount is too large"));
    }
    let mut arr = [0u8; 16];
    arr[..bytes.len()].copy_from_slice(&bytes);
    Ok(u128::from_le_bytes(arr))
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
