use candid::Principal;

use crate::chains::{
    aptos_mainnet, bitcoin, ethereum, internet_computer, near_mainnet, sepolia, solana,
    solana_testnet, sui_mainnet, ton_mainnet, tron,
};
use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    AddConfiguredTokenRequest, AddressResponse, BalanceRequest, BalanceResponse,
    ConfiguredExplorerResponse, ConfiguredRpcResponse, ConfiguredTokenResponse,
    NetworkModuleStatus, RemoveConfiguredRpcRequest, RemoveConfiguredTokenRequest,
    ServiceInfoResponse, SetConfiguredRpcRequest, TransferRequest, TransferResponse,
    WalletNetworkInfoResponse,
};
use crate::{evm_rpc, state, token_registry};

const API_VERSION: &str = "0.1.0";

fn bootstrap_runtime_config_from_static() {
    let default_rpcs = config::rpc_config::wallet_networks()
        .into_iter()
        .filter_map(|info| {
            info.default_rpc_url.map(|rpc_url| ConfiguredRpcResponse {
                network: config::rpc_config::normalize_network(info.id),
                rpc_url: rpc_url.to_string(),
            })
        })
        .collect();
    state::seed_missing_configured_rpcs(default_rpcs);

    let mut builtin_tokens: Vec<ConfiguredTokenResponse> = Vec::new();
    for info in config::rpc_config::wallet_networks() {
        let network_key = normalize_network_name_key(info.id);
        for token in config::token_list_config::configured_tokens(info.id) {
            builtin_tokens.push(ConfiguredTokenResponse {
                network: network_key.clone(),
                symbol: token.symbol.to_string(),
                name: token.name.to_string(),
                token_address: token.token_address.to_string(),
                decimals: token.decimals,
            });
        }
    }
    state::set_builtin_tokens(builtin_tokens);
}

fn ensure_not_paused() -> WalletResult<()> {
    if state::is_paused() {
        return Err(WalletError::Paused);
    }
    Ok(())
}

fn require_owner_placeholder() -> WalletResult<()> {
    // TODO(auth): enforce msg_caller() == owner after the function layer is stable.
    let _ = ic_cdk::api::msg_caller();
    Ok(())
}

#[ic_cdk::init]
fn init() {
    state::restore(state::State::default());
    bootstrap_runtime_config_from_static();
}

#[ic_cdk::query]
fn whoami() -> Principal {
    ic_cdk::api::msg_caller()
}

#[ic_cdk::query]
fn get_owner() -> Option<Principal> {
    state::owner()
}

#[ic_cdk::query]
fn is_paused() -> bool {
    state::is_paused()
}

#[ic_cdk::query]
fn service_info() -> ServiceInfoResponse {
    ServiceInfoResponse {
        version: API_VERSION.to_string(),
        owner: state::owner(),
        paused: state::is_paused(),
        caller: ic_cdk::api::msg_caller(),
        note: Some(
            "Auth is placeholder for now; network modules return scaffold responses.".into(),
        ),
    }
}

#[ic_cdk::query]
fn supported_networks() -> Vec<NetworkModuleStatus> {
    let note = Some("Scaffold only. Real on-chain logic will be implemented later.".to_string());
    config::rpc_config::wallet_networks()
        .into_iter()
        .map(|info| NetworkModuleStatus {
            network: info.id.to_string(),
            balance_ready: false,
            transfer_ready: false,
            note: note.clone(),
        })
        .collect()
}

#[ic_cdk::query]
fn wallet_networks() -> Vec<WalletNetworkInfoResponse> {
    config::rpc_config::wallet_networks()
        .into_iter()
        .map(|info| WalletNetworkInfoResponse {
            id: info.id.to_string(),
            primary_symbol: info.primary_symbol.to_string(),
            address_family: info.address_family.to_string(),
            shared_address_group: info.shared_address_group.to_string(),
            supports_send: info.supports_send,
            supports_balance: info.supports_balance,
            default_rpc_url: config::rpc_config::configured_rpc(info.id).map(|c| c.rpc_url),
        })
        .collect()
}

#[ic_cdk::query]
fn configured_rpcs() -> Vec<ConfiguredRpcResponse> {
    state::configured_rpcs()
}

