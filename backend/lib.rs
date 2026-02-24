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
mod types;

// Keep these in scope for `export_candid!()` type resolution after moving endpoints to `api.rs`.
#[allow(unused_imports)]
use candid::Principal;
#[allow(unused_imports)]
use error::WalletResult;
#[allow(unused_imports)]
use types::{
    AddressResponse, BalanceRequest, BalanceResponse, ConfiguredExplorerResponse,
    ConfiguredTokenResponse, NetworkModuleStatus, ServiceInfoResponse, TransferRequest,
    TransferResponse, WalletNetworkInfoResponse,
};

ic_cdk::export_candid!();
