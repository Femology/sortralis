#![no_std]
use soroban_sdk::{contract, contractimpl};
#[contract]
pub struct Contract;
#[contractimpl]
impl Contract {
    pub fn add(x: u32, y: u32) -> u32 { x + y }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intentional_regression() { assert_eq!(Contract::add(2, 3), 6, "intentional failing-tests fixture"); }
}
