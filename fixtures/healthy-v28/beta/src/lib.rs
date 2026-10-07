#![no_std]
extern crate sdk as soroban_sdk;
use sdk::{contract, contractimpl};

#[contract]
pub struct Beta;

#[contractimpl]
impl Beta {
    pub fn add(x: u32, y: u32) -> u32 {
        x + y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds_values() {
        assert_eq!(Beta::add(2, 3), 5);
    }
}
