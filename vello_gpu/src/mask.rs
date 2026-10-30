// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! GPU instance data for multiplying layers by their masks.

use crate::schedule::round::MaskOp;
use crate::util::pack_u16_pair;
use bytemuck::{Pod, Zeroable};
use vello_common::geometry::SizeU16;
use vello_common::mask::Mask;

/// Per-instance data for one masked layer region.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuMaskInstance {
    /// Origin of the masked region in the layer texture page, packed as `u16x2`.
    pub(crate) target_texture_origin: u32,
    /// Width and height of the masked region, packed as `u16x2`.
    pub(crate) target_rect_size: u32,
    /// Width and height of the layer texture page, packed as `u16x2`.
    pub(crate) target_texture_size: u32,
    /// Width and height of the mask, packed as `u16x2`.
    pub(crate) mask_size: u32,
    /// Horizontal offset from layer texture pixels to mask pixels, as the bits of an `i32`.
    pub(crate) mask_offset_x: u32,
    /// Vertical offset from layer texture pixels to mask pixels, as the bits of an `i32`.
    pub(crate) mask_offset_y: u32,
}

impl GpuMaskInstance {
    pub(crate) fn new(op: &MaskOp, mask: &Mask, texture_size: SizeU16) -> Self {
        let rect = op.region.texture.rect;

        Self {
            target_texture_origin: pack_u16_pair(rect.x0, rect.y0),
            target_rect_size: pack_u16_pair(rect.width(), rect.height()),
            target_texture_size: pack_u16_pair(texture_size.width(), texture_size.height()),
            mask_size: pack_u16_pair(mask.width(), mask.height()),
            mask_offset_x: op.mask_offset.0.cast_unsigned(),
            mask_offset_y: op.mask_offset.1.cast_unsigned(),
        }
    }
}
