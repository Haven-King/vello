// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The shader pipeline that writes the committed modules in `src/generated/`.
//!
//! Every top-level file in `shaders` is treated as a root module. WESL links each root with its
//! imports. The linked WGSL is parsed into Naga IR, named declarations are deterministically
//! shortened while renderer-facing entry-point names remain unchanged, and redundant comments and
//! whitespace are removed from the serialized WGSL (unless `minified` is false, for inspection).
//!
//! `wgsl.rs` embeds the WGSL; `wgsl_glsl.rs`, which the `glsl` feature includes in its place, adds
//! vertex and fragment GLSL plus reflection metadata. Global names are recorded before renaming so
//! that this metadata continues to expose the resource names authored in WESL.
//!
//! The modules are committed so that building `vello_gpu` needs no WESL linker or Naga at build
//! time. `cargo run -p vello_gpu_shaders --features regenerate` writes them again after a change
//! to `shaders/`, and the `committed_modules_are_the_wesl_sources_linked` test holds them to it.
//!
//! ```text
//!                                      |-> renamed/minified WGSL -|
//! shaders/*.wesl -> linked WGSL -------|                          |-> wgsl.rs
//!                                                                 |-> wgsl_glsl.rs (+ GLSL
//!                                                                 |   and reflection)
//! ```

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use wesl::Wesl;

/// The committed modules' folder and the WESL sources', under the crate's manifest folder.
pub(crate) const GENERATED: &str = "src/generated";
pub(crate) const SHADERS: &str = "shaders";

/// The committed module without GLSL, and the one with it, as they should read.
pub(crate) fn modules(manifest_dir: &Path) -> (String, String) {
    let infos = load_shader_infos(&manifest_dir.join(SHADERS), true);
    (
        generate_compiled_shaders_module(&infos, false, true),
        generate_compiled_shaders_module(&infos, true, true),
    )
}

/// The module with GLSL, its WGSL unminified, for inspection.
pub(crate) fn unminified_module(manifest_dir: &Path) -> String {
    let infos = load_shader_infos(&manifest_dir.join(SHADERS), false);
    generate_compiled_shaders_module(&infos, true, false)
}

/// Where each committed module lives.
pub(crate) fn module_paths(manifest_dir: &Path) -> (PathBuf, PathBuf) {
    let dir = manifest_dir.join(GENERATED);
    (dir.join("wgsl.rs"), dir.join("wgsl_glsl.rs"))
}

struct ShaderInfo {
    name: String,
    wgsl_source: String,
    original_global_names: BTreeMap<String, String>,
}

fn load_shader_infos(shader_dir: &Path, minified: bool) -> Vec<ShaderInfo> {
    let shader_names = load_shader_names(shader_dir);
    let mut compiler = Wesl::new(shader_dir);

    compiler.use_stripping(true);

    shader_names
        .into_iter()
        .map(|name| compile_shader(&compiler, name, minified))
        .collect()
}

fn load_shader_names(shader_dir: &Path) -> Vec<String> {
    let mut shader_names = fs::read_dir(shader_dir)
        .expect("Unable to discover WESL shaders")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? == "wesl" {
                Some(path.file_stem()?.to_str()?.to_owned())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    shader_names.sort();
    shader_names
}

fn compile_shader<R: wesl::Resolver>(
    compiler: &Wesl<R>,
    name: String,
    minified: bool,
) -> ShaderInfo {
    let module_path = format!("package::{name}")
        .parse()
        .expect("generated WESL module path should be valid");
    let linked_wgsl = compiler
        .compile(&module_path)
        .unwrap_or_else(|error| panic!("Unable to compile `{name}.wesl`: {error}"))
        .to_string();

    if minified {
        let minified = crate::minify::minify_wgsl(&linked_wgsl);
        ShaderInfo {
            name,
            wgsl_source: minified.source,
            original_global_names: minified.original_global_names,
        }
    } else {
        ShaderInfo {
            name,
            wgsl_source: linked_wgsl,
            original_global_names: BTreeMap::new(),
        }
    }
}

fn generate_compiled_shaders_module(
    shader_infos: &[ShaderInfo],
    glsl: bool,
    minified: bool,
) -> String {
    let mut buf = String::new();
    writeln!(
        buf,
        "// Generated code by `vello_gpu_shaders` - DO NOT EDIT"
    )
    .unwrap();

    writeln!(buf, "/// WGSL shader sources linked from WESL modules.").unwrap();

    writeln!(buf, "pub mod wgsl {{").unwrap();
    for shader_info in shader_infos {
        generate_wgsl_shader_module(&mut buf, shader_info).unwrap();
    }
    writeln!(
        buf,
        "    /// All linked WGSL shader sources, keyed by WESL root module name."
    )
    .unwrap();
    writeln!(buf, "    pub const ALL: &[(&str, &str)] = &[").unwrap();
    for shader_info in shader_infos {
        let const_name = shader_info.name.to_uppercase();
        writeln!(buf, "        (\"{}\", {const_name}),", shader_info.name).unwrap();
    }
    writeln!(buf, "    ];").unwrap();
    writeln!(buf, "}}").unwrap();

    // Implementation for creating a CompiledGlsl struct per shader assuming the standard entry
    // names of `vs_main` and `fs_main`.
    if glsl {
        writeln!(
            buf,
            "/// Build-time GLSL shaders derived from linked WESL modules."
        )
        .unwrap();

        for shader_info in shader_infos {
            let shader = crate::compile::compile_wgsl_shader(
                &shader_info.wgsl_source,
                &shader_info.name,
                "vs_main",
                "fs_main",
                &shader_info.original_global_names,
                minified,
            );
            let generated_code = shader.to_generated_code(&shader_info.name);
            writeln!(buf, "{generated_code}").unwrap();
        }
    }

    buf
}

fn generate_wgsl_shader_module(buf: &mut String, shader_info: &ShaderInfo) -> std::fmt::Result {
    let const_name = shader_info.name.to_uppercase();
    writeln!(
        buf,
        "    /// Linked WGSL source for `{}.wesl`.",
        shader_info.name
    )?;
    writeln!(
        buf,
        "    pub const {const_name}: &str = r###\"{}\"###;",
        shader_info.wgsl_source
    )?;

    Ok(())
}
