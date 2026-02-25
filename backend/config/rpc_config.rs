use crate::state;
use crate::types::networks;

pub const DEFAULT_SOLANA_RPC_URL: &str = "https://solana-rpc.publicnode.com";
pub const DEFAULT_SOLANA_TESTNET_RPC_URL: &str = "https://solana-testnet-rpc.publicnode.com";
pub const DEFAULT_ETHEREUM_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
pub const DEFAULT_SEPOLIA_RPC_URL: &str = "https://ethereum-sepolia-rpc.publicnode.com";
pub const DEFAULT_BASE_RPC_URL: &str = "https://base-rpc.publicnode.com";
pub const DEFAULT_POLYGON_RPC_URL: &str = "https://polygon-bor-rpc.publicnode.com";
pub const DEFAULT_ARBITRUM_RPC_URL: &str = "https://arbitrum-one-rpc.publicnode.com";
pub const DEFAULT_OPTIMISM_RPC_URL: &str = "https://optimism-rpc.publicnode.com";
pub const DEFAULT_BSC_RPC_URL: &str = "https://bsc-rpc.publicnode.com";
pub const DEFAULT_AVALANCHE_RPC_URL: &str = "https://avalanche-c-chain-rpc.publicnode.com";
pub const DEFAULT_OKX_RPC_URL: &str = "https://xlayerrpc.okx.com";
pub const DEFAULT_TRON_RPC_URL: &str = "https://tron-rpc.publicnode.com"; //https://tron-evm-rpc.publicnode.com
pub const DEFAULT_TON_RPC_URL: &str = "https://toncenter.com/api/v2";
pub const DEFAULT_NEAR_RPC_URL: &str = "https://rpc.mainnet.near.org";
pub const DEFAULT_APTOS_RPC_URL: &str = "https://fullnode.mainnet.aptoslabs.com/v1";
pub const DEFAULT_SUI_RPC_URL: &str = "https://fullnode.mainnet.sui.io:443";

pub const DEFAULT_BITCOIN_RPC_URL: &str = "https://blockstream.info/api";

pub const TEST_CUSTOM_RPC_URL: &str = "https://rpc.example";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainConfig {
    pub id: &'static str,
    pub primary_symbol: &'static str,
    pub address_family: &'static str,
    pub shared_address_group: &'static str,
    pub supports_send: bool,
    pub supports_balance: bool,
    pub default_rpc_url: Option<&'static str>,
    pub chain_id: Option<u64>,
    pub wallet_visible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalletNetworkInfo {
    pub id: &'static str,
    pub primary_symbol: &'static str,
    pub address_family: &'static str,
    pub shared_address_group: &'static str,
    pub supports_send: bool,
    pub supports_balance: bool,
    pub default_rpc_url: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcConfig {
    pub network: String,
    pub rpc_url: String,
}

const CHAIN_CONFIGS: &[ChainConfig] = &[
    ChainConfig {
        id: networks::BITCOIN,
        primary_symbol: "BTC",
        address_family: "bitcoin",
        shared_address_group: "btc-taproot-managed-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_BITCOIN_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::INTERNET_COMPUTER,
        primary_symbol: "ICP",
        address_family: "icp",
        shared_address_group: "icp-canister-principal-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: None,
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::ETHEREUM,
        primary_symbol: "ETH",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_ETHEREUM_RPC_URL),
        chain_id: Some(1),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::SEPOLIA,
        primary_symbol: "ETH",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_SEPOLIA_RPC_URL),
        chain_id: Some(11155111),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::BASE,
        primary_symbol: "ETH",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_BASE_RPC_URL),
        chain_id: Some(8453),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::POLYGON,
        primary_symbol: "POL",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_POLYGON_RPC_URL),
        chain_id: Some(137),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::ARBITRUM,
        primary_symbol: "ETH",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_ARBITRUM_RPC_URL),
        chain_id: Some(42161),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::OPTIMISM,
        primary_symbol: "ETH",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_OPTIMISM_RPC_URL),
        chain_id: Some(10),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::BSC,
        primary_symbol: "BNB",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_BSC_RPC_URL),
        chain_id: Some(56),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::AVALANCHE,
        primary_symbol: "AVAX",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_AVALANCHE_RPC_URL),
        chain_id: Some(43114),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::OKX,
        primary_symbol: "OKB",
        address_family: "evm",
        shared_address_group: "evm-secp256k1-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_OKX_RPC_URL),
        chain_id: Some(196),
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::SOLANA,
        primary_symbol: "SOL",
        address_family: "solana",
        shared_address_group: "solana-ed25519-base58-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_SOLANA_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::SOLANA_TESTNET,
        primary_symbol: "SOL",
        address_family: "solana",
        shared_address_group: "solana-ed25519-base58-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_SOLANA_TESTNET_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::TRON,
        primary_symbol: "TRX",
        address_family: "tron",
        shared_address_group: "tron-secp256k1-base58check-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_TRON_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::TON_MAINNET,
        primary_symbol: "TON",
        address_family: "ton",
        shared_address_group: "ton-wallet-v4r2-ed25519-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_TON_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::NEAR_MAINNET,
        primary_symbol: "NEAR",
        address_family: "near",
        shared_address_group: "near-implicit-ed25519-hex-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_NEAR_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::APTOS_MAINNET,
        primary_symbol: "APT",
        address_family: "aptos",
        shared_address_group: "aptos-authkey-ed25519-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_APTOS_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
    ChainConfig {
        id: networks::SUI_MAINNET,
        primary_symbol: "SUI",
        address_family: "sui",
        shared_address_group: "sui-blake2b-ed25519-v1",
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(DEFAULT_SUI_RPC_URL),
        chain_id: None,
        wallet_visible: true,
    },
];

