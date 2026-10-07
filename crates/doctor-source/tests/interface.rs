#![allow(clippy::unwrap_used)]
use doctor_source::{inventory_interface_text, SourceOptions};
use std::path::Path;
#[test]
fn verified_attributes_inventory_functions_types_events_without_body_noise() {
    let source = r#"
use soroban_sdk::{contractimpl, contracttype, contractevent, Env};
#[contracttype] struct State { n: u32 }
#[contractevent(data_format = "vec")] struct Changed { #[topic] id: u32, count: u64 }
#[contractimpl] impl Contract {
    pub fn set(env: Env, n: u32) -> State { State { n } }
    fn private_helper() {}
}
impl Other { pub fn unrelated() {} }
"#;
    let inv = inventory_interface_text(Path::new("src/lib.rs"), source, &SourceOptions::default())
        .unwrap();
    assert_eq!(inv.functions.len(), 1);
    assert_eq!(inv.functions[0].identity, "set");
    assert!(inv.functions[0].signature.contains("u32"));
    assert_eq!(inv.types.len(), 1);
    assert_eq!(inv.events.len(), 1);
    assert!(inv.events[0].fields[0]
        .attributes
        .iter()
        .any(|a| a.contains("topic")));
    assert!(inv.events[0]
        .attributes
        .iter()
        .any(|a| a.contains("data_format")));
}
#[test]
fn unrelated_names_comments_and_strings_are_not_sdk_interfaces() {
    let source = r#"
use other::{contractimpl, contractevent, contracttype};
#[contractevent] struct Fake { n: u32 }
#[contracttype] enum FakeKey { One }
#[contractimpl] impl Fake { pub fn pretend() {} }
// #[contractimpl] impl Real { pub fn false_positive() {} }
"#;
    let inv = inventory_interface_text(Path::new("src/lib.rs"), source, &SourceOptions::default())
        .unwrap();
    assert!(inv.functions.is_empty() && inv.types.is_empty() && inv.events.is_empty());
}
