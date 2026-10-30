// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for images drawn from an `ImageSource::Pixmap`, which `vello_gpu` uploads into its image
//! atlas on first use and frees once no frame draws them.

use crate::load_image;
use crate::renderer::Renderer;
use std::f64::consts::PI;
use std::sync::Arc;
use vello_common::kurbo::{Affine, Point, Rect};
use vello_common::paint::{Image, ImageSource, Magnification};
use vello_common::peniko::{Extend, ImageQuality, ImageSampler};
use vello_common::pixmap::Pixmap;
use vello_dev_macros::vello_test;

fn pixmap_image(pixmap: &Arc<Pixmap>, extend: Extend, quality: ImageQuality) -> Image {
    Image {
        image: ImageSource::Pixmap(Arc::clone(pixmap)),
        sampler: ImageSampler {
            x_extend: extend,
            y_extend: extend,
            quality,
            alpha: 1.0,
        },
    }
}

fn fill_with_pixmap(
    ctx: &mut impl Renderer,
    pixmap: &Arc<Pixmap>,
    rect: Rect,
    paint_transform: Affine,
    quality: ImageQuality,
) {
    ctx.set_paint_transform(paint_transform);
    ctx.set_paint(pixmap_image(pixmap, Extend::Pad, quality));
    ctx.fill_rect(&rect);
}

fn scaled(ctx: &mut impl Renderer, quality: ImageQuality) {
    let pixmap = load_image!("rgb_image_10x10");
    fill_with_pixmap(
        ctx,
        &pixmap,
        Rect::new(15.0, 15.0, 85.0, 85.0),
        Affine::translate((15.0, 15.0)) * Affine::scale(7.0),
        quality,
    );
}

#[vello_test]
fn image_pixmap_scaled_nearest(ctx: &mut impl Renderer) {
    scaled(ctx, ImageQuality::Low);
}

#[vello_test]
fn image_pixmap_scaled_bilinear(ctx: &mut impl Renderer) {
    scaled(ctx, ImageQuality::Medium);
}

#[vello_test]
fn image_pixmap_scaled_bicubic(ctx: &mut impl Renderer) {
    scaled(ctx, ImageQuality::High);
}

#[vello_test]
fn image_pixmap_downscaled(ctx: &mut impl Renderer) {
    let pixmap = load_image!("cowboy");
    let scale = 80.0 / f64::from(pixmap.width().max(pixmap.height()));
    fill_with_pixmap(
        ctx,
        &pixmap,
        Rect::new(10.0, 10.0, 90.0, 90.0),
        Affine::translate((10.0, 10.0)) * Affine::scale(scale),
        ImageQuality::Medium,
    );
}

#[vello_test]
fn image_pixmap_rotated(ctx: &mut impl Renderer) {
    let pixmap = load_image!("rgb_image_10x10");
    ctx.set_transform(Affine::rotate_about(PI / 6.0, Point::new(50.0, 50.0)));
    fill_with_pixmap(
        ctx,
        &pixmap,
        Rect::new(20.0, 20.0, 80.0, 80.0),
        Affine::translate((20.0, 20.0)) * Affine::scale(6.0),
        ImageQuality::Medium,
    );
}

#[vello_test]
fn image_pixmap_rotated_paint_transform(ctx: &mut impl Renderer) {
    let pixmap = load_image!("rgb_image_10x10");
    fill_with_pixmap(
        ctx,
        &pixmap,
        Rect::new(10.0, 10.0, 90.0, 90.0),
        Affine::rotate_about(-PI / 5.0, Point::new(50.0, 50.0))
            * Affine::translate((30.0, 30.0))
            * Affine::scale(4.0),
        ImageQuality::Low,
    );
}

fn tiled(ctx: &mut impl Renderer, x_extend: Extend, y_extend: Extend, quality: ImageQuality) {
    let pixmap = load_image!("rgb_image_2x3");
    ctx.set_paint_transform(Affine::translate((45.0, 45.0)) * Affine::scale(4.0));
    ctx.set_paint(Image {
        image: ImageSource::Pixmap(pixmap),
        sampler: ImageSampler {
            x_extend,
            y_extend,
            quality,
            alpha: 1.0,
        },
    });
    ctx.fill_rect(&Rect::new(5.0, 5.0, 95.0, 95.0));
}

#[vello_test]
fn image_pixmap_tiled_pad(ctx: &mut impl Renderer) {
    tiled(ctx, Extend::Pad, Extend::Pad, ImageQuality::Low);
}