pub fn supported_networks() -> Vec<&'static str> {
    CHAIN_CONFIGS
        .iter()
        .filter(|cfg| cfg.chain_id.is_some())
        .map(|cfg| cfg.id)
        .collect()
}

pub fn wallet_networks() -> Vec<WalletNetworkInfo> {
    CHAIN_CONFIGS
        .iter()
        .filter(|cfg| cfg.wallet_visible)
        .map(chain_wallet_info)
        .collect()
}

pub fn normalize_network(network: &str) -> String {
    let n = normalize_text(network);
    match find_chain_by_input(&n) {
        Some(cfg) => cfg.id.to_string(),
        None => n,
    }
}

pub fn wallet_network_info(network: &str) -> Option<WalletNetworkInfo> {
    let normalized = normalize_text(network);
    if normalized.is_empty() {
        return find_chain_by_id(networks::INTERNET_COMPUTER).map(chain_wallet_info);
    }
    find_wallet_chain_by_input(&normalized).map(chain_wallet_info)
}

pub fn configured_rpc(network: &str) -> Option<RpcConfig> {
    let normalized = normalize_text(network);
    if normalized.is_empty() {
        return None;
    }
    let rpc_url = state::configured_rpc(&normalized).or_else(|| {
        find_chain_by_input(&normalized)
            .and_then(|cfg| cfg.default_rpc_url.map(ToString::to_string))
    })?;
    Some(RpcConfig {
        network: normalized,
        rpc_url,
    })
}

// This mirrors the Motoko behavior: known chain IDs or custom EVM chain IDs are considered supported.
pub fn is_supported(network: &str) -> bool {
    wallet_network_info(network).is_some() || parse_custom_chain_id(network).is_some()
}

pub fn chain_id(network: &str) -> Option<u64> {
    match find_chain_by_input(network) {
        Some(cfg) => cfg.chain_id,
        None => parse_custom_chain_id(network),
    }
}

pub fn default_rpc_url(network: &str) -> Option<&'static str> {
    find_chain_by_input(network).and_then(|cfg| cfg.default_rpc_url)
}

pub fn effective_rpc_url(network: &str, rpc_url: Option<&str>) -> Option<String> {
    if let Some(u) = rpc_url {
        let t = u.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let normalized = normalize_text(network);
    if !normalized.is_empty() {
        if let Some(runtime) = state::configured_rpc(&normalized) {
            return Some(runtime);
        }
    }
    default_rpc_url(network).map(ToString::to_string)
}

pub fn resolve_rpc_url(network: &str, rpc_url: Option<&str>) -> Result<String, String> {
    match effective_rpc_url(network, rpc_url) {
        Some(u) => Ok(u),
        None => {
            if parse_custom_chain_id(network).is_some() {
                Err(format!("rpcUrl is required for custom network: {network}"))
            } else {
                Err(format!("unsupported network: {network}"))
            }
        }
    }
}

pub fn effective_solana_rpc_url(rpc_url: Option<&str>) -> String {
    effective_optional_url(rpc_url).unwrap_or_else(|| DEFAULT_SOLANA_RPC_URL.to_string())
}

pub fn effective_solana_testnet_rpc_url(rpc_url: Option<&str>) -> String {
    effective_optional_url(rpc_url).unwrap_or_else(|| DEFAULT_SOLANA_TESTNET_RPC_URL.to_string())
}

fn find_chain_by_id(network: &str) -> Option<&'static ChainConfig> {
    if network.is_empty() {
        return None;
    }
    CHAIN_CONFIGS.iter().find(|cfg| cfg.id == network)
}

fn find_chain_by_input(network: &str) -> Option<&'static ChainConfig> {
    let normalized = normalize_text(network);
    find_chain_by_id(&normalized)
}

