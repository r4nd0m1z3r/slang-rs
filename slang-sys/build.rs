extern crate bindgen;

use std::env;
use std::path::{Path, PathBuf};

const SLANG_SOURCE_VERSION: &str = "2026.17.1";

#[cfg(feature = "static")]
const SLANG_SOURCE_URL: &str =
	"https://github.com/shader-slang/slang/archive/refs/tags/v2026.17.1.tar.gz";

const SLANG_EXTERNAL_DEPENDENCIES: [(&str, &str); 8] = [
	("cmark", "CMakeLists.txt"),
	("fast_float", "include/fast_float/fast_float.h"),
	("lua", "onelua.c"),
	("lz4", "build/cmake/CMakeLists.txt"),
	("miniz", "CMakeLists.txt"),
	(
		"spirv-headers",
		"include/spirv/unified1/spirv.core.grammar.json",
	),
	("unordered_dense", "include/ankerl/unordered_dense.h"),
	("vulkan", "CMakeLists.txt"),
];

#[cfg(feature = "static")]
struct Archive(PathBuf);
#[cfg(feature = "static")]
impl Archive {
	fn extract(&self, destination: &Path) {
		let file_name = destination
			.file_name()
			.expect("Destination has no file name.")
			.to_string_lossy();
		let staging_dir = destination.with_file_name(format!("{file_name}.extracting"));

		let _ = std::fs::remove_dir_all(&staging_dir);
		std::fs::create_dir_all(&staging_dir)
			.unwrap_or_else(|err| panic!("Couldn't create {}: {err}", staging_dir.display()));

		let file = std::fs::File::open(&self.0)
			.unwrap_or_else(|err| panic!("Couldn't open {}: {err}", self.0.display()));
		let decoder = flate2::read::GzDecoder::new(file);
		let mut archive = tar::Archive::new(decoder);
		archive
			.unpack(&staging_dir)
			.unwrap_or_else(|err| panic!("Couldn't extract {}: {err}", self.0.display()));

		let mut entries = std::fs::read_dir(&staging_dir)
			.unwrap_or_else(|err| panic!("Couldn't read {}: {err}", staging_dir.display()));

		let top_level = entries
			.next()
			.unwrap_or_else(|| panic!("Archive {} is empty.", self.0.display()))
			.unwrap_or_else(|err| panic!("Couldn't read {}: {err}", staging_dir.display()))
			.path();

		if entries.next().is_some() {
			panic!(
				"Archive {} contains more than one top-level entry.",
				self.0.display()
			);
		}

		let _ = std::fs::remove_dir_all(destination);
		std::fs::rename(&top_level, destination).unwrap_or_else(|err| {
			panic!(
				"Couldn't move {} to {}: {err}",
				top_level.display(),
				destination.display()
			)
		});
		let _ = std::fs::remove_dir_all(&staging_dir);
	}
}

#[cfg(feature = "static")]
struct DependencyArchive {
	name: &'static str,
	url: &'static str,
	version: &'static str,
}
#[cfg(feature = "static")]
impl DependencyArchive {
	fn tarball_url(&self) -> String {
		format!(
			"https://github.com/{}/archive/{}.tar.gz",
			self.url, self.version
		)
	}

	fn download(&self, archives_dir: &Path) -> Archive {
		let archive_path = archives_dir.join(format!("{}.tar.gz", self.name));

		if archive_path
			.metadata()
			.is_ok_and(|metadata| metadata.len() > 0)
		{
			return Archive(archive_path);
		}

		let tarball_url = self.tarball_url();

		println!("cargo:warning=Downloading {tarball_url}");

		let status = std::process::Command::new("curl")
			.args(["-L", "-f", "-sS", "--retry", "3", "-o"])
			.arg(&archive_path)
			.arg(&tarball_url)
			.status()
			.unwrap_or_else(|err| {
				let _ = std::fs::remove_file(&archive_path);

				if err.kind() == std::io::ErrorKind::NotFound {
					panic!(
						"`curl` is required to download the Slang source. Install curl, initialize the \
					slang-src submodule, or point SLANG_SOURCE_DIR at a checkout."
					);
				}

				panic!("Couldn't run curl: {err}");
			});

		if !status.success() {
			let _ = std::fs::remove_file(&archive_path);
			panic!("Downloading {tarball_url} failed: {status}.");
		}

		Archive(archive_path)
	}
}

#[cfg(feature = "static")]
const SLANG_DEPENDENCY_ARCHIVES: [DependencyArchive; 8] = [
	DependencyArchive {
		name: "cmark",
		url: "swiftlang/swift-cmark",
		version: "924936d0427cb25a61169739a7660230bffa6ea6",
	},
	DependencyArchive {
		name: "fast_float",
		url: "fastfloat/fast_float",
		version: "e0b53eaf63c6d00e0725788ef1dbb759aa321d79",
	},
	DependencyArchive {
		name: "lua",
		url: "lua/lua",
		version: "3fe7be956f23385aa1950dc31e2f25127ccfc0ea",
	},
	DependencyArchive {
		name: "lz4",
		url: "lz4/lz4",
		version: "7f71be01f2c6f6b2c3cddb67eef8c38eab12ffd4",
	},
	DependencyArchive {
		name: "miniz",
		url: "richgel999/miniz",
		version: "6ef6c68f4fcbb8287aa8edf9c6670804932f41c6",
	},
	DependencyArchive {
		name: "spirv-headers",
		url: "KhronosGroup/SPIRV-Headers",
		version: "496543121ce6419f23d6fa5d7194ba66c36212d2",
	},
	DependencyArchive {
		name: "unordered_dense",
		url: "martinus/unordered_dense",
		version: "73f3cbb237e84d483afafc743f1f14ec53e12314",
	},
	DependencyArchive {
		name: "vulkan",
		url: "KhronosGroup/Vulkan-Headers",
		version: "387259ecf4b0fe0cfac161b1d0b0a74e42796710",
	},
];

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

