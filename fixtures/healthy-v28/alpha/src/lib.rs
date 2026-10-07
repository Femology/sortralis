#![no_std]
extern crate sdk as soroban_sdk;
use sdk::{contract, contractimpl};

#[contract]
pub struct Alpha;

#[contractimpl]
impl Alpha {
    pub fn add(x: u32, y: u32) -> u32 {
        x + y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds_values() {
        assert_eq!(Alpha::add(2, 3), 5);
    }
}
