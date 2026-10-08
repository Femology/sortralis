#![no_std]
use soroban_sdk::{contract, contractimpl, BytesN, Env};

#[contract]
pub struct LegacyDeployContract;

#[contractimpl]
impl LegacyDeployContract {
    pub fn deploy_contract(env: Env, salt: BytesN<32>, wasm_hash: BytesN<32>) {
        env.deployer().with_current_contract(salt).deploy(wasm_hash);
    }
}
