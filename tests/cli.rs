use assert_cmd::Command;
use predicates::prelude::*;

const SOURCE: &str = "tests/fixtures/basic/src";
const COVERAGE: &str = "tests/fixtures/basic/coverage/coverage-final.json";

#[test]
fn prints_json_metrics() {
    Command::cargo_bin("crap4ts")
        .unwrap()
        .args([SOURCE, "--coverage", COVERAGE, "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"function\": \"classify\""))
        .stdout(predicate::str::contains("\"complexity\": 4"))
        .stdout(predicate::str::contains("\"coverage\": 0.5"));
}

#[test]
fn threshold_failure_uses_exit_code_one() {
    Command::cargo_bin("crap4ts")
        .unwrap()
        .args([SOURCE, "--coverage", COVERAGE, "--threshold", "3"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "1 function(s) above threshold 3.00",
        ));
}

#[test]
fn input_failure_uses_exit_code_two() {
    Command::cargo_bin("crap4ts")
        .unwrap()
        .args([SOURCE, "--coverage", "missing.json"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read coverage file"));
}
