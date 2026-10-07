use soroban_sdk::{contractimpl, contracttype, contractevent, Env};
#[contracttype(export = false)] enum DataKey { State }
#[contracttype] struct State { count: u32 }
#[contractevent] struct Changed { #[topic] id: u32, count: u32 }
#[contractimpl] impl Contract {
    pub fn update(env: Env, value: State) -> u32 {
        env.storage().persistent().set(&DataKey::State, &value);
        1u32
    }
    pub fn removed() {}
}
