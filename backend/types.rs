use candid::{CandidType, Principal};
use serde::Deserialize;

pub type Network = String;

pub mod networks {
    pub const BITCOIN: &str = "bitcoin";
    pub const ETHEREUM: &str = "ethereum";
    pub const SEPOLIA: &str = "sepolia";
    pub const BASE: &str = "base";
    pub const BSC: &str = "bsc";
    pub const ARBITRUM: &str = "arbitrum";
    pub const OPTIMISM: &str = "optimism";
    pub const AVALANCHE: &str = "avalanche";
    pub const OKX: &str = "okx";
    pub const POLYGON: &str = "polygon";
    pub const INTERNET_COMPUTER: &str = "internet_computer";
    pub const SOLANA: &str = "solana";
    pub const SOLANA_TESTNET: &str = "solana_testnet";
    pub const TRON: &str = "tron";
    pub const TON_MAINNET: &str = "ton_mainnet";
    pub const NEAR_MAINNET: &str = "near_mainnet";
    pub const APTOS_MAINNET: &str = "aptos_mainnet";
    pub const SUI_MAINNET: &str = "sui_mainnet";
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct AddressResponse {
    pub network: Network,
    pub address: String,
    pub public_key_hex: String,
    pub key_name: String,
    pub message: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ConfiguredTokenResponse {
    pub network: Network,
    pub symbol: String,
    pub name: String,
    pub token_address: String,
    pub decimals: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct AddConfiguredTokenRequest {
    pub network: Network,
    pub token_address: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct RemoveConfiguredTokenRequest {
    pub network: Network,
    pub token_address: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ConfiguredRpcResponse {
    pub network: Network,
    pub rpc_url: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct SetConfiguredRpcRequest {
    pub network: Network,
    pub rpc_url: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct RemoveConfiguredRpcRequest {
    pub network: Network,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ConfiguredExplorerResponse {
    pub network: Network,
    pub address_url_template: String,
    pub token_url_template: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct BalanceRequest {
    pub account: String,
    pub token: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct BalanceResponse {
    pub network: Network,
    pub account: String,
    pub token: Option<String>,
    pub amount: Option<String>,
    pub decimals: Option<u8>,
    pub block_ref: Option<String>,
    pub pending: bool,
    pub message: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug, Default)]
pub struct TransferRequest {
    pub from: Option<String>,
    pub to: String,
    pub amount: String,
    pub token: Option<String>,
    pub memo: Option<String>,
    pub nonce: Option<String>,
    pub metadata: Vec<(String, String)>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct TransferResponse {
    pub network: Network,
    pub accepted: bool,
    pub tx_id: Option<String>,
    pub signed_tx: Option<String>,
    pub signed_tx_encoding: Option<String>,
    pub broadcast_request: Option<BroadcastHttpRequest>,
    pub message: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct BroadcastHttpRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct NetworkModuleStatus {
    pub network: Network,
    pub balance_ready: bool,
    pub transfer_ready: bool,
    pub note: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct WalletNetworkInfoResponse {
    pub id: Network,
    pub primary_symbol: String,
    pub address_family: String,
    pub shared_address_group: String,
    pub supports_send: bool,
    pub supports_balance: bool,
    pub default_rpc_url: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ServiceInfoResponse {
    pub version: String,
    pub owner: Option<Principal>,
    pub paused: bool,
    pub caller: Principal,
    pub note: Option<String>,
}
