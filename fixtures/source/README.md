# Source-parser fixtures

These four fixtures exercise syntax detection only; no contract build is claimed.
The custom-account fixture follows the verified SDK v28 trait signature but its
body deliberately panics and must never be executed or deployed. The legacy export
fixture intentionally contains arguments rejected by SDK v28. All fixtures are
read as text, with exact source-line assertions in doctor-source tests.
