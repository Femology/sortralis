#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct ContractB;

#[contractimpl]
impl ContractB {
    pub fn greet(_env: Env) -> u32 {
        200
    }
}