#[ic_cdk::query]
fn configured_tokens(network: String) -> Vec<ConfiguredTokenResponse> {
    let request_network = normalize_network_name_key(&network);
    let mut merged: Vec<ConfiguredTokenResponse> =
        state::builtin_tokens_for_network(&request_network)
            .into_iter()
            .filter(|t| !state::is_removed_token(&request_network, &t.token_address))
            .collect();

    let custom = state::custom_tokens_for_network(&request_network);
    for token in custom {
        if let Some(existing) = merged
            .iter_mut()
            .find(|t| t.token_address == token.token_address)
        {
            *existing = token;
        } else {
            merged.push(token);
        }
    }
    merged
}

#[ic_cdk::query]
fn configured_explorer(network: String) -> Option<ConfiguredExplorerResponse> {
    let request_network = normalize_network_name_key(&network);
    config::explorer_config::configured_explorer(&network).map(|c| ConfiguredExplorerResponse {
        network: request_network,
        address_url_template: c.address_url_template.to_string(),
        token_url_template: c.token_url_template.map(ToString::to_string),
    })
}

#[ic_cdk::update]
async fn add_configured_token(
    req: AddConfiguredTokenRequest,
) -> WalletResult<ConfiguredTokenResponse> {
    require_owner_placeholder()?;
    ensure_not_paused()?;
    let normalized_network = normalize_network_name_key(&req.network);
    let mut discovered =
        token_registry::discover_token_metadata(&normalized_network, &req.token_address).await?;
    discovered.network = normalized_network;
    state::upsert_custom_token(discovered.clone());
    Ok(discovered)
}

#[ic_cdk::update]
fn remove_configured_token(req: RemoveConfiguredTokenRequest) -> WalletResult<bool> {
    require_owner_placeholder()?;
    let network = normalize_network_name_key(&req.network);
    let token_address = req.token_address.trim();
    if token_address.is_empty() {
        return Err(WalletError::invalid_input("token_address is required"));
    }
    Ok(state::remove_token(&network, token_address))
}

#[ic_cdk::update]
fn set_configured_rpc(req: SetConfiguredRpcRequest) -> WalletResult<ConfiguredRpcResponse> {
    require_owner_placeholder()?;
    let network = config::rpc_config::normalize_network(&req.network);
    if network.is_empty() {
        return Err(WalletError::invalid_input("network is required"));
    }
    if config::rpc_config::wallet_network_info(&network).is_none() {
        return Err(WalletError::invalid_input("unsupported network"));
    }
    let rpc_url = req.rpc_url.trim();
    if rpc_url.is_empty() {
        return Err(WalletError::invalid_input("rpc_url is required"));
    }
    state::upsert_configured_rpc(&network, rpc_url);
    Ok(ConfiguredRpcResponse {
        network,
        rpc_url: rpc_url.to_string(),
    })
}

#[ic_cdk::update]
fn remove_configured_rpc(req: RemoveConfiguredRpcRequest) -> WalletResult<bool> {
    require_owner_placeholder()?;
    let network = config::rpc_config::normalize_network(&req.network);
    if network.is_empty() {
        return Err(WalletError::invalid_input("network is required"));
    }
    Ok(state::remove_configured_rpc(&network))
}

fn normalize_network_name_key(input: &str) -> String {
    token_registry::normalize_network_name(input)
}

#[ic_cdk::update]
fn rotate_owner(new_owner: Principal) -> WalletResult<Option<Principal>> {
    require_owner_placeholder()?;
    if new_owner == Principal::anonymous() {
        return Err(WalletError::invalid_input("new_owner cannot be anonymous"));
    }
    Ok(state::rotate_owner(new_owner))
}

#[ic_cdk::update]
fn pause() -> WalletResult<()> {
    require_owner_placeholder()?;
    state::set_paused(true);
    Ok(())
}

#[ic_cdk::update]
fn unpause() -> WalletResult<()> {
    require_owner_placeholder()?;
    state::set_paused(false);
    Ok(())
}

macro_rules! balance_update {
    ($name:ident, $module:ident) => {
        #[ic_cdk::update]
        async fn $name(req: BalanceRequest) -> WalletResult<BalanceResponse> {
            $module::get_balance(req).await
        }
    };
}

macro_rules! address_update {
    ($name:ident, $module:ident) => {
        #[ic_cdk::update]
        async fn $name() -> WalletResult<AddressResponse> {
            ensure_not_paused()?;
            $module::request_address().await
        }
    };
}

