use crate::config::token_list_config::ConfiguredToken;

pub const TOKENS: &[ConfiguredToken] = &[
    ConfiguredToken {
        network: "polygon",
        symbol: "USDC",
        name: "USD Coin",
        token_address: "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
        decimals: 6,
    },
    ConfiguredToken {
        network: "polygon",
        symbol: "USDT",
        name: "Tether USD",
        token_address: "0xc2132d05d31c914a87c6611c10748aeb04b58e8f",
        decimals: 6,
    },
];
