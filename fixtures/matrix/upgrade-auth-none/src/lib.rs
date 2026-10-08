#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct NoAuthUpgradeContract;

#[contractimpl]
impl NoAuthUpgradeContract {
    pub fn upgrade(_env: Env) {
        // missing require_auth()
    }
}
