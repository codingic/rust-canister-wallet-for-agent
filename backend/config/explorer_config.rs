use crate::types::networks;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExplorerConfig {
    pub network: &'static str,
    pub address_url_template: &'static str,
    pub token_url_template: Option<&'static str>,
}

pub fn configured_explorer(network: &str) -> Option<ExplorerConfig> {
    let normalized = normalize_config_network_name(network);
    match normalized.as_str() {
        networks::ETHEREUM => Some(ExplorerConfig {
            network: networks::ETHEREUM,
            address_url_template: "https://etherscan.io/address/{address}",
            token_url_template: Some("https://etherscan.io/token/{token}?a={address}"),
        }),
        networks::SEPOLIA => Some(ExplorerConfig {
            network: networks::SEPOLIA,
            address_url_template: "https://sepolia.etherscan.io/address/{address}",
            token_url_template: Some("https://sepolia.etherscan.io/token/{token}?a={address}"),
        }),
        networks::BASE => Some(ExplorerConfig {
            network: networks::BASE,
            address_url_template: "https://basescan.org/address/{address}",
            token_url_template: Some("https://basescan.org/token/{token}?a={address}"),
        }),
        networks::BSC => Some(ExplorerConfig {
            network: networks::BSC,
            address_url_template: "https://bscscan.com/address/{address}",
            token_url_template: Some("https://bscscan.com/token/{token}?a={address}"),
        }),
        networks::ARBITRUM => Some(ExplorerConfig {
            network: networks::ARBITRUM,
            address_url_template: "https://arbiscan.io/address/{address}",
            token_url_template: Some("https://arbiscan.io/token/{token}?a={address}"),
        }),
        networks::OPTIMISM => Some(ExplorerConfig {
            network: networks::OPTIMISM,
            address_url_template: "https://optimistic.etherscan.io/address/{address}",
            token_url_template: Some("https://optimistic.etherscan.io/token/{token}?a={address}"),
        }),
        networks::AVALANCHE => Some(ExplorerConfig {
            network: networks::AVALANCHE,
            address_url_template: "https://snowtrace.io/address/{address}",
            token_url_template: Some("https://snowtrace.io/token/{token}?a={address}"),
        }),
        networks::POLYGON => Some(ExplorerConfig {
            network: networks::POLYGON,
            address_url_template: "https://polygonscan.com/address/{address}",
            token_url_template: Some("https://polygonscan.com/token/{token}?a={address}"),
        }),
        networks::OKX => Some(ExplorerConfig {
            network: networks::OKX,
            //https://www.oklink.com/zh-hans/x-layer/address/0xa1d2c4533d867ce4623681f68df84d9cad73cb6b
            //https://www.oklink.com/zh-hans/x-layer/tx/0x35c460a65ade91b5ecc8d89eaa4b67627aecfb66352bc46550f40fe29fab1aeb
            address_url_template: "https://www.oklink.com/zh-hans/x-layer/address/{address}",
            token_url_template: Some(
                "https://www.oklink.com/zh-hans/x-layer/token/{token}?tab=holders",
            ),
        }),
        networks::BITCOIN => Some(ExplorerConfig {
            network: networks::BITCOIN,
            address_url_template: "https://mempool.space/address/{address}",
            token_url_template: None,
        }),
        networks::INTERNET_COMPUTER => Some(ExplorerConfig {
            network: networks::INTERNET_COMPUTER,
            address_url_template: "https://dashboard.internetcomputer.org/canister/{address}",
            token_url_template: Some("https://dashboard.internetcomputer.org/canister/{token}"),
        }),
        networks::SOLANA => Some(ExplorerConfig {
            network: networks::SOLANA,
            address_url_template: "https://solscan.io/account/{address}",
            token_url_template: Some("https://solscan.io/token/{token}"),
        }),
        networks::SOLANA_TESTNET => Some(ExplorerConfig {
            network: networks::SOLANA_TESTNET,
            address_url_template: "https://solscan.io/account/{address}?cluster=testnet",
            token_url_template: Some("https://solscan.io/token/{token}?cluster=testnet"),
        }),
        networks::TRON => Some(ExplorerConfig {
            network: networks::TRON,
            address_url_template: "https://tronscan.org/#/address/{address}",
            token_url_template: Some("https://tronscan.org/#/token20/{token}"),
        }),
        networks::TON_MAINNET => Some(ExplorerConfig {
            network: networks::TON_MAINNET,
            address_url_template: "https://tonviewer.com/{address}",
            token_url_template: Some("https://tonviewer.com/{token}"),
        }),
        networks::NEAR_MAINNET => Some(ExplorerConfig {
            network: networks::NEAR_MAINNET,
            address_url_template: "https://nearblocks.io/address/{address}",
            token_url_template: Some("https://nearblocks.io/token/{token}"),
        }),
        networks::APTOS_MAINNET => Some(ExplorerConfig {
            network: networks::APTOS_MAINNET,
            address_url_template:
                "https://explorer.aptoslabs.com/account/{address}?network=mainnet",
            token_url_template: Some(
                "https://explorer.aptoslabs.com/account/{token}?network=mainnet",
            ),
        }),
        networks::SUI_MAINNET => Some(ExplorerConfig {
            network: networks::SUI_MAINNET,
            address_url_template: "https://suiscan.xyz/mainnet/account/{address}",
            token_url_template: Some("https://suiscan.xyz/mainnet/coin/{token}"),
        }),
        _ => None,
    }
}

fn normalize_config_network_name(network: &str) -> String {
    network.trim().to_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_config_follows_network_names() {
        assert!(configured_explorer(networks::ETHEREUM).is_some());
        assert_eq!(
            configured_explorer(networks::ARBITRUM).unwrap().network,
            networks::ARBITRUM
        );
        assert_eq!(
            configured_explorer(networks::OPTIMISM).unwrap().network,
            networks::OPTIMISM
        );
        assert_eq!(
            configured_explorer(networks::AVALANCHE).unwrap().network,
            networks::AVALANCHE
        );
        assert!(configured_explorer("unknown").is_none());
    }
}
