//! Integration tests for `decdnnf_rs` (sampling).

use std::io::Write;

use assert_cmd::{assert::Assert, Command};
use predicates::prelude::{predicate, PredicateBooleanExt};
use tempfile::NamedTempFile;

fn create_tempfile(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

const TRIVIAL_INSTANCE: &str = "t 1 0\n";

fn create_command(input_file: &NamedTempFile, additional_args: &[&str]) -> Assert {
    Command::cargo_bin("decdnnf_rs")
        .unwrap()
        .args([
            "sampling",
            "--logging-level",
            "off",
            "-i",
            input_file.path().as_os_str().to_str().unwrap(),
        ])
        .args(additional_args)
        .assert()
}

#[test]
fn test_sampling_all() {
    let file = create_tempfile(TRIVIAL_INSTANCE);
    let assert = create_command(&file, &["--n-vars=2"]);
    assert.success().stdout(
        predicate::str::contains("v -1 -2 0")
            .and(predicate::str::contains("v -1  2 0"))
            .and(predicate::str::contains("v  1 -2 0"))
            .and(predicate::str::contains("v  1  2 0")),
    );
    std::mem::drop(file);
}
