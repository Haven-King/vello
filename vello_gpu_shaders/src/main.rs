// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Writes the committed shader modules in `src/generated/` again from the WESL sources.
//!
//! `cargo run -p vello_gpu_shaders --features regenerate` rewrites them. With `-- --inspect`, it
//! writes local copies of the WebGL GLSL and the module into `generated_glsl/` instead, and with
//! `-- --inspect --unminified` keeps the authored identifiers and Naga's readable formatting.

#![allow(
    warnings,
    reason = "the shader pipeline's modules, shared with the library's tests"
)]

mod compile;
mod generate;
mod lint;
mod minify;
mod types;

fn main() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|a| a == "--inspect") {
        let (wgsl, wgsl_glsl) = generate::modules(&manifest_dir);
        let (wgsl_path, wgsl_glsl_path) = generate::module_paths(&manifest_dir);
        std::fs::create_dir_all(wgsl_path.parent().unwrap()).unwrap();
        std::fs::write(&wgsl_path, wgsl).unwrap();
        std::fs::write(&wgsl_glsl_path, wgsl_glsl).unwrap();
        println!(
            "wrote {} and {}",
            wgsl_path.display(),
            wgsl_glsl_path.display()
        );
        return;
    }

    let minified = !args.iter().any(|a| a == "--unminified");
    let module = if !minified {
        generate::unminified_module(&manifest_dir)
    } else {
        generate::modules(&manifest_dir).1
    };
    let output_dir = manifest_dir.join("generated_glsl");
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(output_dir.join("compiled_shaders.rs"), module).unwrap();
    for &(name, wgsl_source) in vello_gpu_shaders::wgsl::ALL {
        let shader = compile::compile_wgsl_shader(
            wgsl_source,
            name,
            "vs_main",
            "fs_main",
            &std::collections::BTreeMap::default(),
            minified,
        );
        std::fs::write(
            output_dir.join(format!("{name}.vert.glsl")),
            shader.vertex.source,
        )
        .unwrap();
        std::fs::write(
            output_dir.join(format!("{name}.frag.glsl")),
            shader.fragment.source,
        )
        .unwrap();
    }
    println!("wrote {}", output_dir.display());
}
