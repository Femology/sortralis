#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    pub fn hello(_env: Env) -> u32 {
        1
    }
    pub fn remove_me(_env: Env) -> u32 {
        2
    }
}
