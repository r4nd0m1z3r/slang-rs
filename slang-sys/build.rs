extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};

const SLANG_SOURCE_VERSION: &str = "2026.17.1";

const SLANG_STATIC_LIBRARIES: [&[&str]; 6] = [
	&[
		"libslang-compiler.a",
		"slang-compiler.lib",
		"libslang.a",
		"slang.lib",
	],
	&["libcompiler-core.a", "compiler-core.lib"],
	&["libcore.a", "core.lib"],
	&["libcmark-gfm.a", "cmark-gfm.lib"],
	&["libminiz.a", "miniz.lib"],
	&["liblz4.a", "liblz4_static.a", "lz4.lib", "lz4_static.lib"],
];

fn main() {
	println!("cargo:rerun-if-env-changed=SLANG_DIR");
	println!("cargo:rerun-if-env-changed=SLANG_INCLUDE_DIR");
	println!("cargo:rerun-if-env-changed=SLANG_LIB_DIR");
	println!("cargo:rerun-if-env-changed=VULKAN_SDK");
	println!("cargo:rerun-if-env-changed=SLANG_SOURCE_DIR");
	println!("cargo:rerun-if-env-changed=SLANG_ENABLE_DXIL");

	let static_build = env::var("CARGO_FEATURE_STATIC").is_ok();
	let mut extra_include_dirs = Vec::new();

	let include_dir = if static_build {
		let source_dir = slang_source_dir();

		let build_dir = compile_slang(&source_dir);

		link_slang_static(&build_dir);

		if let Some(dir) = find_file(&build_dir, "slang-tag-version.h")
			.and_then(|path| path.parent().map(Path::to_path_buf))
		{
			extra_include_dirs.push(dir);
		}

		source_dir.join("include")
	} else {
		let (include_dir, lib_dir) = slang_installation();

		if !lib_dir.as_os_str().is_empty() {
			println!("cargo:rustc-link-search=native={}", lib_dir.display());
		}

		println!("cargo:rustc-link-lib=dylib=slang");

		include_dir
	};

	let out_dir = env::var("OUT_DIR").expect("Couldn't determine output directory.");

	let mut builder =
		bindgen::builder().header(format!("{}/slang.h", include_dir.display()).as_str());

	if static_build {
		builder = builder.clang_arg("-DSLANG_STATIC");
	}
	for dir in extra_include_dirs {
		builder = builder.clang_arg(format!("-I{}", dir.display()));
	}

	builder
		.clang_arg("-v")
		.clang_arg("-xc++")
		.clang_arg("-std=c++17")
		.allowlist_function("spReflection.*")
		.allowlist_function("spComputeStringHash")
		.allowlist_function("slang_.*")
		.allowlist_type("slang.*")
		.allowlist_var("SLANG_.*")
		.with_codegen_config(
			bindgen::CodegenConfig::FUNCTIONS
				| bindgen::CodegenConfig::TYPES
				| bindgen::CodegenConfig::VARS,
		)
		.parse_callbacks(Box::new(ParseCallback {}))
		.default_enum_style(bindgen::EnumVariation::Rust {
			non_exhaustive: false,
		})
		.constified_enum("SlangProfileID")
		.constified_enum("SlangCapabilityID")
		.vtable_generation(true)
		.layout_tests(false)
		.derive_copy(true)
		.generate()
		.expect("Couldn't generate bindings.")
		.write_to_file(format!("{out_dir}/bindings.rs").as_str())
		.expect("Couldn't write bindings.");
}

fn slang_installation() -> (PathBuf, PathBuf) {
	let include_dir = if let Ok(dir) = env::var("SLANG_INCLUDE_DIR") {
		PathBuf::from(dir)
	} else if let Ok(dir) = env::var("SLANG_DIR") {
		PathBuf::from(dir).join("include")
	} else if let Ok(dir) = env::var("VULKAN_SDK") {
		PathBuf::from(dir).join("include/slang")
	} else {
		panic!(
			"The environment variable SLANG_INCLUDE_DIR, SLANG_DIR, or VULKAN_SDK must be set, \
			or the `static` feature must be enabled to build Slang from source."
		);
	};

	let lib_dir = if let Ok(dir) = env::var("SLANG_LIB_DIR") {
		PathBuf::from(dir)
	} else if let Ok(dir) = env::var("SLANG_DIR") {
		PathBuf::from(dir).join("lib")
	} else if let Ok(dir) = env::var("VULKAN_SDK") {
		PathBuf::from(dir).join("lib")
	} else {
		panic!(
			"The environment variable SLANG_LIB_DIR, SLANG_DIR, or VULKAN_SDK must be set, \
			or the `static` feature must be enabled to build Slang from source."
		);
	};

	(include_dir, lib_dir)
}

fn slang_source_dir() -> PathBuf {
	if let Ok(dir) = env::var("SLANG_SOURCE_DIR") {
		let dir = PathBuf::from(dir);
		println!("cargo:rerun-if-changed={}", dir.display());
		return dir;
	}

	let manifest_dir = PathBuf::from(
		env::var("CARGO_MANIFEST_DIR").expect("Couldn't determine manifest directory."),
	);

	for candidate in [
		manifest_dir.join("slang-src"),
		manifest_dir.join("../slang-src"),
	] {
		if candidate.join("include/slang.h").is_file() {
			println!("cargo:rerun-if-changed={}", candidate.display());
			return candidate;
		}
	}

	panic!("Couldn't find the Slang {SLANG_SOURCE_VERSION} source tree.");
}

