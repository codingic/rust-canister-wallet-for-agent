use crate::chains::{
    aptos_mainnet, internet_computer, near_mainnet, solana, sui_mainnet, ton_mainnet, tron,
};
use crate::error::{WalletError, WalletResult};
use crate::evm_rpc;
use crate::types::{self, ConfiguredTokenResponse};

pub async fn discover_token_metadata(
    network: &str,
    token_address: &str,
) -> WalletResult<ConfiguredTokenResponse> {
    let network = normalize_network_name(network);
    let token_address = token_address.trim();
    if token_address.is_empty() {
        return Err(WalletError::invalid_input("token_address is required"));
    }

    match network.as_str() {
        types::networks::ETHEREUM
        | types::networks::SEPOLIA
        | types::networks::BASE
        | types::networks::BSC
        | types::networks::ARBITRUM
        | types::networks::OPTIMISM
        | types::networks::AVALANCHE
        | types::networks::OKX
        | types::networks::POLYGON => evm_rpc::discover_erc20_token(&network, token_address).await,
        types::networks::INTERNET_COMPUTER => {
            internet_computer::discover_icrc_token(token_address).await
        }
        types::networks::SOLANA | types::networks::SOLANA_TESTNET => {
            solana::discover_spl_token(&network, token_address).await
        }
        types::networks::TRON => tron::discover_trc20_token(token_address).await,
        types::networks::TON_MAINNET => ton_mainnet::discover_jetton_token(token_address).await,
        types::networks::NEAR_MAINNET => near_mainnet::discover_nep141_token(token_address).await,
        types::networks::APTOS_MAINNET => {
            aptos_mainnet::discover_coin_type_token(token_address).await
        }
        types::networks::SUI_MAINNET => sui_mainnet::discover_coin_type_token(token_address).await,
        other => Err(WalletError::Unimplemented {
            network: other.to_string(),
            operation: "token metadata discovery".to_string(),
        }),
    }
}

pub fn normalize_network_name(input: &str) -> String {
    input.trim().to_lowercase().replace('-', "_")
}
