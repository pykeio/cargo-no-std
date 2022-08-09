mod check;
mod check_source;
mod ext;
mod util;
#[cfg(target_os = "linux")]
mod verify;

use std::{collections::HashSet, path::PathBuf};

use cargo_metadata::{Metadata, Package};
use clap::{App, Arg, ArgMatches};
use colored::Colorize;
use itertools::Itertools;

use crate::check::*;
use crate::check_source::*;
use crate::ext::*;
use crate::util::*;

pub static SUCCESS: &str = "✅";
pub static FAILURE: &str = "❌";
pub static MAYBE: &str = "❓";
pub static SKIPPED: &str = "⏩";

fn check_and_print_package(
	package: &Package,
	resolved_dependency_features: &[Feature],
	metadata: &Metadata,
	metadata_full: &Metadata,
	allowed: &HashSet<String>,
	is_main_pkg: bool
) -> bool {
	let mut package_did_fail = false;

	let package_features: Vec<Feature> = resolved_dependency_features
		.iter()
		.filter(|n| n.package_id == package.id.repr)
		.map(|n| n.to_owned())
		.collect();
	let active_features = package.active_features_for_features(&package_features);
	let active_dependencies = package.active_dependencies(&active_features);
	let _active_packages = dependencies_to_packages(package, metadata_full, &active_dependencies);
	let _resolved_dependency_features = package.all_dependency_features(metadata_full, &active_features);

	let mut support = CrateSupport::NoOffenseDetected;
	if package.is_proc_macro() {
		support = CrateSupport::ProcMacro;
	}
	if allowed.contains(&package.name) {
		support = CrateSupport::Skipped;
	}
	if support == CrateSupport::NoOffenseDetected {
		match is_main_pkg {
			false => {
				// TODO: check more than one
				support = package
					.lib_target_sources()
					.into_iter()
					.map(PathBuf::from)
					.map(|src_path| get_crate_support_from_source(&src_path))
					.next()
					.unwrap_or(CrateSupport::NoOffenseDetected);
			}
			true => {
				support = package
					.bin_target_sources()
					.into_iter()
					.chain(package.lib_target_sources())
					.map(PathBuf::from)
					.map(|src_path| get_crate_support_from_source(&src_path))
					.next()
					.unwrap_or(CrateSupport::NoOffenseDetected);
			}
		}
	}

	let check = CheckResult {
		package_name: package.name.clone(),
		support,
		active_features
	};

	// set flag that at least one crate check failed
	if !check.no_std_itself() {
		package_did_fail = true;
	}
	let overall_res = match check.support {
		CrateSupport::ProcMacro => SUCCESS,
		CrateSupport::OnlyWithoutFeature(ref feature) => match check.is_feature_active(feature) {
			false => SUCCESS,
			true => MAYBE
		},
		CrateSupport::NoOffenseDetected => SUCCESS,
		CrateSupport::SourceOffenses(ref offenses) => {
			if offenses.contains(&SourceOffense::MissingNoStdAttribute) {
				FAILURE
			} else {
				MAYBE
			}
		}
		CrateSupport::Skipped => SKIPPED
	};
	println!("{} {}", overall_res, check.package_name.bold());
	if check.no_std_itself() {
		return package_did_fail;
	}
	if let CrateSupport::OnlyWithoutFeature(feature) = &check.support {
		println!("{}{}{}", "  ★ Crate supports no_std if \"".green(), feature.green(), "\" feature is deactivated.".green());
		let feat = check.find_active_feature_by_name(feature).unwrap();
		feat.print(metadata, 2);
	}
	if let CrateSupport::SourceOffenses(ref offenses) = check.support {
		for offense in offenses.iter().sorted() {
			match offense {
				SourceOffense::MissingNoStdAttribute => {
					println!("{}", "  ❯ Did not find a #![no_std] attribute or a simple conditional attribute like #![cfg_attr(not(feature = \"std\"), no_std)] in the crate source. Crate most likely doesn't support no_std without changes.".red());
					return package_did_fail;
				}
				SourceOffense::UseStdStatement(stmt) => {
					println!("{}", "  ❯ Source code contains an explicit `use std::` statement.".bright_yellow());
					print!("{}", stmt.to_string().bright_black());
				}
			}
		}
	}

	package_did_fail
}

fn run_check(matches: &ArgMatches) -> anyhow::Result<()> {
	let metadata_full = metadata_run(Some("--all-features".to_owned())).unwrap();
	let metadata = metadata_run(None).unwrap();

	let target_workspace_member = main_ws_member_from_args(&metadata, matches.value_of("package"));

	let target_package = metadata.find_package(&target_workspace_member.repr).unwrap();
	let features = features_from_args(
		target_package.id.repr.clone(),
		matches.is_present("no-default-features"),
		matches
			.values_of("features")
			.map(|n| n.into_iter().map(|m| m.to_owned()).collect::<Vec<String>>())
			.unwrap_or_default()
	);
	let allowed = matches
		.values_of("allow")
		.map(|n| n.into_iter().map(|m| m.to_owned()).collect::<HashSet<String>>())
		.unwrap_or_default();

	let active_features = target_package.active_features_for_features(&features);
	let active_dependencies = target_package.active_dependencies(&active_features);
	let active_packages = dependencies_to_packages(target_package, &metadata_full, &active_dependencies);

	let mut package_did_fail = false;
	let resolved_dependency_features = target_package.all_dependency_features(&metadata_full, &active_features);

	let main_package = metadata
		.packages
		.iter()
		.find(|n| &n.id == target_workspace_member)
		.expect("Unable to find main package.");
	if check_and_print_package(main_package, &resolved_dependency_features, &metadata, &metadata_full, &allowed, true) {
		package_did_fail = true;
	}

	for package in active_packages.iter() {
		if check_and_print_package(package, &resolved_dependency_features, &metadata, &metadata_full, &allowed, false) {
			package_did_fail = true;
		}
	}

	#[cfg(not(target_os = "linux"))]
	if package_did_fail {
		println!();
		println!("{}", "These results are only guesses; run again on Linux to truly verify no_std support for crates.".bright_black());
		std::process::exit(1);
	}

	Ok(())
}

