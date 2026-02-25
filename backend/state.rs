use candid::{CandidType, Principal};
use serde::Deserialize;
use std::cell::RefCell;

use crate::types::{ConfiguredRpcResponse, ConfiguredTokenResponse};

#[derive(CandidType, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenKey {
    pub network: String,
    pub token_address: String,
}

#[derive(CandidType, Deserialize, Clone, Debug, Default)]
pub struct State {
    pub owner: Option<Principal>,
    pub paused: bool,
    #[serde(default)]
    pub builtin_tokens: Vec<ConfiguredTokenResponse>,
    #[serde(default)]
    pub custom_tokens: Vec<ConfiguredTokenResponse>,
    #[serde(default)]
    pub removed_tokens: Vec<TokenKey>,
    #[serde(default)]
    pub runtime_rpcs: Vec<ConfiguredRpcResponse>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[allow(dead_code)]
pub fn init_owner(owner: Principal) {
    STATE.with(|state| {
        state.borrow_mut().owner = Some(owner);
    });
}

pub fn owner() -> Option<Principal> {
    STATE.with(|state| state.borrow().owner)
}

pub fn rotate_owner(new_owner: Principal) -> Option<Principal> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let prev = state.owner;
        state.owner = Some(new_owner);
        prev
    })
}

pub fn is_paused() -> bool {
    STATE.with(|state| state.borrow().paused)
}

pub fn set_paused(paused: bool) {
    STATE.with(|state| {
        state.borrow_mut().paused = paused;
    });
}

pub fn snapshot() -> State {
    STATE.with(|state| state.borrow().clone())
}

pub fn restore(snapshot: State) {
    STATE.with(|state| {
        *state.borrow_mut() = snapshot;
    });
}

pub fn custom_tokens_for_network(network: &str) -> Vec<ConfiguredTokenResponse> {
    STATE.with(|state| {
        state
            .borrow()
            .custom_tokens
            .iter()
            .filter(|t| t.network == network)
            .cloned()
            .collect()
    })
}

pub fn builtin_tokens_for_network(network: &str) -> Vec<ConfiguredTokenResponse> {
    STATE.with(|state| {
        state
            .borrow()
            .builtin_tokens
            .iter()
            .filter(|t| t.network == network)
            .cloned()
            .collect()
    })
}

pub fn set_builtin_tokens(tokens: Vec<ConfiguredTokenResponse>) {
    STATE.with(|state| {
        state.borrow_mut().builtin_tokens = tokens;
    });
}

pub fn upsert_custom_token(token: ConfiguredTokenResponse) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let mut replaced = false;
        if let Some(existing) = state
            .custom_tokens
            .iter_mut()
            .find(|t| t.network == token.network && t.token_address == token.token_address)
        {
            *existing = token.clone();
            replaced = true;
        } else {
            state.custom_tokens.push(token.clone());
        }
        state
            .removed_tokens
            .retain(|k| !(k.network == token.network && k.token_address == token.token_address));
        replaced
    })
}

pub fn remove_token(network: &str, token_address: &str) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let before_len = state.custom_tokens.len();
        state
            .custom_tokens
            .retain(|t| !(t.network == network && t.token_address == token_address));
        let custom_removed = state.custom_tokens.len() != before_len;
        let key = TokenKey {
            network: network.to_string(),
            token_address: token_address.to_string(),
        };
        let already_tombstoned = state.removed_tokens.iter().any(|k| k == &key);
        if !already_tombstoned {
            state.removed_tokens.push(key);
        }
        custom_removed || !already_tombstoned
    })
}

pub fn is_removed_token(network: &str, token_address: &str) -> bool {
    STATE.with(|state| {
        state
            .borrow()
            .removed_tokens
            .iter()
            .any(|k| k.network == network && k.token_address == token_address)
    })
}

pub fn configured_rpcs() -> Vec<ConfiguredRpcResponse> {
    STATE.with(|state| {
        let mut items = state.borrow().runtime_rpcs.clone();
        items.sort_by(|a, b| a.network.cmp(&b.network));
        items
    })
}

pub fn configured_rpc(network: &str) -> Option<String> {
    STATE.with(|state| {
        state
            .borrow()
            .runtime_rpcs
            .iter()
            .find(|r| r.network == network)
            .map(|r| r.rpc_url.clone())
    })
}

pub fn upsert_configured_rpc(network: &str, rpc_url: &str) -> bool {
    let network = network.trim();
    let rpc_url = rpc_url.trim();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(existing) = state.runtime_rpcs.iter_mut().find(|r| r.network == network) {
            let replaced = existing.rpc_url != rpc_url;
            existing.rpc_url = rpc_url.to_string();
            replaced
        } else {
            state.runtime_rpcs.push(ConfiguredRpcResponse {
                network: network.to_string(),
                rpc_url: rpc_url.to_string(),
            });
            false
        }
    })
}

pub fn remove_configured_rpc(network: &str) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let before = state.runtime_rpcs.len();
        state.runtime_rpcs.retain(|r| r.network != network);
        state.runtime_rpcs.len() != before
    })
}

pub fn seed_missing_configured_rpcs(defaults: Vec<ConfiguredRpcResponse>) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        for default in defaults {
            let exists = state
                .runtime_rpcs
                .iter()
                .any(|r| r.network == default.network);
            if !exists {
                state.runtime_rpcs.push(default);
            }
        }
    });
}
