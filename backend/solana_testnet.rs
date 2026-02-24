use crate::chains::sol;
use crate::error::WalletResult;
use crate::types::{
    self, AddressResponse, BalanceRequest, BalanceResponse, TransferRequest, TransferResponse,
};

const NETWORK_NAME: &str = types::networks::SOLANA_TESTNET;

pub async fn request_address() -> WalletResult<AddressResponse> {
    sol::request_address_for_network(NETWORK_NAME).await
}

pub async fn get_balance(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    sol::get_balance_for_network(NETWORK_NAME, req).await
}

pub async fn transfer_sol(req: TransferRequest) -> WalletResult<TransferResponse> {
    sol::transfer_sol_for_network(NETWORK_NAME, req).await
}

pub async fn transfer_spl(req: TransferRequest) -> WalletResult<TransferResponse> {
    sol::transfer_spl_for_network(NETWORK_NAME, req).await
}
