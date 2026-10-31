// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! WESL shader sources linked to WGSL and optionally compiled to GLSL for `vello_gpu`.

#[cfg(feature = "glsl")]
mod compile;
#[cfg(feature = "glsl")]
mod lint;
#[cfg(any(test, feature = "glsl"))]
mod minify;
#[cfg(feature = "glsl")]
mod types;

// Written by `cargo run -p vello_gpu_shaders --features regenerate` from `shaders/` and committed,
// so that no build links WESL; `tests/committed.rs` holds them to the sources.
#[cfg(not(feature = "glsl"))]
include!("generated/wgsl.rs");
#[cfg(feature = "glsl")]
include!("generated/wgsl_glsl.rs");

#[cfg(all(test, feature = "glsl"))]
mod tests {
    use naga::front::wgsl;

    use crate::lint::lint;

    #[test]
    fn every_shipped_shader_passes_the_lint() {
        assert!(
            !crate::wgsl::ALL.is_empty(),
            "expected at least one linked WESL shader"
        );
        for &(name, source) in crate::wgsl::ALL {
            let module = wgsl::parse_str(source).expect("linked WGSL parses");
            lint(name, &module);
        }
    }
}
