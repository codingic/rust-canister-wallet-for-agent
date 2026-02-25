use crate::addressing;
use crate::error::WalletResult;
use crate::types::{self, AddressResponse};

const NETWORK_NAME: &str = types::networks::ETHEREUM;

pub async fn request_address() -> WalletResult<AddressResponse> {
    addressing::derive_evm_address(NETWORK_NAME).await
}
