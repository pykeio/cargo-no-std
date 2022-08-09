use std::path::PathBuf;
use std::{borrow, fs};

use fallible_iterator::FallibleIterator;
use object::{Object, ObjectSection};

fn die_entry_is_namespace<T: gimli::read::Reader>(
	dwarf: &gimli::Dwarf<T>,
	unit: &gimli::read::Unit<T>,
	entry: &gimli::read::DebuggingInformationEntry<T>,
	namespace_name: &str
) -> bool {
	if entry.tag() != gimli::DW_TAG_namespace {
		return false;
	}

	for attr in entry.attrs().iterator() {
		let attr_name = attr.clone().unwrap().name().static_string().unwrap();
		if attr_name == "DW_AT_name" {
			let raw_attrs_str = dwarf.attr_string(unit, attr.unwrap().value()).unwrap();
			let value = raw_attrs_str.to_string().unwrap();
			// dbg!(&value);
			return value == namespace_name;
		}
	}

	false
}

/// Check wether a object file contains a specified namespace.
fn object_file_contains_namespace(object: &object::File, namespace_name: &str) -> Result<bool, gimli::Error> {
	let endian = if object.is_little_endian() { gimli::RunTimeEndian::Little } else { gimli::RunTimeEndian::Big };

	// Load a section and return as `Cow<[u8]>`.
	let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
		Ok(object
			.section_by_name(id.name())
			.map(|s| borrow::Cow::Borrowed(s.data().unwrap()))
			.unwrap_or(borrow::Cow::Borrowed(&[][..])))
	};

	// Load all of the sections.
	let dwarf_cow = gimli::Dwarf::load(&load_section)?;

	// Borrow a `Cow<[u8]>` to create an `EndianSlice`.
	let borrow_section: &dyn for<'a> Fn(&'a borrow::Cow<[u8]>) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
		&|section| gimli::EndianSlice::new(section, endian);

	// Create `EndianSlice`s for all of the sections.
	let dwarf = dwarf_cow.borrow(&borrow_section);

	// Iterate over the compilation units.
	let mut iter = dwarf.units();
	while let Some(header) = iter.next()? {
		let unit = dwarf.unit(header)?;

		// Iterate over the Debugging Information Entries (DIEs) in the unit.
		let mut entries = unit.entries();
		while let Some((_, entry)) = entries.next_dfs()? {
			if die_entry_is_namespace(&dwarf, &unit, entry, namespace_name) {
				return Ok(true);
			}
		}
	}
	Ok(false)
}

pub fn rlib_contains_namespace(rlib_path: &PathBuf, namespace_name: &str) -> bool {
	let contents = fs::read(&rlib_path).unwrap();
	let archive = goblin::archive::Archive::parse(&contents).unwrap();

	for entry in archive.members() {
		let entry_path: PathBuf = entry.into();
		if entry_path.extension().map(|n| n.to_str().unwrap()) != Some("o") {
			continue;
		}
		// dbg!(entry);
		let entry_bytes = archive.extract(entry, &contents).unwrap();
		let object = object::File::parse(entry_bytes).unwrap();
		if object_file_contains_namespace(&object, namespace_name).unwrap() {
			return true;
		}
	}

	false
}
