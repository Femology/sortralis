#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

#[contract]
pub struct MultilineContract;

#[contractimpl]
impl MultilineContract {
    pub fn upgrade(
        env: Env,
        admin: Address,
        new_wasm_hash: BytesN<32>,
    ) {
        admin.require_auth();
        env
            .deployer(   )
            .update_current_contract_wasm (
                new_wasm_hash
            );
    }
}
