#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

// Legacy comments that must not trigger findings:
// env.deployer().update_current_contract_wasm(hash);
// env.deployer().with_current_contract(salt).deploy(hash);
// #[contracttrait]
// pub const S: &str = __SPEC_XDR_INPUT;
// fn upgrade(env: Env) {}
// #[soroban_sdk::contracttype(export = false)]
// fn __check_auth() {}

/*
   Block comment containing:
   update_current_contract_wasm
   with_current_contract
   __SPEC_XDR_
   contracttrait
   export = true
*/

const STRING_ONE: &str = "update_current_contract_wasm";
const STRING_TWO: &str = "with_current_contract";
const STRING_THREE: &str = "#[contracttrait]";
const STRING_FOUR: &str = "__SPEC_XDR_INPUT";
const STRING_FIVE: &str = "contracttype(export = false)";
const STRING_SIX: &str = "__check_auth";

#[contract]
pub struct CleanContract;

#[contractimpl]
impl CleanContract {
    pub fn info(_env: Env) -> &'static str {
        STRING_ONE
    }
}
