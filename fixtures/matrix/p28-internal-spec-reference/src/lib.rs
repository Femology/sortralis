#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

pub const SPEC_ENTRY_REF: &str = __SPEC_XDR_INPUT;

#[contract]
pub struct InternalSpecContract;

#[contractimpl]
impl InternalSpecContract {
    pub fn test(_env: Env) {}
}
