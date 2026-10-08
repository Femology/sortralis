#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthyState {
    pub count: u32,
}

#[contract]
pub struct HealthyContract;

#[contractimpl]
impl HealthyContract {
    pub fn add(_env: Env, a: u32, b: u32) -> u32 {
        a + b
    }
}