#[vello_test]
fn image_pixmap_tiled_repeat(ctx: &mut impl Renderer) {
    tiled(ctx, Extend::Repeat, Extend::Repeat, ImageQuality::Low);
}

#[vello_test]
fn image_pixmap_tiled_reflect(ctx: &mut impl Renderer) {
    tiled(ctx, Extend::Reflect, Extend::Reflect, ImageQuality::Low);
}

// Vello 0.3.0's GPU shader applies the extend mode once before filtering, so bilinear and
// bicubic taps across a repeat or reflect seam differ from the CPU. Uploaded `OpaqueId` images
// do the same. Upstream main samples each tap with its extend mode.
#[vello_test(skip_gpu)]
fn image_pixmap_tiled_repeat_x_reflect_y_bilinear(ctx: &mut impl Renderer) {
    tiled(ctx, Extend::Repeat, Extend::Reflect, ImageQuality::Medium);
}

// Vello 0.3.0's GPU shader applies the extend mode once before filtering, so bilinear and
// bicubic taps across a repeat or reflect seam differ from the CPU. Uploaded `OpaqueId` images
// do the same. Upstream main samples each tap with its extend mode.
#[vello_test(skip_gpu)]
fn image_pixmap_tiled_reflect_x_pad_y_bicubic(ctx: &mut impl Renderer) {
    tiled(ctx, Extend::Reflect, Extend::Pad, ImageQuality::High);
}

#[vello_test]
fn image_pixmap_translucent(ctx: &mut impl Renderer) {
    let opaque = load_image!("rgb_image_10x10");
    let translucent = load_image!("rgba_image_10x10");
    fill_with_pixmap(
        ctx,
        &opaque,
        Rect::new(10.0, 10.0, 60.0, 60.0),
        Affine::translate((10.0, 10.0)) * Affine::scale(5.0),
        ImageQuality::Low,
    );
    fill_with_pixmap(
        ctx,
        &translucent,
        Rect::new(40.0, 40.0, 90.0, 90.0),
        Affine::translate((40.0, 40.0)) * Affine::scale(5.0),
        ImageQuality::Low,
    );

    // A sampler alpha on top of the image's own alpha.
    ctx.set_paint_transform(Affine::translate((40.0, 10.0)) * Affine::scale(5.0));
    ctx.set_paint(Image {
        image: ImageSource::Pixmap(translucent),
        sampler: ImageSampler {
            x_extend: Extend::Pad,
            y_extend: Extend::Pad,
            quality: ImageQuality::Medium,
            alpha: 0.5,
        },
    });
    ctx.fill_rect(&Rect::new(40.0, 10.0, 90.0, 60.0));
}

#[vello_test]
fn image_pixmap_drawn_many_times(ctx: &mut impl Renderer) {
    let pixmap = load_image!("rgba_image_10x10");
    for row in 0..6 {
        for col in 0..6 {
            let x = 4.0 + 16.0 * f64::from(col);
            let y = 4.0 + 16.0 * f64::from(row);
            let scale = 1.0 + 0.1 * f64::from(row * 6 + col);
            let size = 10.0 * scale.min(1.4);
            fill_with_pixmap(
                ctx,
                &pixmap,
                Rect::new(x, y, x + size, y + size),
                Affine::translate((x, y)) * Affine::scale(scale),
                ImageQuality::Medium,
            );
        }
    }
}

#[vello_test]
fn image_pixmap_distinct_pixmaps(ctx: &mut impl Renderer) {
    let pixmaps = [
        load_image!("rgb_image_10x10"),
        load_image!("rgba_image_10x10"),
        load_image!("luma_image_10x10"),
        load_image!("lumaa_image_10x10"),
        load_image!("rgb_image_2x3"),
        load_image!("color_grid_16x16"),
        // The same content in a different `Arc`.
        load_image!("rgb_image_10x10"),
    ];
    for (i, pixmap) in pixmaps.iter().enumerate() {
        let x = 5.0 + 30.0 * (i % 3) as f64;
        let y = 5.0 + 30.0 * (i / 3) as f64;
        // Keeps nearest-neighbour samples off texel boundaries, where CPU and GPU round
        // differently.
        let scale = 24.7 / f64::from(pixmap.width().max(pixmap.height()));
        fill_with_pixmap(
            ctx,
            pixmap,
            Rect::new(x, y, x + 25.0, y + 25.0),
            Affine::translate((x, y)) * Affine::scale(scale),
            ImageQuality::Low,
        );
    }
}

