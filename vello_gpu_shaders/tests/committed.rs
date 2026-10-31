// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The committed shader modules are the WESL sources linked, minified and compiled as they are
//! now: a change to `shaders/` is followed by `cargo run -p vello_gpu_shaders --features
//! regenerate`.

#![allow(
    warnings,
    reason = "the shader pipeline's modules, compiled into this test"
)]

#[path = "../src/compile.rs"]
mod compile;
#[path = "../src/generate.rs"]
mod generate;
#[path = "../src/lint/mod.rs"]
mod lint;
#[path = "../src/minify.rs"]
mod minify;
#[path = "../src/types.rs"]
mod types;

#[test]
fn committed_modules_are_the_wesl_sources_linked() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let (wgsl, wgsl_glsl) = generate::modules(manifest_dir);
    let (wgsl_path, wgsl_glsl_path) = generate::module_paths(manifest_dir);
    let stale = "is stale: run `cargo run -p vello_gpu_shaders --features regenerate`";
    assert!(
        std::fs::read_to_string(&wgsl_path).unwrap() == wgsl,
        "{} {stale}",
        wgsl_path.display()
    );
    assert!(
        std::fs::read_to_string(&wgsl_glsl_path).unwrap() == wgsl_glsl,
        "{} {stale}",
        wgsl_glsl_path.display()
    );
}
