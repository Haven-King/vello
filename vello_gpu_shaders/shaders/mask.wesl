// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Multiply a rendered layer atlas region by a mask.
//!
//! The pipeline blends with `dst * src`, so the fragment shader only outputs the mask value.

import package::helpers::geometry::{pixel_to_ndc, quad_corner};
import package::helpers::util::unpack_u16_pair;

struct MaskInstance {
    @location(0) target_texture_origin: u32,
    @location(1) target_rect_size: u32,
    @location(2) target_texture_size: u32,
    @location(3) mask_size: u32,
    // Signed offsets from layer texture pixels to mask pixels, stored as `u32`.
    @location(4) mask_offset_x: u32,
    @location(5) mask_offset_y: u32,
}

struct VertexOutput {
    @location(0) mask_xy: vec2<f32>,
    @interpolate(flat)
    @location(1) mask_size: u32,
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0)
var mask_texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: MaskInstance) -> VertexOutput {
    let target_texture_origin = unpack_u16_pair(instance.target_texture_origin);
    let target_rect_size = unpack_u16_pair(instance.target_rect_size);
    let target_texture_size = unpack_u16_pair(instance.target_texture_size);
    let mask_offset = vec2<i32>(
            bitcast<i32>(instance.mask_offset_x),
            bitcast<i32>(instance.mask_offset_y),
        );

    let target_xy = vec2<f32>(target_texture_origin)
        + quad_corner(vertex_index) * vec2<f32>(target_rect_size);

    var out: VertexOutput;
    out.mask_xy = target_xy + vec2<f32>(mask_offset);
    out.mask_size = instance.mask_size;

    let ndc = pixel_to_ndc(target_xy, vec2<f32>(target_texture_size));
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(
    @location(0) mask_xy: vec2<f32>,
    @interpolate(flat)
    @location(1) packed_mask_size: u32,
) -> @location(0) vec4<f32> {
    let mask_size = vec2<i32>(unpack_u16_pair(packed_mask_size));
    let xy = vec2<i32>(floor(mask_xy));

    // Like Vello CPU, a mask layer keeps nothing outside of its mask.
    var value = 0.0;
    if xy.x >= 0 && xy.y >= 0 && xy.x < mask_size.x && xy.y < mask_size.y {
        value = textureLoad(mask_texture, xy, 0).r;
    }

    return vec4<f32>(value);
}
