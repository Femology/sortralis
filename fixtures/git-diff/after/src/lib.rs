use soroban_sdk::{contractimpl, contracttype, contractevent, Env};
#[contracttype(export = false)] enum DataKey { State }
#[contracttype] struct State { count: u64 }
#[contractevent] struct Changed { #[topic] id: u32, count: u64 }
#[contractimpl] impl Contract {
    pub fn update(env: Env, value: State) -> u64 {
        env.storage().persistent().set(&DataKey::State, &value);
        1u64
    }
    pub fn added() {}
}
