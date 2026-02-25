use crate::config::token_list::{
    arbitrum, avalanche, base, bsc, ethereum, internet_computer, optimism, polygon, sepolia, solana,
};
use crate::types::networks;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfiguredToken {
    pub network: &'static str,
    pub symbol: &'static str,
    pub name: &'static str,
    pub token_address: &'static str,
    pub decimals: u64,
}

pub fn configured_tokens(network: &str) -> &'static [ConfiguredToken] {
    match normalize_config_network_name(network).as_str() {
        networks::INTERNET_COMPUTER => internet_computer::TOKENS,
        networks::ETHEREUM => ethereum::TOKENS,
        networks::SEPOLIA => sepolia::TOKENS,
        networks::BASE => base::TOKENS,
        networks::POLYGON => polygon::TOKENS,
        networks::ARBITRUM => arbitrum::TOKENS,
        networks::OPTIMISM => optimism::TOKENS,
        networks::BSC => bsc::TOKENS,
        networks::AVALANCHE => avalanche::TOKENS,
        networks::SOLANA => solana::TOKENS,
        _ => &[],
    }
}

fn normalize_config_network_name(network: &str) -> String {
    network.trim().to_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_lists_follow_network_names() {
        assert!(!configured_tokens(networks::ETHEREUM).is_empty());
        let icp_tokens = configured_tokens(networks::INTERNET_COMPUTER);
        assert!(!icp_tokens.is_empty());
        assert!(icp_tokens.iter().any(|t| t.symbol == "CHAT"));
        assert_eq!(configured_tokens(networks::SOLANA).len(), 1);
    }
}
