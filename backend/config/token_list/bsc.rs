use crate::config::token_list_config::ConfiguredToken;

pub const TOKENS: &[ConfiguredToken] = &[
    ConfiguredToken {
        network: "bsc",
        symbol: "USDC",
        name: "USD Coin",
        token_address: "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
        decimals: 18,
    },
    ConfiguredToken {
        network: "bsc",
        symbol: "USDT",
        name: "Tether USD",
        token_address: "0x55d398326f99059ff775485246999027b3197955",
        decimals: 18,
    },
];
