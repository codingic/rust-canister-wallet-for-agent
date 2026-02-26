export const TOKEN_VLIST_HEIGHT = 420;
export const TOKEN_VLIST_ROW_HEIGHT = 124;
export const TOKEN_VLIST_OVERSCAN = 3;

export const NETWORK_CONFIG = {
  ethereum: {
    title: 'Ethereum',
    nativeSymbol: 'ETH',
    nativeLabel: 'ETH 地址',
    tokenLabel: 'Token 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  sepolia: {
    title: 'Sepolia',
    nativeSymbol: 'ETH',
    nativeLabel: 'Sepolia 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  base: {
    title: 'Base',
    nativeSymbol: 'ETH',
    nativeLabel: 'Base 地址',
    tokenLabel: 'Token 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  bsc: {
    title: 'BNB Smart Chain',
    nativeSymbol: 'BNB',
    nativeLabel: 'BSC 地址',
    tokenLabel: 'BEP20 合约地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  arbitrum: {
    title: 'Arbitrum',
    nativeSymbol: 'ETH',
    nativeLabel: 'Arbitrum 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  optimism: {
    title: 'Optimism',
    nativeSymbol: 'ETH',
    nativeLabel: 'Optimism 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  avalanche: {
    title: 'Avalanche',
    nativeSymbol: 'AVAX',
    nativeLabel: 'Avalanche 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC.e',
    showToken: true
  },
  okx: {
    title: 'OKX Chain',
    nativeSymbol: 'OKB',
    nativeLabel: 'OKX 链地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  polygon: {
    title: 'Polygon',
    nativeSymbol: 'POL',
    nativeLabel: 'Polygon 地址',
    tokenLabel: 'ERC20 合约地址',
    tokenSymbol: 'USDC',
    showToken: true
  },
  'internet-computer': {
    title: 'Internet Computer',
    nativeSymbol: 'ICP',
    nativeLabel: '账户地址',
    tokenLabel: 'ICRC Token Canister',
    tokenSymbol: 'ICRC',
    showToken: true
  },
  bitcoin: {
    title: 'Bitcoin',
    nativeSymbol: 'BTC',
    nativeLabel: 'BTC 地址',
    tokenLabel: 'Token 地址',
    tokenSymbol: '',
    showToken: true
  },
  solana: {
    title: 'Solana',
    nativeSymbol: 'SOL',
    nativeLabel: 'Solana 地址',
    tokenLabel: 'SPL Token Mint',
    tokenSymbol: 'USDC',
    showToken: true
  },
  'solana-testnet': {
    title: 'Solana Testnet',
    nativeSymbol: 'SOL',
    nativeLabel: 'Solana Testnet 地址',
    tokenLabel: 'SPL Token Mint',
    tokenSymbol: 'USDC',
    showToken: true
  },
  tron: {
    title: 'TRON',
    nativeSymbol: 'TRX',
    nativeLabel: 'TRX 地址',
    tokenLabel: 'TRC20 合约地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  'ton-mainnet': {
    title: 'TON',
    nativeSymbol: 'TON',
    nativeLabel: 'TON 地址',
    tokenLabel: 'Jetton Master 地址',
    tokenSymbol: 'USDT',
    showToken: true
  },
  'near-mainnet': {
    title: 'NEAR',
    nativeSymbol: 'NEAR',
    nativeLabel: 'NEAR 账户',
    tokenLabel: 'NEP-141 Token 合约',
    tokenSymbol: 'USDT',
    showToken: true
  },
  'aptos-mainnet': {
    title: 'Aptos',
    nativeSymbol: 'APT',
    nativeLabel: 'Aptos 地址',
    tokenLabel: 'Token 地址',
    tokenSymbol: 'APT',
    showToken: true
  },
  'sui-mainnet': {
    title: 'Sui',
    nativeSymbol: 'SUI',
    nativeLabel: 'Sui 地址',
    tokenLabel: 'Token Type',
    tokenSymbol: 'SUI',
    showToken: true
  }
};

export const DEFAULT_NETWORK_ORDER = [
  'ethereum',
  'sepolia',
  'base',
  'bsc',
  'arbitrum',
  'optimism',
  'avalanche',
  'okx',
  'polygon',
  'internet-computer',
  'bitcoin',
  'solana',
  'solana-testnet',
  'tron',
  'ton-mainnet',
  'near-mainnet',
  'aptos-mainnet',
  'sui-mainnet'
];

export function normalizeNetworkId(networkId) {
  const n = String(networkId || '')
    .trim()
    .toLowerCase()
    .replaceAll('_', '-');
  if (!n) return '';
  if (n === 'internetcomputer') return 'internet-computer';
  return n;
}

export function fallbackNetworkConfig(networkId) {
  const upper = String(networkId || 'unknown').toUpperCase();
  return {
    title: upper,
    nativeSymbol: upper,
    nativeLabel: `${upper} 地址`,
    tokenLabel: 'Token 地址',
    tokenSymbol: 'Token',
    showToken: false
  };
}
