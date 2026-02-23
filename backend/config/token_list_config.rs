use crate::config::rpc_config;
use crate::config::token_list::{
    arbitrum, avalanche, base, bsc, ethereum, icp, optimism, polygon, sepolia, solana,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfiguredToken {
    pub network: &'static str,
    pub symbol: &'static str,
    pub name: &'static str,
    pub token_address: &'static str,
    pub decimals: u64,
}

pub fn configured_tokens(network: &str) -> &'static [ConfiguredToken] {
    match rpc_config::normalize_wallet_network(network).as_str() {
        "icp" => icp::TOKENS,
        "eth" => ethereum::TOKENS,
        "sepolia" => sepolia::TOKENS,
        "base" => base::TOKENS,
        "polygon" => polygon::TOKENS,
        "arbitrum" => arbitrum::TOKENS,
        "optimism" => optimism::TOKENS,
        "bsc" => bsc::TOKENS,
        "avalanche" => avalanche::TOKENS,
        "sol" => solana::TOKENS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_lists_follow_network_normalization() {
        assert!(!configured_tokens("eth").is_empty());
        assert_eq!(configured_tokens("ic").len(), 0);
        assert_eq!(configured_tokens("sol").len(), 1);
    }
}
