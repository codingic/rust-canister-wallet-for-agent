use std::cell::RefCell;
use std::collections::BTreeMap;

use candid::{CandidType, Deserialize, Principal};
use ic_cdk::management_canister::{self, CanisterSettings, CreateCanisterArgs};

const API_VERSION: &str = "0.1.0";
const DEFAULT_EXTRA_CYCLES: u64 = 500_000_000_000;
const DEFAULT_MAX_EXTRA_CYCLES_PER_CREATE: u64 = 5_000_000_000_000;
const DEFAULT_MAX_CANISTERS_PER_CALLER: u32 = 3;

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CreateCanisterForCallerRequest {
    pub cycles: Option<u64>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CreateCanisterForCallerError {
    Internal(String),
    QuotaExceeded(String),
    Forbidden(String),
    InvalidInput(String),
}

pub type CreateCanisterForCallerResult = Result<Principal, CreateCanisterForCallerError>;

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ServiceInfoResponse {
    pub version: String,
    pub caller: Principal,
    pub owner: Option<Principal>,
    pub paused: bool,
    pub public_create_enabled: bool,
    pub default_extra_cycles: u64,
    pub max_extra_cycles_per_create: u64,
    pub max_canisters_per_caller: u32,
    pub total_created: u64,
    pub caller_created_count: u32,
    pub last_created_canister: Option<Principal>,
    pub factory_cycles_balance: u128,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
struct State {
    owner: Option<Principal>,
    paused: bool,
    public_create_enabled: bool,
    default_extra_cycles: u64,
    max_extra_cycles_per_create: u64,
    max_canisters_per_caller: u32,
    total_created: u64,
    created_by_caller: BTreeMap<Principal, u32>,
    last_created_canister: Option<Principal>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: None,
            paused: false,
            public_create_enabled: true,
            default_extra_cycles: DEFAULT_EXTRA_CYCLES,
            max_extra_cycles_per_create: DEFAULT_MAX_EXTRA_CYCLES_PER_CREATE,
            max_canisters_per_caller: DEFAULT_MAX_CANISTERS_PER_CALLER,
            total_created: 0,
            created_by_caller: BTreeMap::new(),
            last_created_canister: None,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn state_read<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with(|s| f(&s.borrow()))
}

fn state_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn err_internal(msg: impl Into<String>) -> CreateCanisterForCallerError {
    CreateCanisterForCallerError::Internal(msg.into())
}

fn err_forbidden(msg: impl Into<String>) -> CreateCanisterForCallerError {
    CreateCanisterForCallerError::Forbidden(msg.into())
}

fn err_invalid(msg: impl Into<String>) -> CreateCanisterForCallerError {
    CreateCanisterForCallerError::InvalidInput(msg.into())
}

fn err_quota(msg: impl Into<String>) -> CreateCanisterForCallerError {
    CreateCanisterForCallerError::QuotaExceeded(msg.into())
}

fn caller() -> Principal {
    ic_cdk::api::msg_caller()
}

fn require_owner() -> Result<Principal, CreateCanisterForCallerError> {
    let c = caller();
    if c == Principal::anonymous() {
        return Err(err_forbidden("anonymous caller is not allowed"));
    }
    let owner = state_read(|s| s.owner);
    match owner {
        Some(o) if o == c => Ok(c),
        Some(_) => Err(err_forbidden("owner only")),
        None => Err(err_forbidden(
            "owner is not initialized; bootstrap with rotate_owner(caller)",
        )),
    }
}

fn can_bootstrap_owner(new_owner: Principal) -> bool {
    if new_owner == Principal::anonymous() {
        return false;
    }
    let c = caller();
    c != Principal::anonymous() && c == new_owner && state_read(|s| s.owner.is_none())
}

fn normalize_cycles(requested: Option<u64>) -> Result<u64, CreateCanisterForCallerError> {
    let (cycles, max_cycles) = state_read(|s| {
        (
            requested.unwrap_or(s.default_extra_cycles),
            s.max_extra_cycles_per_create,
        )
    });
    if cycles > max_cycles {
        return Err(err_quota(format!(
            "requested cycles {} exceeds max {}",
            cycles, max_cycles
        )));
    }
    Ok(cycles)
}

fn ensure_create_allowed(c: Principal) -> Result<(), CreateCanisterForCallerError> {
    if c == Principal::anonymous() {
        return Err(err_forbidden("anonymous caller is not allowed"));
    }
    let (paused, public_create_enabled, owner, max_per_caller, count) = state_read(|s| {
        (
            s.paused,
            s.public_create_enabled,
            s.owner,
            s.max_canisters_per_caller,
            s.created_by_caller.get(&c).copied().unwrap_or(0),
        )
    });
    if paused {
        return Err(err_forbidden("factory is paused"));
    }
    if !public_create_enabled && owner != Some(c) {
        return Err(err_forbidden("public create is disabled"));
    }
    if count >= max_per_caller {
        return Err(err_quota(format!(
            "caller create quota exceeded: {count}/{max_per_caller}"
        )));
    }
    Ok(())
}

fn record_created(c: Principal, canister_id: Principal) {
    state_mut(|s| {
        s.total_created = s.total_created.saturating_add(1);
        let entry = s.created_by_caller.entry(c).or_insert(0);
        *entry = entry.saturating_add(1);
        s.last_created_canister = Some(canister_id);
    });
}

#[ic_cdk::init]
fn init() {
    state_mut(|s| *s = State::default());
}

#[ic_cdk::query]
fn whoami() -> Principal {
    caller()
}

#[ic_cdk::query]
fn get_owner() -> Option<Principal> {
    state_read(|s| s.owner)
}

#[ic_cdk::query]
fn service_info() -> ServiceInfoResponse {
    let c = caller();
    state_read(|s| ServiceInfoResponse {
        version: API_VERSION.to_string(),
        caller: c,
        owner: s.owner,
        paused: s.paused,
        public_create_enabled: s.public_create_enabled,
        default_extra_cycles: s.default_extra_cycles,
        max_extra_cycles_per_create: s.max_extra_cycles_per_create,
        max_canisters_per_caller: s.max_canisters_per_caller,
        total_created: s.total_created,
        caller_created_count: s.created_by_caller.get(&c).copied().unwrap_or(0),
        last_created_canister: s.last_created_canister,
        factory_cycles_balance: ic_cdk::api::canister_cycle_balance(),
    })
}

#[ic_cdk::query]
fn my_created_count() -> u32 {
    let c = caller();
    state_read(|s| s.created_by_caller.get(&c).copied().unwrap_or(0))
}

#[ic_cdk::update]
async fn create_canister_for_caller(
    req: CreateCanisterForCallerRequest,
) -> CreateCanisterForCallerResult {
    let c = caller();
    ensure_create_allowed(c)?;
    let extra_cycles = normalize_cycles(req.cycles)?;

    let args = CreateCanisterArgs {
        settings: Some(CanisterSettings {
            controllers: Some(vec![c]),
            ..Default::default()
        }),
    };

    let result =
        management_canister::create_canister_with_extra_cycles(&args, u128::from(extra_cycles))
            .await
            .map_err(|err| err_internal(format!("create_canister failed: {err}")))?;

    let canister_id = result.canister_id;
    record_created(c, canister_id);
    Ok(canister_id)
}

#[ic_cdk::update]
fn rotate_owner(new_owner: Principal) -> Result<Option<Principal>, CreateCanisterForCallerError> {
    if new_owner == Principal::anonymous() {
        return Err(err_invalid("new_owner cannot be anonymous"));
    }
    if !can_bootstrap_owner(new_owner) {
        let _ = require_owner()?;
    }
    Ok(state_mut(|s| s.owner.replace(new_owner)))
}

#[ic_cdk::update]
fn set_paused(paused: bool) -> Result<bool, CreateCanisterForCallerError> {
    let _ = require_owner()?;
    state_mut(|s| {
        s.paused = paused;
    });
    Ok(paused)
}

#[ic_cdk::update]
fn set_public_create_enabled(enabled: bool) -> Result<bool, CreateCanisterForCallerError> {
    let _ = require_owner()?;
    state_mut(|s| {
        s.public_create_enabled = enabled;
    });
    Ok(enabled)
}

#[ic_cdk::update]
fn set_default_extra_cycles(cycles: u64) -> Result<u64, CreateCanisterForCallerError> {
    let _ = require_owner()?;
    let max_cycles = state_read(|s| s.max_extra_cycles_per_create);
    if cycles > max_cycles {
        return Err(err_invalid(format!(
            "default_extra_cycles {} exceeds max_extra_cycles_per_create {}",
            cycles, max_cycles
        )));
    }
    state_mut(|s| {
        s.default_extra_cycles = cycles;
    });
    Ok(cycles)
}

#[ic_cdk::update]
fn set_max_extra_cycles_per_create(cycles: u64) -> Result<u64, CreateCanisterForCallerError> {
    let _ = require_owner()?;
    let default_cycles = state_read(|s| s.default_extra_cycles);
    if cycles < default_cycles {
        return Err(err_invalid(format!(
            "max_extra_cycles_per_create {} is lower than default_extra_cycles {}",
            cycles, default_cycles
        )));
    }
    state_mut(|s| {
        s.max_extra_cycles_per_create = cycles;
    });
    Ok(cycles)
}

#[ic_cdk::update]
fn set_max_canisters_per_caller(limit: u32) -> Result<u32, CreateCanisterForCallerError> {
    let _ = require_owner()?;
    if limit == 0 {
        return Err(err_invalid("limit must be > 0"));
    }
    state_mut(|s| {
        s.max_canisters_per_caller = limit;
    });
    Ok(limit)
}

#[ic_cdk::update]
fn reset_caller_quota(target: Principal) -> Result<u32, CreateCanisterForCallerError> {
    let _ = require_owner()?;
    if target == Principal::anonymous() {
        return Err(err_invalid("target cannot be anonymous"));
    }
    let previous = state_mut(|s| s.created_by_caller.remove(&target).unwrap_or(0));
    Ok(previous)
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let snapshot = state_read(Clone::clone);
    ic_cdk::storage::stable_save((snapshot,)).expect("stable_save failed");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let restored: Result<(State,), _> = ic_cdk::storage::stable_restore();
    let state = restored.map(|(s,)| s).unwrap_or_default();
    state_mut(|s| *s = state);
}

ic_cdk::export_candid!();
