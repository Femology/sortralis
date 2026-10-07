#![allow(clippy::unwrap_used)]
use doctor_wasm::{parse_entries, parse_hash};
#[test]
fn recorded_json_and_hash_examples_parse_and_malformed_outputs_fail() {
    for bytes in [
        include_bytes!("../../../fixtures/stellar-cli-27/interface.json").as_slice(),
        include_bytes!("../../../fixtures/stellar-cli-27/meta.json").as_slice(),
        include_bytes!("../../../fixtures/stellar-cli-27/env-meta.json").as_slice(),
    ] {
        assert!(!parse_entries(bytes).unwrap().is_empty());
    }
    let hash = include_bytes!("../../../fixtures/stellar-cli-27/hash.txt");
    assert_eq!(parse_hash(hash).unwrap().len(), 64);
    for bytes in [b"{broken".as_slice(), b"{}", b"[true]", b"\xff"] {
        assert!(parse_entries(bytes).is_err());
    }
    for bytes in [b"not hash".as_slice(), b"aa", b"zzzz", b"\xff"] {
        assert!(parse_hash(bytes).is_err());
    }
}

#[test]
fn recorded_absence_text_is_isolated_and_near_matches_are_not_ignored() {
    assert!(doctor_wasm::is_absent_metadata(include_str!(
        "../../../fixtures/stellar-cli-27/absent-meta.stderr"
    )));
    assert!(doctor_wasm::is_absent_metadata(include_str!(
        "../../../fixtures/stellar-cli-27/absent-env-meta.stderr"
    )));
    assert!(!doctor_wasm::is_absent_metadata("❌ error: invalid wasm"));
    assert!(!doctor_wasm::is_absent_metadata(
        "❌ error: no meta present in provided WASM file\nadditional failure"
    ));
}