#[cfg(target_os = "linux")]
fn active_packages(matches: &clap::ArgMatches) -> Vec<Package> {
	let metadata_full = metadata_run(Some("--all-features".to_owned())).unwrap();
	let metadata = metadata_run(None).unwrap();

	let target_workspace_member = main_ws_member_from_args(&metadata, matches.value_of("package"));

	let target_package = metadata.find_package(&target_workspace_member.repr).unwrap();
	let features = features_from_args(
		target_package.id.repr.clone(),
		matches.is_present("no-default-features"),
		matches
			.values_of("features")
			.map(|n| n.into_iter().map(|m| m.to_owned()).collect::<Vec<String>>())
			.unwrap_or_default()
	);

	let active_features = target_package.active_features_for_features(&features);
	let active_dependencies = target_package.active_dependencies(&active_features);

	dependencies_to_packages(target_package, &metadata_full, &active_dependencies)
}

#[cfg(target_os = "linux")]
fn run_verify(matches: &clap::ArgMatches) -> anyhow::Result<()> {
	// First run a normal build so we see build progress
	let mut build_args = vec!["build"];
	if matches.is_present("no-default-features") {
		build_args.push("--no-default-features");
	}
	let features_arg = matches
		.values_of("features")
		.map(|n| n.into_iter().map(|m| m.to_owned()).collect::<Vec<String>>())
		.unwrap_or_default()
		.join(",");
	if !features_arg.is_empty() {
		build_args.push("--features");
		build_args.push(&features_arg);
	}

	duct::cmd("cargo", &build_args).run().unwrap();
	println!();

	let build_result = escargot::CargoBuild::new()
		.set_features(
			matches.is_present("no-default-features"),
			matches
				.values_of("features")
				.map(|n| n.into_iter().map(|m| m.to_owned()).collect::<Vec<String>>())
				.unwrap_or_default()
		)
		.exec()
		.unwrap();
	let raw_messages: Vec<escargot::Message> = build_result.into_iter().filter_map(|raw_msg| raw_msg.ok()).collect::<Vec<_>>();
	let decoded_messages = raw_messages.iter().filter_map(|raw_msg| raw_msg.decode().ok()).collect::<Vec<_>>();

	let as_compiler_artifact = |msg| {
		if let escargot::format::Message::CompilerArtifact(artifact) = msg {
			return Some(artifact);
		}
		None
	};
	let artifact_filenames_for_message =
		|msg| as_compiler_artifact(msg).map(|artifact| artifact.filenames.into_iter().map(|n| n.into_owned()).collect::<Vec<_>>());

	let artifact_messages = decoded_messages
		.clone()
		.into_iter()
		.filter_map(artifact_filenames_for_message)
		.collect::<Vec<_>>();

	let main_artifact_message = artifact_messages.last().unwrap();
	let main_artifact_path = main_artifact_message.first().unwrap();

	let main_has_std = self::verify::rlib_contains_namespace(main_artifact_path, "std");

	for (i, msg) in decoded_messages.clone().into_iter().enumerate() {
		let is_last = i == decoded_messages.len() - 1;
		let artifact_filenames = artifact_filenames_for_message(msg.clone());
		if let Some(filenames) = artifact_filenames {
			let dependency_name = as_compiler_artifact(msg).unwrap().target.name;
			if !active_packages(matches).into_iter().map(|pkg| pkg.name).any(|x| x == dependency_name) && !is_last {
				continue;
			}
			let artifact_path = filenames.first().unwrap();
			if artifact_path.extension() != Some(std::ffi::OsStr::new("rlib")) {
				continue;
			}

			let has_std = verify::rlib_contains_namespace(artifact_path, "std");
			let icon = match has_std {
				true => FAILURE,
				false => SUCCESS
			};
			println!("{} {}{}", icon, dependency_name.bold(), if has_std { " references std" } else { " contains no references to std" });
		}
	}

	if main_has_std {
		std::process::exit(1);
	}

	Ok(())
}

fn main() -> anyhow::Result<()> {
	#[allow(unused_mut)]
	let mut app = App::new("cargo no-std")
		.arg(Arg::with_name("dummy").hidden(true).possible_value("no-std"))
		.arg(Arg::with_name("no-default-features").long("no-default-features"))
		.arg(Arg::with_name("features").long("features").multiple(true).takes_value(true))
		.arg(Arg::with_name("allowed").long("allowed").multiple(true).takes_value(true))
		.arg(Arg::with_name("package").long("package").short('p').takes_value(true));

	#[cfg(target_os = "linux")]
	{
		app.arg(Arg::with_name("no-verify").long("no-verify"));
	}

	let matches = app.clone().get_matches();

	run_check(&matches)?;
	println!();

	#[cfg(not(target_os = "linux"))]
	return Ok(());

	#[cfg(target_os = "linux")]
	if matches.is_present("no-verify") {
		println!("{}", "Checking crates for std linkage".bright_black());
		run_verify(&matches)?;
		Ok(())
	}
}
