use soroban_sdk::{contracttype, contracterror};
#[contracttype]
pub struct State { pub value: u32 }
#[contracterror]
pub enum Error { Failed = 1 }