macro_rules! evm_native_balance_update {
    ($name:ident, $network:literal) => {
        #[ic_cdk::update]
        async fn $name(req: BalanceRequest) -> WalletResult<BalanceResponse> {
            if req.token.is_some() {
                return Err(WalletError::invalid_input(concat!(
                    stringify!($name),
                    " does not accept token parameter"
                )));
            }
            evm_rpc::get_native_eth_balance($network, req).await
        }
    };
}

macro_rules! evm_token_balance_update {
    ($name:ident, $network:literal) => {
        #[ic_cdk::update]
        async fn $name(req: BalanceRequest) -> WalletResult<BalanceResponse> {
            evm_rpc::get_erc20_balance($network, req).await
        }
    };
}

macro_rules! evm_native_transfer_update {
    ($name:ident, $network:literal) => {
        #[ic_cdk::update]
        async fn $name(req: TransferRequest) -> WalletResult<TransferResponse> {
            ensure_not_paused()?;
            evm_rpc::transfer_native_eth($network, req).await
        }
    };
}

macro_rules! evm_token_transfer_update {
    ($name:ident, $network:literal) => {
        #[ic_cdk::update]
        async fn $name(req: TransferRequest) -> WalletResult<TransferResponse> {
            ensure_not_paused()?;
            evm_rpc::transfer_erc20($network, req).await
        }
    };
}

address_update!(bitcoin_request_address, bitcoin);
address_update!(ethereum_request_address, ethereum);
address_update!(base_request_address, ethereum);
address_update!(bsc_request_address, ethereum);
address_update!(arbitrum_request_address, ethereum);
address_update!(optimism_request_address, ethereum);
address_update!(avalanche_request_address, ethereum);
address_update!(okx_request_address, ethereum);
address_update!(polygon_request_address, ethereum);
address_update!(sepolia_request_address, sepolia);
address_update!(solana_request_address, solana);
address_update!(solana_testnet_request_address, solana_testnet);
address_update!(tron_request_address, tron);
address_update!(ton_mainnet_request_address, ton_mainnet);
address_update!(near_mainnet_request_address, near_mainnet);
address_update!(aptos_mainnet_request_address, aptos_mainnet);
address_update!(sui_mainnet_request_address, sui_mainnet);

#[ic_cdk::update]
async fn bitcoin_get_balance_btc(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    bitcoin::get_balance(req).await
}

#[ic_cdk::update]
async fn bitcoin_transfer_btc(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    bitcoin::transfer(req).await
}

evm_native_balance_update!(ethereum_get_balance_eth, "ethereum");
evm_token_balance_update!(ethereum_get_balance_erc20, "ethereum");
evm_native_transfer_update!(ethereum_transfer_eth, "ethereum");
evm_token_transfer_update!(ethereum_transfer_erc20, "ethereum");

evm_native_balance_update!(sepolia_get_balance_eth, "sepolia");
evm_token_balance_update!(sepolia_get_balance_erc20, "sepolia");
evm_native_transfer_update!(sepolia_transfer_eth, "sepolia");
evm_token_transfer_update!(sepolia_transfer_erc20, "sepolia");

evm_native_balance_update!(base_get_balance_eth, "base");
evm_token_balance_update!(base_get_balance_erc20, "base");
evm_native_transfer_update!(base_transfer_eth, "base");
evm_token_transfer_update!(base_transfer_erc20, "base");

evm_native_balance_update!(bsc_get_balance_bnb, "bsc");
evm_token_balance_update!(bsc_get_balance_bep20, "bsc");
evm_native_transfer_update!(bsc_transfer_bnb, "bsc");
evm_token_transfer_update!(bsc_transfer_bep20, "bsc");

evm_native_balance_update!(arbitrum_get_balance_eth, "arbitrum");
evm_token_balance_update!(arbitrum_get_balance_erc20, "arbitrum");
evm_native_transfer_update!(arbitrum_transfer_eth, "arbitrum");
evm_token_transfer_update!(arbitrum_transfer_erc20, "arbitrum");

evm_native_balance_update!(optimism_get_balance_eth, "optimism");
evm_token_balance_update!(optimism_get_balance_erc20, "optimism");
evm_native_transfer_update!(optimism_transfer_eth, "optimism");
evm_token_transfer_update!(optimism_transfer_erc20, "optimism");

evm_native_balance_update!(avalanche_get_balance_avax, "avalanche");
evm_token_balance_update!(avalanche_get_balance_erc20, "avalanche");
evm_native_transfer_update!(avalanche_transfer_avax, "avalanche");
evm_token_transfer_update!(avalanche_transfer_erc20, "avalanche");

