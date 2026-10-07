use doctor_runner::redact;
#[test]
fn secrets_are_masked_but_public_stellar_addresses_are_preserved() {
    let secret = format!("S{}", "A".repeat(55));
    let public = format!("G{}", "A".repeat(55));
    let contract = format!("C{}", "B".repeat(55));
    let input = format!("API_KEY=do-not-show\nAuthorization: Bearer sensitive\nseed={secret}\n{secret}\naddress={public}\ncontract={contract}\nghp_sensitive_token_value\nhttps://user:password@example.invalid\n-----BEGIN PRIVATE KEY-----\nsecret block\n-----END PRIVATE KEY-----\n");
    let output = redact(&input);
    for secret in [
        "do-not-show",
        "sensitive",
        "secret block",
        "user:password",
        &secret,
    ] {
        assert!(!output.contains(secret), "{output}");
    }
    assert!(output.contains(&public));
    assert!(output.contains(&contract));
}
#[test]
fn ordinary_diagnostics_and_network_passphrase_are_preserved() {
    let input = "error[E0308]: expected u32\nTest SDF failed\nPublic Global Stellar Network ; September 2015\ncompile package sdk 28.0.0\n";
    assert_eq!(redact(input), input);
}

#[test]
fn spaced_and_multiple_assignments_do_not_hide_adjacent_public_addresses() {
    let public = format!("G{}", "B".repeat(55));
    let input = format!("API_KEY = first-secret address={public}\n{{\"api_key\" : \"first-secret\", \"password\": \"second-secret\", \"address\": \"{public}\"}}\n");
    let output = redact(&input);
    assert!(!output.contains("first-secret"), "{output}");
    assert!(!output.contains("second-secret"), "{output}");
    assert_eq!(output.matches(&public).count(), 2, "{output}");
}

#[test]
fn synthetic_secret_fixture_is_redacted_without_replacing_public_identifiers() {
    let input = include_str!("fixtures/secret-redaction.txt");
    let output = redact(input);
    for secret in [
        "dummy-api-secret",
        "dummy-access-secret",
        "dummy-json-secret",
        "dummy-password-secret",
        "dummy-pem-secret",
        "ghp_dummy_example_token_value",
    ] {
        assert!(!output.contains(secret), "{output}");
    }
    assert!(output.contains(&format!("G{}", "B".repeat(55))));
    assert!(output.contains(&format!("C{}", "C".repeat(55))));
}

#[test]
fn displayed_arguments_are_escaped_and_secret_flag_values_are_masked() {
    let mut command = doctor_core::CommandResult {
        program: "cargo".into(),
        args: vec![
            "--token".into(),
            "unknown-sensitive-value".into(),
            "literal; echo surprise\n".into(),
        ],
        working_directory: std::path::PathBuf::from("."),
        status: doctor_core::CommandStatus::Exited { code: 0 },
        stdout: String::new(),
        stderr: String::new(),
        elapsed_millis: 0,
    };
    let display = doctor_runner::safe_command(&mut command);
    assert!(!display.contains("unknown-sensitive-value"));
    assert!(!display.contains('\n'));
    assert!(display.contains("literal; echo surprise"));
    assert_eq!(command.args[1], "[REDACTED]");
}

#[test]
fn redaction_is_idempotent_for_empty_assignments_and_already_masked_values() {
    let input = "API_KEY=\ntoken=[REDACTED]\npassword = \n";
    let once = redact(input);
    assert_eq!(redact(&once), once);
}
