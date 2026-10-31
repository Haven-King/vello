<div align="center">

# Vello GPU Shaders

[![Apache 2.0 or MIT license.](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue.svg)](#license)
\
[![Linebender Zulip chat.](https://img.shields.io/badge/Linebender-%23vello-blue?logo=Zulip)](https://xi.zulipchat.com/#narrow/channel/197075-vello) [![GitHub Actions CI status.](https://img.shields.io/github/actions/workflow/status/linebender/vello/ci.yml?logo=github&label=CI)](https://github.com/linebender/vello/actions)

</div>

This crate contains the WESL programs and linked WGSL output, plus optionally
generated GLSL shader programs, used by the Vello GPU renderer.

## Features

- Single source of truth authored as [WESL](https://wesl-lang.dev/) programs.
- A generator that links WESL, then uses
  [naga](https://github.com/gfx-rs/wgpu/tree/trunk/naga) to minify the resulting
  WGSL, its output committed so that a build of `vello_gpu` runs no WESL linker.
- Optional minified GLSL and reflection metadata for WebGL (the `glsl` feature).

## Usage

This crate provides linked WGSL programs and GLSL programs used by Vello GPU, as
two committed modules in `src/generated/`: `wgsl.rs`, and `wgsl_glsl.rs`, which
the `glsl` feature includes in its place.

Whenever the WESL shaders are updated, write the modules again:

```sh
cargo run -p vello_gpu_shaders --features regenerate
```

The crate's tests fail while a committed module differs from what the shaders
give.

To inspect the generated WebGL GLSL and the module used by `vello_gpu`, create
local copies in the `generated_glsl` folder by running:

```sh
cargo run -p vello_gpu_shaders --features regenerate -- --inspect
```

To retain authored identifiers and Naga's readable formatting for debugging, add
`--unminified`:

```sh
cargo run -p vello_gpu_shaders --features regenerate -- --inspect --unminified
```

## Minimum supported Rust Version (MSRV)

This version of Vello GPU Shaders has been verified to compile with **Rust
1.89** and later.

Future versions of Vello GPU Shaders might increase the Rust version
requirement. It will not be treated as a breaking change and as such can even
happen with small patch releases.

<details>
<summary>Click here if compiling fails.</summary>

As time has passed, some of Vello GPU's dependencies could have released
versions with a higher Rust requirement. If you encounter a compilation issue
due to a dependency and don't want to upgrade your Rust toolchain, then you
could downgrade the dependency.

```sh
# Use the problematic dependency's name and version
cargo update -p package_name --precise 0.1.1
```

</details>

## Community

Discussion of Vello GPU development happens in the
[Linebender Zulip](https://xi.zulipchat.com/), specifically the
[#vello channel](https://xi.zulipchat.com/#narrow/channel/197075-vello). All
public content can be read without logging in.

Contributions are welcome by pull request. The [Rust code of conduct] applies.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