evm_native_balance_update!(okx_get_balance_okb, "okx");
evm_token_balance_update!(okx_get_balance_erc20, "okx");
evm_native_transfer_update!(okx_transfer_okb, "okx");
evm_token_transfer_update!(okx_transfer_erc20, "okx");

evm_native_balance_update!(polygon_get_balance_pol, "polygon");
evm_token_balance_update!(polygon_get_balance_erc20, "polygon");
evm_native_transfer_update!(polygon_transfer_pol, "polygon");
evm_token_transfer_update!(polygon_transfer_erc20, "polygon");

#[ic_cdk::query(composite = true)]
async fn internet_computer_get_balance_icp(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    internet_computer::get_balance_icp(req).await
}

#[ic_cdk::query(composite = true)]
async fn internet_computer_get_balance_icrc(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    internet_computer::get_balance_icrc(req).await
}

#[ic_cdk::update]
async fn internet_computer_transfer_icp(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    internet_computer::transfer_icp(req).await
}

#[ic_cdk::update]
async fn internet_computer_transfer_icrc(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    internet_computer::transfer_icrc(req).await
}

balance_update!(solana_get_balance_sol, solana);
balance_update!(solana_get_balance_spl, solana);
#[ic_cdk::update]
async fn solana_transfer_sol(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    solana::transfer_sol(req).await
}
#[ic_cdk::update]
async fn solana_transfer_spl(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    solana::transfer_spl(req).await
}

balance_update!(solana_testnet_get_balance_sol, solana_testnet);
balance_update!(solana_testnet_get_balance_spl, solana_testnet);
#[ic_cdk::update]
async fn solana_testnet_transfer_sol(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    solana_testnet::transfer_sol(req).await
}
#[ic_cdk::update]
async fn solana_testnet_transfer_spl(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    solana_testnet::transfer_spl(req).await
}

#[ic_cdk::update]
async fn tron_get_balance_trx(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    tron::get_balance(req).await
}

#[ic_cdk::update]
async fn tron_get_balance_trc20(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    tron::get_balance(req).await
}

#[ic_cdk::update]
async fn tron_transfer_trx(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    tron::transfer(req).await
}

#[ic_cdk::update]
async fn tron_transfer_trc20(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    tron::transfer(req).await
}

#[ic_cdk::update]
async fn ton_mainnet_get_balance_ton(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    ton_mainnet::get_balance(req).await
}

#[ic_cdk::update]
async fn ton_mainnet_get_balance_jetton(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    ton_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn ton_mainnet_transfer_ton(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    ton_mainnet::transfer(req).await
}

#[ic_cdk::update]
async fn ton_mainnet_transfer_jetton(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    ton_mainnet::transfer(req).await
}

#[ic_cdk::update]
async fn near_mainnet_get_balance_near(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    near_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn near_mainnet_get_balance_nep141(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    near_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn near_mainnet_transfer_near(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    near_mainnet::transfer(req).await
}
#[ic_cdk::update]
async fn near_mainnet_transfer_nep141(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    near_mainnet::transfer(req).await
}

#[ic_cdk::update]
async fn aptos_mainnet_get_balance_apt(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    aptos_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn aptos_mainnet_get_balance_token(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    aptos_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn aptos_mainnet_transfer_apt(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    aptos_mainnet::transfer(req).await
}
#[ic_cdk::update]
async fn aptos_mainnet_transfer_token(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    aptos_mainnet::transfer(req).await
}

#[ic_cdk::update]
async fn sui_mainnet_get_balance_sui(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    sui_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn sui_mainnet_get_balance_token(req: BalanceRequest) -> WalletResult<BalanceResponse> {
    sui_mainnet::get_balance(req).await
}
#[ic_cdk::update]
async fn sui_mainnet_transfer_sui(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    sui_mainnet::transfer(req).await
}
#[ic_cdk::update]
async fn sui_mainnet_transfer_token(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    sui_mainnet::transfer(req).await
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let snapshot = state::snapshot();
    if let Err(err) = ic_cdk::storage::stable_save((snapshot,)) {
        ic_cdk::trap(&format!("stable_save failed: {err}"));
    }
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    match ic_cdk::storage::stable_restore::<(state::State,)>() {
        Ok((snapshot,)) => state::restore(snapshot),
        Err(_) => state::restore(state::State::default()),
    }
    bootstrap_runtime_config_from_static();
}
