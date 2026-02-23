use crate::addressing;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest,
    TransferResponse,
};

const NETWORK_NAME: &str = "sol";

pub async fn request_address(req: AddressRequest) -> WalletResult<AddressResponse> {
    let resolved = addressing::resolve_address_request(NETWORK_NAME, req)?;
    let (public_key, key_name) = addressing::fetch_schnorr_public_key(
        ic_cdk::management_canister::SchnorrAlgorithm::Ed25519,
        resolved.derivation_path,
    )
    .await?;

    if public_key.len() != 32 {
        return Err(WalletError::Internal(format!(
            "unexpected ed25519 public key length for sol address: {}",
            public_key.len()
        )));
    }

    Ok(AddressResponse {
        network: NETWORK_NAME.to_string(),
        address: addressing::base58_encode(&public_key),
        public_key_hex: addressing::hex_encode(&public_key),
        key_name,
        index: resolved.index,
        account_tag: resolved.account_tag,
        message: Some("Derived from management canister Schnorr(ed25519) public key".into()),
    })
}

pub fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: req.token,
        amount: None,
        decimals: Some(9),
        block_ref: None,
        pending: true,
        message: Some("Solana balance query not implemented yet".to_string()),
    })
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