fn static_build_include_dir() -> (PathBuf, Vec<PathBuf>) {
	let mut extra_include_dirs = Vec::new();
	let source_dir = slang_source_dir();
	let build_dir = compile_slang(&source_dir);

	link_slang_static(&build_dir);

	if let Some(dir) = find_file(&build_dir, "slang-tag-version.h")
		.and_then(|path| path.parent().map(Path::to_path_buf))
	{
		extra_include_dirs.push(dir);
	}

	(source_dir.join("include"), extra_include_dirs)
}

fn dylib_build_include_dir() -> (PathBuf, Vec<PathBuf>) {
	let (include_dir, lib_dir) = slang_installation();

	if !lib_dir.as_os_str().is_empty() {
		println!("cargo:rustc-link-search=native={}", lib_dir.display());
	}

	println!("cargo:rustc-link-lib=dylib=slang");

	(include_dir, Vec::new())
}

fn main() {
	println!("cargo:rerun-if-env-changed=SLANG_DIR");
	println!("cargo:rerun-if-env-changed=SLANG_INCLUDE_DIR");
	println!("cargo:rerun-if-env-changed=SLANG_LIB_DIR");
	println!("cargo:rerun-if-env-changed=VULKAN_SDK");
	println!("cargo:rerun-if-env-changed=SLANG_SOURCE_DIR");
	println!("cargo:rerun-if-env-changed=SLANG_ENABLE_DXIL");

	let static_build = env::var("CARGO_FEATURE_STATIC").is_ok();

	let (include_dir, extra_include_dirs) = if static_build {
		static_build_include_dir()
	} else {
		dylib_build_include_dir()
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

	if cfg!(feature = "static") {
		return download_slang_source();
	} else {
		panic!(
			"Couldn't find the Slang {SLANG_SOURCE_VERSION} source tree at slang-sys/slang-src."
		);
	}
}

fn missing_external_dependencies(source_dir: &Path) -> Vec<&'static str> {
	SLANG_EXTERNAL_DEPENDENCIES
		.iter()
		.filter(|(name, marker)| {
			!source_dir
				.join("external")
				.join(name)
				.join(marker)
				.is_file()
		})
		.map(|(name, _)| *name)
		.collect()
}

#[cfg(feature = "static")]
fn is_complete_source(source_dir: &Path) -> bool {
	source_dir.join("include/slang.h").is_file()
		&& missing_external_dependencies(source_dir).is_empty()
}

fn check_slang_source(source_dir: &Path) {
	let missing = missing_external_dependencies(source_dir);

	if missing.is_empty() {
		return;
	}

	let init_args = missing
		.iter()
		.map(|name| format!("external/{name}"))
		.collect::<Vec<_>>()
		.join(" ");

	panic!(
		"The Slang source tree at {} is missing its external dependencies {missing:?}. Initialize them:\n\
		\n\tgit -C {} submodule update --init --depth 1 {init_args}",
		source_dir.display(),
		source_dir.display(),
	);
}

#[cfg(feature = "static")]
fn download_slang_source() -> PathBuf {
	let cache_dir = slang_cache_dir();
	let source_dir = cache_dir.join("src");

	if !is_complete_source(&source_dir) {
		let _ = std::fs::remove_dir_all(&source_dir);

		let archives_dir = cache_dir.join("archives");
		std::fs::create_dir_all(&archives_dir)
			.unwrap_or_else(|err| panic!("Couldn't create {}: {err}", archives_dir.display()));

		println!(
			"cargo:warning=Downloading Slang {SLANG_SOURCE_VERSION} sources into {}",
			cache_dir.display()
		);

		DependencyArchive {
			name: "slang",
			url: SLANG_SOURCE_URL,
			version: SLANG_SOURCE_VERSION,
		}
		.download(&archives_dir)
		.extract(&source_dir);

		for dependency in SLANG_DEPENDENCY_ARCHIVES {
			dependency
				.download(&archives_dir)
				.extract(&source_dir.join("external").join(dependency.name));
		}
	}

	check_slang_source(&source_dir);

	source_dir
}

#[cfg(feature = "static")]
fn slang_cache_dir() -> PathBuf {
	let cargo_home = env::var_os("CARGO_HOME")
		.map(PathBuf::from)
		.or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
		.or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")));

	match cargo_home {
		Some(cargo_home) => cargo_home.join("shader-slang").join(SLANG_SOURCE_VERSION),
		None => PathBuf::from(env::var("OUT_DIR").expect("Couldn't determine output directory."))
			.join("slang-download"),
	}
}

fn compile_slang(source_dir: &Path) -> PathBuf {
	let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Couldn't determine output directory."));
	let destination = out_dir.join(format!("slang-{SLANG_SOURCE_VERSION}"));

	let source_dir = source_dir
		.canonicalize()
		.unwrap_or_else(|_| source_dir.to_path_buf());

	// Switching between the submodule and a downloaded copy has to start from a fresh
	// build tree too, stale object files would be mixed into the archives.
	let build_dir = destination.join("build");
	let cache = std::fs::read_to_string(build_dir.join("CMakeCache.txt"));

	if let Ok(cache) = cache {
		if !cache.contains(&format!(
			"CMAKE_HOME_DIRECTORY:INTERNAL={}",
			source_dir.display()
		)) {
			let _ = std::fs::remove_dir_all(&build_dir);
		}
	}

	cmake::Config::new(&source_dir)
		// Build the compiler library as a single static archive. Everything else is
		// disabled so that no tools, tests, examples, submodules beyond the
		// dependencies checked above, or downloads are needed.
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
