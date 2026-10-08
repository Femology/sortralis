#![no_std]
use soroban_sdk::{contract, contractevent, contractimpl, Env};

#[contractevent]
pub struct SparseRiskEvent {
    pub flag: (),
}

#[contract]
pub struct SparseEventContract;

#[contractimpl]
impl SparseEventContract {
    pub fn emit_event(_env: Env) {}
}
