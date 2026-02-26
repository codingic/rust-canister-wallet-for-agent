export const BUILTIN_DEFAULT_RPCS = {
  ethereum: 'https://ethereum-rpc.publicnode.com',
  sepolia: 'https://ethereum-sepolia-rpc.publicnode.com',
  base: 'https://base-rpc.publicnode.com',
  bsc: 'https://bsc-rpc.publicnode.com',
  arbitrum: 'https://arbitrum-one-rpc.publicnode.com',
  optimism: 'https://optimism-rpc.publicnode.com',
  avalanche: 'https://avalanche-c-chain-rpc.publicnode.com',
  okx: 'https://exchainrpc.okex.org',
  polygon: 'https://polygon-bor-rpc.publicnode.com',
  bitcoin: 'https://blockstream.info/api',
  solana: 'https://api.mainnet-beta.solana.com',
  solana_testnet: 'https://api.testnet.solana.com',
  tron: 'https://api.trongrid.io',
  ton_mainnet: 'https://toncenter.com/api/v2',
  near_mainnet: 'https://rpc.mainnet.near.org',
  aptos_mainnet: 'https://fullnode.mainnet.aptoslabs.com/v1',
  sui_mainnet: 'https://fullnode.mainnet.sui.io:443',
  internet_computer: 'https://icp-api.io'
};

export const EVM_NATIVE_ASSET_KIND = {
  ethereum: 'eth',
  sepolia: 'eth',
  base: 'eth',
  bsc: 'bnb',
  arbitrum: 'eth',
  optimism: 'eth',
  avalanche: 'avax',
  okx: 'okb',
  polygon: 'pol'
};

export const EVM_TOKEN_ASSET_KIND = {
  ethereum: 'erc20',
  sepolia: 'erc20',
  base: 'erc20',
  bsc: 'bep20',
  arbitrum: 'erc20',
  optimism: 'erc20',
  avalanche: 'erc20',
  okx: 'erc20',
  polygon: 'erc20'
};

export const BALANCE_METHOD_PATTERN = '<network_prefix>_get_balance_<asset_kind>';