fn find_wallet_chain_by_input(network: &str) -> Option<&'static ChainConfig> {
    find_chain_by_input(network).filter(|cfg| cfg.wallet_visible)
}

fn chain_wallet_info(cfg: &ChainConfig) -> WalletNetworkInfo {
    WalletNetworkInfo {
        id: cfg.id,
        primary_symbol: cfg.primary_symbol,
        address_family: cfg.address_family,
        shared_address_group: cfg.shared_address_group,
        supports_send: cfg.supports_send,
        supports_balance: cfg.supports_balance,
        default_rpc_url: cfg.default_rpc_url,
    }
}

fn normalize_text(value: &str) -> String {
    value.trim().to_lowercase().replace('-', "_")
}

fn effective_optional_url(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn parse_custom_chain_id(network: &str) -> Option<u64> {
    let n = normalize_network(network);
    let mut parts = n.split(':');
    let prefix = parts.next()?;
    let chain_id_text = parts.next()?.trim();
    if parts.next().is_some() {
        return None;
    }
    if !matches!(prefix, "eip155" | "chainid" | "evm") {
        return None;
    }
    let parsed = chain_id_text.parse::<u64>().ok()?;
    (parsed != 0).then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_inputs() {
        assert_eq!(normalize_network(" ETH "), "eth");
        assert_eq!(normalize_network("arb"), "arb");
        assert_eq!(normalize_network("TON_MAINNET"), networks::TON_MAINNET);
    }

    #[test]
    fn configured_rpc_follows_network_names() {
        assert_eq!(
            configured_rpc(networks::ETHEREUM).unwrap(),
            RpcConfig {
                network: networks::ETHEREUM.to_string(),
                rpc_url: DEFAULT_ETHEREUM_RPC_URL.to_string(),
            }
        );
        assert!(configured_rpc("eth").is_none());
        assert_eq!(
            configured_rpc("SOLANA_TESTNET").unwrap(),
            RpcConfig {
                network: networks::SOLANA_TESTNET.to_string(),
                rpc_url: DEFAULT_SOLANA_TESTNET_RPC_URL.to_string(),
            }
        );
        assert!(configured_rpc("unknown").is_none());
    }

    #[test]
    fn wallet_network_info_maps_public_network_names() {
        assert_eq!(
            wallet_network_info(networks::INTERNET_COMPUTER).unwrap().id,
            networks::INTERNET_COMPUTER
        );
        assert_eq!(
            wallet_network_info(networks::SOLANA).unwrap().id,
            networks::SOLANA
        );
        assert_eq!(
            wallet_network_info(networks::SOLANA_TESTNET).unwrap().id,
            networks::SOLANA_TESTNET
        );
        assert_eq!(
            wallet_network_info(networks::ETHEREUM).unwrap().id,
            networks::ETHEREUM
        );
        assert_eq!(
            wallet_network_info(networks::OKX).unwrap().id,
            networks::OKX
        );
        assert_eq!(
            wallet_network_info(networks::BITCOIN).unwrap().id,
            networks::BITCOIN
        );
        assert_eq!(
            wallet_network_info(networks::TON_MAINNET).unwrap().id,
            networks::TON_MAINNET
        );
        assert_eq!(
            wallet_network_info(networks::NEAR_MAINNET).unwrap().id,
            networks::NEAR_MAINNET
        );
        assert_eq!(
            wallet_network_info(networks::ETHEREUM)
                .unwrap()
                .shared_address_group,
            "evm-secp256k1-hex-v1"
        );
        assert_eq!(
            wallet_network_info(networks::SEPOLIA)
                .unwrap()
                .shared_address_group,
            "evm-secp256k1-hex-v1"
        );
        assert_eq!(
            wallet_network_info(networks::SOLANA)
                .unwrap()
                .shared_address_group,
            wallet_network_info(networks::SOLANA_TESTNET)
                .unwrap()
                .shared_address_group
        );
    }

    #[test]
    fn parses_custom_chain_ids() {
        assert_eq!(chain_id("eip155:84532"), Some(84532));
        assert_eq!(chain_id("evm:10"), Some(10));
        assert_eq!(chain_id("chainid:0"), None);
        assert_eq!(chain_id("foo:1"), None);
    }

    #[test]
    fn resolves_rpc_urls() {
        let custom_rpc_url = format!(" {TEST_CUSTOM_RPC_URL} ");
        assert_eq!(
            resolve_rpc_url(networks::ETHEREUM, None).as_deref(),
            Ok(DEFAULT_ETHEREUM_RPC_URL)
        );
        assert!(resolve_rpc_url("eip155:99999", None).is_err());
        assert_eq!(
            resolve_rpc_url("eip155:99999", Some(custom_rpc_url.as_str())).unwrap(),
            TEST_CUSTOM_RPC_URL
        );
    }
}
