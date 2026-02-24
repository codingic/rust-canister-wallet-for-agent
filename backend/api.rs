use candid::Principal;

use crate::config;
use crate::error::{WalletError, WalletResult};
use crate::types::{
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, ConfiguredExplorerResponse,
    ConfiguredTokenResponse, NetworkModuleStatus, ServiceInfoResponse, TransferRequest,
    TransferResponse, WalletNetworkInfoResponse,
};
use crate::{
    aptos, btc, eth, evm_rpc, icp, near, sepolia, sol, solana_testnet, state, sui, ton, trx,
};

const API_VERSION: &str = "0.1.0";

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
            supports_send: info.supports_send,
            supports_balance: info.supports_balance,
            default_rpc_url: info.default_rpc_url.map(ToString::to_string),
        })
        .collect()
}

#[ic_cdk::query]
fn configured_tokens(network: String) -> Vec<ConfiguredTokenResponse> {
    let request_network = normalize_network_name_key(&network);
    config::token_list_config::configured_tokens(&network)
        .iter()
        .map(|t| ConfiguredTokenResponse {
            network: request_network.clone(),
            symbol: t.symbol.to_string(),
            name: t.name.to_string(),
            token_address: t.token_address.to_string(),
            decimals: t.decimals,
        })
        .collect()
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

fn normalize_network_name_key(input: &str) -> String {
    input.trim().to_lowercase().replace('_', "-")
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

macro_rules! balance_query {
    ($name:ident, $module:ident) => {
        #[ic_cdk::query]
        fn $name(req: BalanceRequest) -> WalletResult<BalanceResponse> {
            $module::get_balance(req)
        }
    };
}

macro_rules! balance_update {
    ($name:ident, $module:ident) => {
        #[ic_cdk::update]
        async fn $name(req: BalanceRequest) -> WalletResult<BalanceResponse> {
            $module::get_balance(req).await
        }
    };
}

macro_rules! transfer_update {
    ($name:ident, $module:ident) => {
        #[ic_cdk::update]
        fn $name(req: TransferRequest) -> WalletResult<TransferResponse> {
            ensure_not_paused()?;
            $module::transfer(req)
        }
    };
}

macro_rules! address_update {
    ($name:ident, $module:ident) => {
        #[ic_cdk::update]
        async fn $name(req: AddressRequest) -> WalletResult<AddressResponse> {
            ensure_not_paused()?;
            $module::request_address(req).await
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

address_update!(btc_request_address, btc);
address_update!(eth_request_address, eth);
address_update!(sepolia_request_address, sepolia);
address_update!(sol_request_address, sol);
address_update!(solana_testnet_request_address, solana_testnet);

balance_query!(btc_get_balance_btc, btc);
transfer_update!(btc_transfer_btc, btc);

evm_native_balance_update!(eth_get_balance_eth, "ethereum");
evm_token_balance_update!(eth_get_balance_erc20, "ethereum");
evm_native_transfer_update!(eth_transfer_eth, "ethereum");
evm_token_transfer_update!(eth_transfer_erc20, "ethereum");

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

evm_native_balance_update!(arb_get_balance_eth, "arbitrum");
evm_token_balance_update!(arb_get_balance_erc20, "arbitrum");
evm_native_transfer_update!(arb_transfer_eth, "arbitrum");
evm_token_transfer_update!(arb_transfer_erc20, "arbitrum");

evm_native_balance_update!(op_get_balance_eth, "optimism");
evm_token_balance_update!(op_get_balance_erc20, "optimism");
evm_native_transfer_update!(op_transfer_eth, "optimism");
evm_token_transfer_update!(op_transfer_erc20, "optimism");

evm_native_balance_update!(avax_get_balance_avax, "avalanche");
evm_token_balance_update!(avax_get_balance_erc20, "avalanche");
evm_native_transfer_update!(avax_transfer_avax, "avalanche");
evm_token_transfer_update!(avax_transfer_erc20, "avalanche");

evm_native_balance_update!(okb_get_balance_okb, "okx");
evm_token_balance_update!(okb_get_balance_erc20, "okx");
evm_native_transfer_update!(okb_transfer_okb, "okx");
evm_token_transfer_update!(okb_transfer_erc20, "okx");

evm_native_balance_update!(polygon_get_balance_pol, "polygon");
evm_token_balance_update!(polygon_get_balance_erc20, "polygon");
evm_native_transfer_update!(polygon_transfer_pol, "polygon");
evm_token_transfer_update!(polygon_transfer_erc20, "polygon");

balance_query!(icp_get_balance_icp, icp);
balance_query!(icp_get_balance_icrc, icp);
transfer_update!(icp_transfer_icp, icp);
transfer_update!(icp_transfer_icrc, icp);

balance_update!(sol_get_balance_sol, sol);
balance_update!(sol_get_balance_spl, sol);
#[ic_cdk::update]
async fn sol_transfer_sol(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    sol::transfer_sol(req).await
}
#[ic_cdk::update]
async fn sol_transfer_spl(req: TransferRequest) -> WalletResult<TransferResponse> {
    ensure_not_paused()?;
    sol::transfer_spl(req).await
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

balance_query!(trx_get_balance_trx, trx);
balance_query!(trx_get_balance_trc20, trx);
transfer_update!(trx_transfer_trx, trx);
transfer_update!(trx_transfer_trc20, trx);

balance_query!(ton_get_balance_ton, ton);
balance_query!(ton_get_balance_jetton, ton);
transfer_update!(ton_transfer_ton, ton);
transfer_update!(ton_transfer_jetton, ton);

balance_query!(near_get_balance_near, near);
balance_query!(near_get_balance_nep141, near);
transfer_update!(near_transfer_near, near);
transfer_update!(near_transfer_nep141, near);

balance_query!(aptos_get_balance_apt, aptos);
balance_query!(aptos_get_balance_token, aptos);
transfer_update!(aptos_transfer_apt, aptos);
transfer_update!(aptos_transfer_token, aptos);

balance_query!(sui_get_balance_sui, sui);
balance_query!(sui_get_balance_token, sui);
transfer_update!(sui_transfer_sui, sui);
transfer_update!(sui_transfer_token, sui);

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
}
