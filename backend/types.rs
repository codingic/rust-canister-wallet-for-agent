use candid::{CandidType, Principal};
use serde::Deserialize;

pub type Network = String;

#[derive(CandidType, Deserialize, Clone, Debug, Default)]
pub struct AddressRequest {
    pub index: Option<u32>,
    pub account_tag: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct AddressResponse {
    pub network: Network,
    pub address: String,
    pub public_key_hex: String,
    pub key_name: String,
    pub index: u32,
    pub account_tag: Option<String>,
    pub message: Option<String>,
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
    pub message: String,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct NetworkModuleStatus {
    pub network: Network,
    pub balance_ready: bool,
    pub transfer_ready: bool,
    pub note: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ServiceInfoResponse {
    pub version: String,
    pub owner: Option<Principal>,
    pub paused: bool,
    pub caller: Principal,
    pub note: Option<String>,
}

pub fn default_network_statuses() -> Vec<NetworkModuleStatus> {
    let note = Some("Scaffold only. Real on-chain logic will be implemented later.".to_string());
    vec![
        NetworkModuleStatus {
            network: "btc".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "eth".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "base".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "bsc".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "arb".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "op".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "avax".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "okb".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "polygon".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "icp".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "sol".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "trx".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "ton".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "near".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "aptos".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        },
        NetworkModuleStatus {
            network: "sui".to_string(),
            balance_ready: false,
            transfer_ready: false,
            note,
        },
    ]
}
