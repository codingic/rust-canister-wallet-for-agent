mod addressing;
mod api;
mod chains;
#[allow(dead_code)]
mod config;
mod error;
mod evm_rpc;
mod outcall;
mod sdk;
mod state;
mod token_registry;
mod types;

// Keep these in scope for `export_candid!()` type resolution after moving endpoints to `api.rs`.
#[allow(unused_imports)]
use candid::Principal;
#[allow(unused_imports)]
use error::WalletResult;
#[allow(unused_imports)]
use types::{
    AddConfiguredTokenRequest, AddressResponse, BalanceRequest, BalanceResponse,
    BroadcastHttpRequest, ConfiguredExplorerResponse, ConfiguredRpcResponse,
    ConfiguredTokenResponse, NetworkModuleStatus, RemoveConfiguredRpcRequest,
    RemoveConfiguredTokenRequest, ServiceInfoResponse, SetConfiguredRpcRequest, TransferRequest,
    TransferResponse, WalletNetworkInfoResponse,
};

ic_cdk::export_candid!();