/// Draws a pixmap, then frames without it until its atlas region is freed and reused by other
/// pixmaps. The last frame must not show any of it.
#[vello_test]
fn image_pixmap_dropped_between_frames(ctx: &mut impl Renderer) {
    let rect = Rect::new(10.0, 10.0, 90.0, 90.0);
    let paint_transform = Affine::translate((10.0, 10.0)) * Affine::scale(8.0);

    let dropped = load_image!("rgba_image_10x10");
    let kept = load_image!("rgb_image_2x3");
    fill_with_pixmap(ctx, &dropped, rect, paint_transform, ImageQuality::Low);
    fill_with_pixmap(
        ctx,
        &kept,
        Rect::new(0.0, 0.0, 10.0, 10.0),
        Affine::scale(3.0),
        ImageQuality::Low,
    );
    ctx.flush();
    ctx.render();
    ctx.reset();
    drop(dropped);

    // Frames that don't draw `dropped`, after which it is freed.
    for _ in 0..2 {
        fill_with_pixmap(
            ctx,
            &kept,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Affine::scale(3.0),
            ImageQuality::Low,
        );
        ctx.flush();
        ctx.render();
        ctx.reset();
    }

    // New pixmaps of the same size, which may take over the freed region.
    let replacement = load_image!("rgb_image_10x10");
    fill_with_pixmap(ctx, &replacement, rect, paint_transform, ImageQuality::Low);
    let other = load_image!("lumaa_image_10x10");
    fill_with_pixmap(
        ctx,
        &other,
        Rect::new(60.0, 60.0, 95.0, 95.0),
        Affine::translate((60.0, 60.0)) * Affine::scale(3.5),
        ImageQuality::Low,
    );
    fill_with_pixmap(
        ctx,
        &kept,
        Rect::new(0.0, 0.0, 10.0, 10.0),
        Affine::scale(3.0),
        ImageQuality::Low,
    );
}

/// A pixmap skipped for one frame is drawn again in the next.
#[vello_test(cpu_u8_tolerance = 4)]
fn image_pixmap_skipped_for_one_frame(ctx: &mut impl Renderer) {
    let rect = Rect::new(10.0, 10.0, 90.0, 90.0);
    let paint_transform = Affine::translate((10.0, 10.0)) * Affine::scale(8.0);
    let pixmap = load_image!("rgba_image_10x10");

    fill_with_pixmap(ctx, &pixmap, rect, paint_transform, ImageQuality::Medium);
    ctx.flush();
    ctx.render();
    ctx.reset();

    let other = load_image!("rgb_image_10x10");
    fill_with_pixmap(ctx, &other, rect, paint_transform, ImageQuality::Medium);
    ctx.flush();
    ctx.render();
    ctx.reset();

    fill_with_pixmap(ctx, &pixmap, rect, paint_transform, ImageQuality::Medium);
}

#[vello_test]
fn image_pixmap_with_atlas_image(ctx: &mut impl Renderer) {
    let pixmap = load_image!("rgb_image_10x10");
    let atlas_source = ctx.get_image_source(load_image!("rgba_image_10x10"));
    fill_with_pixmap(
        ctx,
        &pixmap,
        Rect::new(10.0, 10.0, 60.0, 60.0),
        Affine::translate((10.0, 10.0)) * Affine::scale(5.0),
        ImageQuality::Low,
    );
    ctx.set_paint_transform(Affine::translate((40.0, 40.0)) * Affine::scale(5.0));
    ctx.set_paint(Image {
        image: atlas_source,
        sampler: ImageSampler {
            x_extend: Extend::Pad,
            y_extend: Extend::Pad,
            quality: ImageQuality::Low,
            alpha: 1.0,
        },
    });
    ctx.fill_rect(&Rect::new(40.0, 40.0, 90.0, 90.0));
}

// Reflect, as `Extend::Pad` with crisp magnification differs between CPU and GPU on this branch
// for uploaded `OpaqueId` images too.
#[vello_test]
fn image_pixmap_crisp_magnification(ctx: &mut impl Renderer) {
    let pixmap = load_image!("rgb_image_2x3");
    ctx.set_image_magnification(Magnification::Crisp);
    ctx.set_transform(Affine::rotate_about(PI / 12.0, Point::new(50.0, 50.0)));
    ctx.set_paint_transform(Affine::translate((25.0, 15.0)) * Affine::scale(23.3));
    ctx.set_paint(Image {
        image: ImageSource::Pixmap(pixmap),
        sampler: ImageSampler {
            x_extend: Extend::Reflect,
            y_extend: Extend::Reflect,
            quality: ImageQuality::Medium,
            alpha: 1.0,
        },
    });
    ctx.fill_rect(&Rect::new(20.0, 10.0, 75.0, 90.0));
}
