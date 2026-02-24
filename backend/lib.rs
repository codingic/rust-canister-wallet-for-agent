mod addressing;
mod api;
mod aptos;
mod arb;
mod avax;
mod base;
mod bsc;
mod btc;
#[allow(dead_code)]
mod config;
mod error;
mod eth;
mod evm_rpc;
mod icp;
mod near;
mod okb;
mod op;
mod polygon;
mod sepolia;
mod sol;
mod solana_testnet;
mod state;
mod sui;
mod ton;
mod trx;
mod types;

// Keep these in scope for `export_candid!()` type resolution after moving endpoints to `api.rs`.
#[allow(unused_imports)]
use candid::Principal;
#[allow(unused_imports)]
use error::WalletResult;
#[allow(unused_imports)]
use types::{
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, ConfiguredExplorerResponse,
    ConfiguredTokenResponse, NetworkModuleStatus, ServiceInfoResponse, TransferRequest,
    TransferResponse, WalletNetworkInfoResponse,
};

ic_cdk::export_candid!();
