#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct DirectAuthContract;

#[contractimpl]
impl DirectAuthContract {
    pub fn upgrade(_env: Env, admin: Address) {
        admin.require_auth();
    }
}
