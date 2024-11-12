use assert_cmd::{assert::Assert, Command};
use predicates::prelude::predicate;
use std::io::Write;
use tempfile::NamedTempFile;

fn create_tempfile(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

const FORCE_ONE_INSTANCE: &str = "a 1 0\nt 2 0\n1 2 1 0\n";

fn create_command(input_file: &NamedTempFile, additional_args: &[&str]) -> Assert {
    Command::cargo_bin("decdnnf_rs")
        .unwrap()
        .args(&[
            "model-enumeration",
            "--logging-level",
            "off",
            "-i",
            input_file.path().as_os_str().to_str().unwrap(),
        ])
        .args(additional_args)
        .assert()
}

#[test]
fn test_enumeration_default() {
    let file = create_tempfile(FORCE_ONE_INSTANCE);
    let assert = create_command(&file, &[]);
    assert.success().stdout(predicate::eq("v  1 0 \n"));
    std::mem::drop(file);
}

#[test]
fn test_enumeration_no_output() {
    let file = create_tempfile(FORCE_ONE_INSTANCE);
    let assert = create_command(&file, &["--do-not-print"]);
    assert.success().stdout(predicate::eq(""));
    std::mem::drop(file);
}

#[test]
fn test_enumeration_n_vars() {
    let file = create_tempfile(FORCE_ONE_INSTANCE);
    let assert = create_command(&file, &["--n-vars", "2"]);
    assert
        .success()
        .stdout(predicate::eq("v  1 -2 0 \nv  1  2 0 \n"));
    std::mem::drop(file);
}

#[test]
fn test_enumeration_n_vars_compact() {
    let file = create_tempfile(FORCE_ONE_INSTANCE);
    let assert = create_command(&file, &["--n-vars", "2", "-c"]);
    assert.success().stdout(predicate::eq("v  1 *2 0 \n"));
    std::mem::drop(file);
}
