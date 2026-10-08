#![no_std]
use soroban_sdk::{contract, contractimpl, contracttrait, Env};

#[contracttrait]
pub struct InvalidTraitStruct {
    pub value: u32,
}

#[contract]
pub struct ContractTraitContract;

#[contractimpl]
impl ContractTraitContract {
    pub fn test(_env: Env) {}
}
