use candid::Principal;

use crate::error::{WalletError, WalletResult};
use crate::types::{
    AddressRequest, AddressResponse, BalanceRequest, BalanceResponse, NetworkModuleStatus,
    ServiceInfoResponse, TransferRequest, TransferResponse,
};
use crate::{
    aptos, arb, avax, base, bsc, btc, eth, icp, near, okb, op, polygon, sol, state, sui, ton,
    trx, types,
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
    types::default_network_statuses()
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

address_update!(btc_request_address, btc);
address_update!(eth_request_address, eth);
address_update!(sol_request_address, sol);

balance_query!(btc_get_balance_btc, btc);
transfer_update!(btc_transfer_btc, btc);

balance_query!(eth_get_balance_eth, eth);
balance_query!(eth_get_balance_erc20, eth);
transfer_update!(eth_transfer_eth, eth);
transfer_update!(eth_transfer_erc20, eth);

balance_query!(base_get_balance_eth, base);
balance_query!(base_get_balance_erc20, base);
transfer_update!(base_transfer_eth, base);
transfer_update!(base_transfer_erc20, base);

balance_query!(bsc_get_balance_bnb, bsc);
balance_query!(bsc_get_balance_bep20, bsc);
transfer_update!(bsc_transfer_bnb, bsc);
transfer_update!(bsc_transfer_bep20, bsc);

balance_query!(arb_get_balance_eth, arb);
balance_query!(arb_get_balance_erc20, arb);
transfer_update!(arb_transfer_eth, arb);
transfer_update!(arb_transfer_erc20, arb);

balance_query!(op_get_balance_eth, op);
balance_query!(op_get_balance_erc20, op);
transfer_update!(op_transfer_eth, op);
transfer_update!(op_transfer_erc20, op);

balance_query!(avax_get_balance_avax, avax);
balance_query!(avax_get_balance_erc20, avax);
transfer_update!(avax_transfer_avax, avax);
transfer_update!(avax_transfer_erc20, avax);

balance_query!(okb_get_balance_okb, okb);
balance_query!(okb_get_balance_erc20, okb);
transfer_update!(okb_transfer_okb, okb);
transfer_update!(okb_transfer_erc20, okb);

balance_query!(polygon_get_balance_pol, polygon);
balance_query!(polygon_get_balance_erc20, polygon);
transfer_update!(polygon_transfer_pol, polygon);
transfer_update!(polygon_transfer_erc20, polygon);

balance_query!(icp_get_balance_icp, icp);
balance_query!(icp_get_balance_icrc, icp);
transfer_update!(icp_transfer_icp, icp);
transfer_update!(icp_transfer_icrc, icp);

balance_query!(sol_get_balance_sol, sol);
balance_query!(sol_get_balance_spl, sol);
transfer_update!(sol_transfer_sol, sol);
transfer_update!(sol_transfer_spl, sol);

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
