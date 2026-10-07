use soroban_sdk::{contract, contractimpl, Env};
#[contract]
pub struct Contract;
#[contractimpl]
impl Contract {
    pub fn value(_env: Env) -> u32 { 7 }
}
