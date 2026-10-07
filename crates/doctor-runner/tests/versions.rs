use doctor_runner::parse_version;

#[test]
fn verified_real_outputs_parse_the_correct_tool_versions() {
    for (program, output, version) in [
        (
            "rustc",
            include_str!("fixtures/rustc-version.txt"),
            "1.96.0",
        ),
        (
            "cargo",
            include_str!("fixtures/cargo-version.txt"),
            "1.96.0",
        ),
        (
            "stellar",
            include_str!("fixtures/stellar-version.txt"),
            "27.0.0",
        ),
    ] {
        assert_eq!(
            parse_version(program, output).map(|version| version.to_string()),
            Some(version.into())
        );
    }
}

#[test]
fn synthetic_format_variations_keep_semver_information() {
    for (output, version) in [
        ("  stellar 27.0.0\r\n", "27.0.0"),
        (
            "Stellar CLI version: v28.0.0-rc.1+build.42",
            "28.0.0-rc.1+build.42",
        ),
        (
            "unrelated line\nstellar 27.0.0, (hash)\nstellar-xdr 99.0.0",
            "27.0.0",
        ),
    ] {
        assert_eq!(
            parse_version("stellar", output).map(|version| version.to_string()),
            Some(version.into())
        );
    }
}

#[test]
fn unrelated_or_malformed_versions_are_not_guessed() {
    for output in [
        "",
        "stellar-xdr 27.0.0",
        "cargo 1.96.0",
        "stellar unknown 28.0.0",
        "stellar 27",
        "stellar 27.0",
        "version 28.0.0",
        "error: stellar 27.0.0",
    ] {
        assert!(parse_version("stellar", output).is_none(), "{output}");
    }
}