fn compile_slang(source_dir: &Path) -> PathBuf {
	let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Couldn't determine output directory."));
	let destination = out_dir.join(format!("slang-{SLANG_SOURCE_VERSION}"));

	cmake::Config::new(source_dir)
		.define("SLANG_LIB_TYPE", "STATIC")
		.define("SLANG_ENABLE_SLANG_GLSLANG", "OFF")
		.define("SLANG_ENABLE_TESTS", "OFF")
		.define("SLANG_ENABLE_EXAMPLES", "OFF")
		.define("SLANG_ENABLE_GFX", "OFF")
		.define("SLANG_ENABLE_SLANG_RHI", "OFF")
		.define("SLANG_ENABLE_SLANGD", "OFF")
		.define("SLANG_ENABLE_SLANGC", "OFF")
		.define("SLANG_ENABLE_SLANGI", "OFF")
		.define("SLANG_ENABLE_SLANGRT", "OFF")
		.define("SLANG_ENABLE_REPLAYER", "OFF")
		.define("SLANG_SLANG_LLVM_FLAVOR", "DISABLE")
		.define("SLANG_EXCLUDE_DAWN", "ON")
		.define("SLANG_EXCLUDE_TINT", "ON")
		.define("SLANG_STANDARD_MODULE_DEVELOP_BUILD", "OFF")
		.define("SLANG_ENABLE_RELEASE_DEBUG_INFO", "OFF")
		.define("SLANG_ENABLE_CUDA", "OFF")
		.define("SLANG_ENABLE_OPTIX", "OFF")
		.define("SLANG_ENABLE_NVAPI", "OFF")
		.define("SLANG_ENABLE_XLIB", "OFF")
		.define("SLANG_ENABLE_AFTERMATH", "OFF")
		.define(
			"SLANG_ENABLE_DXIL",
			env::var("SLANG_ENABLE_DXIL").unwrap_or_else(|_| "OFF".to_string()),
		)
		.define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
		.define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
		.profile("Release")
		.out_dir(&destination)
		.build_target("slang")
		.build();

	destination.join("build")
}

fn link_slang_static(build_dir: &Path) {
	let mut emitted_search_dirs = Vec::new();

	for candidates in SLANG_STATIC_LIBRARIES {
		let archive = candidates
			.iter()
			.find_map(|name| find_file(build_dir, name))
			.unwrap_or_else(|| {
				panic!(
					"Couldn't find any of {candidates:?} in {}.",
					build_dir.display()
				)
			});

		let search_dir = archive.parent().expect("Archive has no parent directory.");

		if !emitted_search_dirs.iter().any(|dir| dir == search_dir) {
			emitted_search_dirs.push(search_dir.to_path_buf());
			println!("cargo:rustc-link-search=native={}", search_dir.display());
		}

		println!("cargo:rustc-link-lib=static={}", link_name(&archive));
	}

	match env::var("TARGET")
		.expect("Couldn't determine target.")
		.as_str()
	{
		target if target.contains("msvc") => println!("cargo:rustc-link-lib=msvcprt"),
		target if target.contains("apple") => println!("cargo:rustc-link-lib=c++"),
		_ => println!("cargo:rustc-link-lib=stdc++"),
	}
}

fn link_name(archive: &Path) -> String {
	let stem = archive
		.file_stem()
		.expect("Archive has no file stem.")
		.to_string_lossy()
		.into_owned();

	stem.strip_prefix("lib").unwrap_or(&stem).to_string()
}

fn find_file(dir: &Path, file_name: &str) -> Option<PathBuf> {
	let entries = std::fs::read_dir(dir).ok()?;

	for entry in entries.flatten() {
		let path = entry.path();

		if path.is_dir() {
			if let Some(found) = find_file(&path, file_name) {
				return Some(found);
			}
		} else if path.file_name().is_some_and(|name| name == file_name) {
			return Some(path);
		}
	}

	None
}

#[derive(Debug)]
struct ParseCallback {}

impl bindgen::callbacks::ParseCallbacks for ParseCallback {
	fn enum_variant_name(
		&self,
		enum_name: Option<&str>,
		original_variant_name: &str,
		_variant_value: bindgen::callbacks::EnumVariantValue,
	) -> Option<String> {
		let enum_name = enum_name?;

		// Map enum names to the part of their variant names that needs to be trimmed.
		// When an enum name is not in this map the code below will try to trim the enum name itself.
		let mut map = std::collections::HashMap::new();
		map.insert("SlangMatrixLayoutMode", "SlangMatrixLayout");
		map.insert("SlangCompileTarget", "Slang");

		let trim = map.get(enum_name).unwrap_or(&enum_name);
		let new_variant_name = pascal_case_from_snake_case(original_variant_name);
		let new_variant_name = new_variant_name.trim_start_matches(trim);
		Some(new_variant_name.to_string())
	}

	#[cfg(feature = "serde")]
	fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
		if info.name.starts_with("Slang") && info.kind == bindgen::callbacks::TypeKind::Enum {
			return vec!["serde::Serialize".into(), "serde::Deserialize".into()];
		}
		vec![]
	}
}

/// Converts `snake_case` or `SNAKE_CASE` to `PascalCase`.
/// If the input is already in `PascalCase` it will be returned as is.
fn pascal_case_from_snake_case(snake_case: &str) -> String {
	let mut result = String::new();

	let should_lower = snake_case
		.chars()
		.filter(|c| c.is_alphabetic())
		.all(|c| c.is_uppercase());

	for part in snake_case.split('_') {
		for (i, c) in part.chars().enumerate() {
			if i == 0 {
				result.push(c.to_ascii_uppercase());
			} else if should_lower {
				result.push(c.to_ascii_lowercase());
			} else {
				result.push(c);
			}
		}
	}

	result
}
