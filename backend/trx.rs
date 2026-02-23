use crate::error::{WalletError, WalletResult};
use crate::types::{BalanceRequest, BalanceResponse, TransferRequest, TransferResponse};

const NETWORK_NAME: &str = "trx";

pub fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    validate_account(&req.account)?;
    Ok(BalanceResponse {
        network: NETWORK_NAME.to_string(),
        account: req.account,
        token: req.token,
        amount: None,
        decimals: Some(6),
        block_ref: None,
        pending: true,
        message: Some("TRON balance query not implemented yet".to_string()),
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
