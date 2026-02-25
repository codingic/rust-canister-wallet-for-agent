use crate::config::token_list_config::ConfiguredToken;

pub const TOKENS: &[ConfiguredToken] = &[
    ConfiguredToken {
        network: "base",
        symbol: "USDC",
        name: "USD Coin",
        token_address: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        decimals: 6,
    },
    ConfiguredToken {
        network: "base",
        symbol: "CLAWNCH",
        name: "CLAWNCH",
        token_address: "0xa1F72459dfA10BAD200Ac160eCd78C6b77a747be",
        decimals: 18,
    },
];
