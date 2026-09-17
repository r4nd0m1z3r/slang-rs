<div align="center">

# shader-slang
**Rust bindings for the [Slang](https://github.com/shader-slang/slang/) shader language compiler**

</div>

Supports both the modern compilation and reflection API.

Currently mostly reflects the needs of our own [engine](https://github.com/FloatyMonkey/engine) but contributions are more than welcome.

## Example

```rust
let global_session = slang::GlobalSession::new().unwrap();

let search_path = std::ffi::CString::new("shaders/directory").unwrap();

// All compiler options are available through this builder.
let session_options = slang::CompilerOptions::default()
	.optimization(slang::OptimizationLevel::High)
	.matrix_layout_row(true);

let target_desc = slang::TargetDesc::default()
	.format(slang::CompileTarget::Spirv)
	.profile(global_session.find_profile("glsl_450"));

let targets = [target_desc];
let search_paths = [search_path.as_ptr()];

let session_desc = slang::SessionDesc::default()
	.targets(&targets)
	.search_paths(&search_paths)
	.options(&session_options);

let session = global_session.create_session(&session_desc).unwrap();
let module = session.load_module("filename.slang").unwrap();
let entry_point = module.find_entry_point_by_name("main").unwrap();

let program = session
	.create_composite_component_type(&[module.into(), entry_point.into()])
	.unwrap();

let linked_program = program.link().unwrap();

// Entry point to the reflection API.
let reflection = linked_program.layout(0).unwrap();

let shader_bytecode = linked_program.entry_point_code(0, 0).unwrap();
```

## Installation

Add `shader-slang` to the `[dependencies]` section of your `Cargo.toml`.

By default this library links against a Slang installation provided by the environment. An easy way is by installing the [LunarG Vulkan SDK](https://vulkan.lunarg.com) which comes bundled with the Slang compiler. During installation `VULKAN_SDK` is added to the `PATH` and automatically picked up by this library.

Alternatively, download Slang from their [releases page](https://github.com/shader-slang/slang/releases) and manually set the `SLANG_DIR` environment variable to the path of your Slang directory. Copy `slang.dll` to your executable's directory. To compile to DXIL bytecode, also copy `dxil.dll` and `dxcompiler.dll` from the [Microsoft DirectXShaderCompiler](https://github.com/microsoft/DirectXShaderCompiler/releases) to your executable's directory.

To specify the `include` and `lib` directories separately, set the `SLANG_INCLUDE_DIR` and `SLANG_LIB_DIR` environment variables.

### Building Slang from source

Enable the `static` feature to build Slang from source and link it into your binary statically, so that neither a Slang installation nor a runtime library search path is needed:

```toml
[dependencies]
shader-slang = { version = "0.1.0", features = ["static"] }
```

Point `SLANG_SOURCE_DIR` at any checkout of the matching Slang version. Building requires CMake 3.22+, a C++17 compiler, and libclang for the bindgen step. Set `CMAKE_GENERATOR=Ninja` and/or `CMAKE_BUILD_PARALLEL_LEVEL` to speed up the first build, which takes several minutes; subsequent builds reuse the CMake build tree in `target/<profile>/build/shader-slang-sys-*/out/slang-<version>`.

Static builds leave out the parts of Slang that need external toolchains or downloads:

- No LLVM, so CPU targets (`CompileTarget::Cpp` etc.) and host-callable entry points are unavailable.
- No DXIL by default. Set `SLANG_ENABLE_DXIL=1` at build time to enable it; this downloads DXC during the CMake configure step and, as for the default installation mode, requires `dxil.dll` and `dxcompiler.dll` at runtime.
- No glslang pass-through, no `slang-rt`, and no tools or tests.

SPIR-V, HLSL, GLSL, Metal and WGSL targets, the core module and the full reflection API are included.

## Credits

Maintained by Lauro Oyen ([@laurooyen](https://github.com/laurooyen)).

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
