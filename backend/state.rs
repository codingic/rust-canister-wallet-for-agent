use candid::{CandidType, Principal};
use serde::Deserialize;
use std::cell::RefCell;

#[derive(CandidType, Deserialize, Clone, Debug, Default)]
pub struct State {
    pub owner: Option<Principal>,
    pub paused: bool,
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
