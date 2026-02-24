use crate::addressing;
use crate::error::{WalletError, WalletResult};
use crate::evm_rpc;
use crate::types::{
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest,
    TransferResponse,
};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::PublicKey;
use sha3::{Digest, Keccak256};

const NETWORK_NAME: &str = "sepolia";

pub async fn request_address(req: AddressRequest) -> WalletResult<AddressResponse> {
    let resolved = addressing::resolve_address_request(NETWORK_NAME, req)?;
    let (public_key, key_name) = addressing::fetch_ecdsa_secp256k1_public_key().await?;

    let secp_pubkey = PublicKey::from_sec1_bytes(&public_key)
        .map_err(|err| WalletError::Internal(format!("invalid sepolia public key: {err}")))?;
    let uncompressed = secp_pubkey.to_encoded_point(false);
    let uncompressed_bytes = uncompressed.as_bytes();
    if uncompressed_bytes.len() != 65 || uncompressed_bytes[0] != 0x04 {
        return Err(WalletError::Internal(
            "unexpected secp256k1 uncompressed public key length".into(),
        ));
    }

    let hash = Keccak256::digest(&uncompressed_bytes[1..]);
    let addr_hex = addressing::hex_encode(&hash[12..]);

    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: format!("0x{addr_hex}"),
        public_key_hex: addressing::hex_encode(&public_key),
        key_name,
        index: resolved.index,
        account_tag: resolved.account_tag,
        message: Some("Derived from management canister ECDSA public key".into()),
    })
}

pub fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: req.token,
        amount: None,
        decimals: Some(18),
        block_ref: None,
        pending: true,
        message: Some("Sepolia balance query not implemented yet".to_string()),
    })
}

pub async fn get_balance_eth(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    if req.token.is_some() {
        return Err(WalletError::invalid_input(
            "sepolia_get_balance_eth does not accept token parameter",
        ));
    }
    evm_rpc::get_native_eth_balance(NETWORK_NAME, req).await
}

pub fn transfer(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    Ok(TransferResponse {
        network: NETWORK_NAME.to_string(),
        accepted: false,
        tx_id: None,
        message: format!(
            "{NETWORK_NAME} transfer scaffold received request; signing/execution not implemented"
        ),
    })
}

pub async fn transfer_eth(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    evm_rpc::transfer_native_eth(NETWORK_NAME, req).await
}

pub async fn transfer_erc20(req: TransferRequest) -> WalletResult<TransferResponse> {
    validate_transfer(&req)?;
    evm_rpc::transfer_erc20(NETWORK_NAME, req).await
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
