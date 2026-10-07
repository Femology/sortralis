use soroban_sdk::{contracttype, contracterror};

#[contracttype(export = false)]
pub struct State { pub value: u32 }

#[contracterror(export = true)]
pub enum Error { Failed = 1 }
