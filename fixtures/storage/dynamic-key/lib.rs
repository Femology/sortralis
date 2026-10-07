use soroban_sdk::{contracttype, Env};
#[contracttype]
pub enum DataKey { State }
#[contracttype]
pub struct State { pub count: u32, pub enabled: bool }
#[contracttype]
pub struct Unrelated { pub label: u32 }
pub fn save(env: Env, value: State, key: u32) {
    env.storage().persistent().set(&key, &value);
}
