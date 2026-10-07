#![allow(clippy::unwrap_used)]
use doctor_cli::check::{sdk28_cli_supported, sdk_context};
use doctor_source::TargetContext;
use semver::Version;
#[test]
fn verified_minimum_is_inclusive_and_prereleases_are_conservative() {
    for (version, expected) in [
        ("25.1.9", false),
        ("25.2.0-rc.1", false),
        ("25.2.0", true),
        ("27.0.0", true),
    ] {
        assert_eq!(
            sdk28_cli_supported(&Version::parse(version).unwrap()),
            expected
        );
    }
}
#[test]
fn requirements_that_do_not_select_sdk28_are_not_guessed() {
    for (requirement, context) in [
        ("=28.0.0", TargetContext::Sdk28),
        ("^28.1", TargetContext::Sdk28),
        ("27", TargetContext::OtherSdk),
        ("*", TargetContext::Unknown),
        (">=27", TargetContext::Unknown),
    ] {
        assert_eq!(sdk_context(requirement), context);
    }
}
