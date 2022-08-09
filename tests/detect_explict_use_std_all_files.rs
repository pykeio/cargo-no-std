extern crate assert_cmd;

use std::process::Command;

use assert_cmd::prelude::*;

#[test]
fn it_fails_with_exit_code_1() {
	Command::cargo_bin(env!("CARGO_PKG_NAME"))
		.unwrap()
		.arg("check")
		.current_dir("./tests/detect_explicit_use_std_all_files")
		.assert()
		.code(1);
}

#[test]
fn it_prints_cause() {
	let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
		.unwrap()
		.arg("check")
		.current_dir("./tests/detect_explicit_use_std_all_files")
		.output()
		.unwrap()
		.stdout;
	let output = String::from_utf8(output).unwrap();

	let expected_cause = "Source code contains an explicit `use std::` statement";
	assert!(output.contains(expected_cause));
}

#[test]
fn it_does_not_warn_about_no_std_statement() {
	let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
		.unwrap()
		.arg("check")
		.current_dir("./tests/detect_explicit_use_std_all_files")
		.output()
		.unwrap()
		.stdout;
	let output = String::from_utf8(output).unwrap();

	let expected_cause = "Did not find a #![no_std] attribute";
	assert!(!output.contains(expected_cause));
}
