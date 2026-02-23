mod addressing;
mod api;
mod avax;
mod arb;
mod aptos;
mod bsc;
mod base;
mod btc;
#[allow(dead_code)]
mod config;
mod error;
mod eth;
mod icp;
mod near;
mod okb;
mod op;
mod polygon;
mod sol;
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
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, NetworkModuleStatus,
    ServiceInfoResponse, TransferRequest, TransferResponse,
};

ic_cdk::export_candid!();
