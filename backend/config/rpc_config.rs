#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvmNetworkConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub chain_id: u64,
    pub default_rpc_url: &'static str,
    pub primary_symbol: &'static str,
    pub aliases: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalletNetworkInfo {
    pub id: &'static str,
    pub kind: &'static str,
    pub name: &'static str,
    pub primary_symbol: &'static str,
    pub supports_send: bool,
    pub supports_balance: bool,
    pub default_rpc_url: Option<&'static str>,
}

pub const DEFAULT_SOLANA_RPC_URL: &str = "https://solana-rpc.publicnode.com";
pub const DEFAULT_SOLANA_TESTNET_RPC_URL: &str = "https://solana-testnet-rpc.publicnode.com";

const EVM_NETWORKS: &[EvmNetworkConfig] = &[
    EvmNetworkConfig {
        id: "eth",
        name: "Ethereum",
        chain_id: 1,
        default_rpc_url: "https://ethereum-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["ethereum", "mainnet"],
    },
    EvmNetworkConfig {
        id: "sepolia",
        name: "Sepolia",
        chain_id: 11155111,
        default_rpc_url: "https://ethereum-sepolia-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["eth-sepolia", "ethereum-sepolia"],
    },
    EvmNetworkConfig {
        id: "base",
        name: "Base",
        chain_id: 8453,
        default_rpc_url: "https://base-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &[],
    },
    EvmNetworkConfig {
        id: "polygon",
        name: "Polygon",
        chain_id: 137,
        default_rpc_url: "https://polygon-bor-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["matic"],
    },
    EvmNetworkConfig {
        id: "arbitrum",
        name: "Arbitrum",
        chain_id: 42161,
        default_rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["arb", "arbitrum-one"],
    },
    EvmNetworkConfig {
        id: "optimism",
        name: "Optimism",
        chain_id: 10,
        default_rpc_url: "https://optimism-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["op", "optimism-mainnet"],
    },
    EvmNetworkConfig {
        id: "bsc",
        name: "BNB Chain",
        chain_id: 56,
        default_rpc_url: "https://bsc-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["bnb", "bsc-mainnet", "binance-smart-chain"],
    },
    EvmNetworkConfig {
        id: "avalanche",
        name: "Avalanche C-Chain",
        chain_id: 43114,
        default_rpc_url: "https://avalanche-c-chain-rpc.publicnode.com",
        primary_symbol: "ETH",
        aliases: &["avax", "avalanche-c"],
    },
];

const INTERNET_COMPUTER_NETWORK: WalletNetworkInfo = WalletNetworkInfo {
    id: "icp",
    kind: "icp",
    name: "Internet Computer",
    primary_symbol: "ICP",
    supports_send: true,
    supports_balance: true,
    default_rpc_url: None,
};

const SOLANA_NETWORK: WalletNetworkInfo = WalletNetworkInfo {
    id: "sol",
    kind: "solana",
    name: "Solana",
    primary_symbol: "SOL",
    supports_send: true,
    supports_balance: true,
    default_rpc_url: Some(DEFAULT_SOLANA_RPC_URL),
};

pub fn supported_networks() -> Vec<&'static str> {
    EVM_NETWORKS.iter().map(|cfg| cfg.id).collect()
}

pub fn wallet_networks() -> Vec<WalletNetworkInfo> {
    let mut out = Vec::with_capacity(EVM_NETWORKS.len() + 2);
    out.push(INTERNET_COMPUTER_NETWORK);
    for cfg in EVM_NETWORKS {
        out.push(WalletNetworkInfo {
            id: cfg.id,
            kind: "evm",
            name: cfg.name,
            primary_symbol: cfg.primary_symbol,
            supports_send: true,
            supports_balance: true,
            default_rpc_url: Some(cfg.default_rpc_url),
        });
    }
    out.push(SOLANA_NETWORK);
    out
}

pub fn normalize_network(network: &str) -> String {
    let n = normalize_text(network);
    match find_evm_by_id_or_alias(&n) {
        Some(cfg) => cfg.id.to_string(),
        None => n,
    }
}

pub fn normalize_wallet_network(network: &str) -> String {
    let n = normalize_text(network);
    if n.is_empty()
        || n == "internet_computer"
        || n == "internet-computer"
        || n == "icp"
        || n == "ic"
    {
        "icp".to_string()
    } else if n == "sol" || n == "solana" {
        "sol".to_string()
    } else {
        normalize_network(&n)
    }
}

pub fn wallet_network_info(network: &str) -> Option<WalletNetworkInfo> {
    let n = normalize_wallet_network(network);
    if n == INTERNET_COMPUTER_NETWORK.id {
        return Some(INTERNET_COMPUTER_NETWORK);
    }
    if n == SOLANA_NETWORK.id {
        return Some(SOLANA_NETWORK);
    }
    find_evm_by_id_or_alias(&n).map(|cfg| WalletNetworkInfo {
        id: cfg.id,
        kind: "evm",
        name: cfg.name,
        primary_symbol: cfg.primary_symbol,
        supports_send: true,
        supports_balance: true,
        default_rpc_url: Some(cfg.default_rpc_url),
    })
}

// This mirrors the Motoko behavior: EVM aliases or custom EVM chain IDs are considered supported.
pub fn is_supported(network: &str) -> bool {
    find_evm_by_id_or_alias(&normalize_text(network)).is_some()
        || parse_custom_chain_id(network).is_some()
}

pub fn chain_id(network: &str) -> Option<u64> {
    match find_evm_by_id_or_alias(&normalize_text(network)) {
        Some(cfg) => Some(cfg.chain_id),
        None => parse_custom_chain_id(network),
    }
}

pub fn default_rpc_url(network: &str) -> Option<&'static str> {
    find_evm_by_id_or_alias(&normalize_text(network)).map(|cfg| cfg.default_rpc_url)
}

pub fn effective_rpc_url(network: &str, rpc_url: Option<&str>) -> Option<String> {
    if let Some(u) = rpc_url {
        let t = u.trim();
        if !t.is_empty() {
            return Some(t.to_string());
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
    match rpc_url.map(str::trim).filter(|s| !s.is_empty()) {
        Some(url) => url.to_string(),
        None => DEFAULT_SOLANA_RPC_URL.to_string(),
    }
}

pub fn effective_solana_testnet_rpc_url(rpc_url: Option<&str>) -> String {
    match rpc_url.map(str::trim).filter(|s| !s.is_empty()) {
        Some(url) => url.to_string(),
        None => DEFAULT_SOLANA_TESTNET_RPC_URL.to_string(),
    }
}

fn find_evm_by_id_or_alias(network: &str) -> Option<&'static EvmNetworkConfig> {
    if network.is_empty() {
        return None;
    }
    EVM_NETWORKS
        .iter()
        .find(|cfg| cfg.id == network || cfg.aliases.iter().any(|alias| *alias == network))
}

fn normalize_text(value: &str) -> String {
    value.trim().to_lowercase()
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
    fn normalizes_aliases() {
        assert_eq!(normalize_network(" ETH "), "eth");
        assert_eq!(normalize_network("arb"), "arbitrum");
        assert_eq!(normalize_wallet_network("ic"), "icp");
        assert_eq!(normalize_wallet_network("sol"), "sol");
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
        assert_eq!(
            resolve_rpc_url("eth", None).as_deref(),
            Ok("https://ethereum-rpc.publicnode.com")
        );
        assert!(resolve_rpc_url("eip155:99999", None).is_err());
        assert_eq!(
            resolve_rpc_url("eip155:99999", Some(" https://rpc.example ")).unwrap(),
            "https://rpc.example"
        );
    }
}
