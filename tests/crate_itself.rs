extern crate assert_cmd;

use std::process::Command;

use assert_cmd::prelude::*;

mod crate_itself_fixed_no_std {
	use super::*;

	#[test]
	fn it_succeeds() {
		Command::cargo_bin(env!("CARGO_PKG_NAME"))
			.unwrap()
			.arg("check")
			.current_dir("./tests/crate_itself_fixed_no_std")
			.assert()
			.success();
	}
}

#[test]
fn it_prints_checkmark() {
	let output = Command::cargo_bin(env!("CARGO_PKG_NAME"))
		.unwrap()
		.arg("check")
		.current_dir("./tests/crate_itself_fixed_no_std")
		.output()
		.unwrap()
		.stdout;
	let output = String::from_utf8(output).unwrap();

	let expected_cause = "crate_itself_fixed_no_std: ✅";
	assert!(output.contains(expected_cause));
}
