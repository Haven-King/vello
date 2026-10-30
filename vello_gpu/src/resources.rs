// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Persistent renderer resources shared across frames.

use crate::RenderError;
use crate::render::common::IMAGE_PADDING;
#[cfg(feature = "text")]
use crate::text::GlyphAtlasResources;
use alloc::sync::{Arc, Weak};
#[cfg(feature = "text")]
use glifo::GlyphPrepCache;
use hashbrown::HashMap;
use vello_common::encode::EncodedPaint;
use vello_common::geometry::RectU16;
use vello_common::image_cache::ImageCache;
use vello_common::multi_atlas::{AtlasConfig, AtlasId};
use vello_common::paint::{ImageId, ImageSource};
use vello_common::pixmap::Pixmap;

/// Persistent resources required by Vello GPU for rendering.
///
/// A set of resources must only be used with the renderer instance associated with it.
#[derive(Debug)]
pub struct Resources {
    pub(crate) image_cache: ImageCache,
    pub(crate) pixmap_images: PixmapImages,
    #[cfg(feature = "text")]
    pub(crate) glyph_prep_cache: GlyphPrepCache,
    #[cfg(feature = "text")]
    pub(crate) glyph_resources: Option<GlyphAtlasResources>,
}

impl Resources {
    pub(crate) fn new(image_atlas_config: AtlasConfig) -> Self {
        Self {
            image_cache: ImageCache::new_with_config(image_atlas_config),
            pixmap_images: PixmapImages::default(),
            #[cfg(feature = "text")]
            glyph_prep_cache: GlyphPrepCache::default(),
            // Will be initialized lazily.
            #[cfg(feature = "text")]
            glyph_resources: None,
        }
    }
}

/// The atlas images of the [`ImageSource::Pixmap`] paints the renderer drew.
///
/// Each distinct pixmap, identified by its `Arc`, is uploaded the first time a scene draws it.
/// Its atlas region is cleared and freed at the start of a render whose scene doesn't draw it,
/// when the render before didn't draw it either. So a pixmap is kept while it is drawn every
/// frame, and the encoder of the previous render may still be unsubmitted when its region is
/// reused, as long as the renders before it were submitted.
#[derive(Debug, Default)]
pub(crate) struct PixmapImages {
    /// The uploaded pixmaps, keyed by the address of their `Arc`.
    entries: HashMap<usize, PixmapImage>,
}

#[derive(Debug)]
struct PixmapImage {
    /// Keeps the `Arc`'s allocation, so its address can't be reused by another pixmap while
    /// the entry exists. The pixmap itself can't change while this is held either, because
    /// `Arc::get_mut` fails and `Arc::make_mut` clones it.
    #[expect(dead_code, reason = "only held to keep the allocation alive")]
    pixmap: Weak<Pixmap>,
    image_id: ImageId,
    /// Whether the previous render drew the pixmap.
    drawn_last: bool,
    /// Whether the scene being prepared draws the pixmap.
    drawn: bool,
}

impl PixmapImages {
    fn key(pixmap: &Arc<Pixmap>) -> usize {
        Arc::as_ptr(pixmap) as usize
    }

    /// Returns the atlas image of a pixmap uploaded by [`Self::prepare`].
    pub(crate) fn get(&self, pixmap: &Arc<Pixmap>) -> Option<ImageId> {
        self.entries
            .get(&Self::key(pixmap))
            .map(|entry| entry.image_id)
    }

    /// Frees the pixmaps that neither the previous render nor `encoded_paints` draw, then
    /// uploads the pixmaps of `encoded_paints` that aren't in the atlas yet.
    ///
    /// Call at the start of a render, before the scene's paints are encoded.
    pub(crate) fn prepare<T, E: From<RenderError>>(
        &mut self,
        image_cache: &mut ImageCache,
        encoded_paints: &[EncodedPaint],
        backend: &mut T,
        mut clear_region: impl FnMut(&mut T, AtlasId, RectU16) -> Result<(), E>,
        mut upload: impl FnMut(&mut T, &ImageCache, ImageId, &Pixmap) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.entries.is_empty() && !encoded_paints.iter().any(|p| pixmap_of(p).is_some()) {
            return Ok(());
        }

        for pixmap in encoded_paints.iter().filter_map(pixmap_of) {
            if let Some(entry) = self.entries.get_mut(&Self::key(pixmap)) {
                entry.drawn = true;
            }
        }

        let mut result = Ok(());
        self.entries.retain(|_, entry| {
            if result.is_err() || entry.drawn || entry.drawn_last {
                return true;
            }
            if let Some(image) = image_cache.get(entry.image_id) {
                let [x, y] = image.offset;
                let padding = image.padding;
                let region = RectU16::new(
                    x - padding,
                    y - padding,
                    x + image.width + padding,
                    y + image.height + padding,
                );
                if let Err(error) = clear_region(backend, image.atlas_id, region) {
                    result = Err(error);
                    return true;
                }
                image_cache.deallocate(entry.image_id);
            }
            false
        });
        result?;

        for pixmap in encoded_paints.iter().filter_map(pixmap_of) {
            let hashbrown::hash_map::Entry::Vacant(slot) = self.entries.entry(Self::key(pixmap))
            else {
                continue;
            };
            let image_id = image_cache
                .allocate(pixmap.width(), pixmap.height(), IMAGE_PADDING)
                .map_err(RenderError::from)?;
            if let Err(error) = upload(backend, image_cache, image_id, pixmap) {
                // Never written, so the region is still transparent.
                image_cache.deallocate(image_id);
                return Err(error);
            }
            slot.insert(PixmapImage {
                pixmap: Arc::downgrade(pixmap),
                image_id,
                drawn_last: false,
                drawn: true,
            });
        }

        for entry in self.entries.values_mut() {
            entry.drawn_last = entry.drawn;
            entry.drawn = false;
        }

        Ok(())
    }
}

fn pixmap_of(paint: &EncodedPaint) -> Option<&Arc<Pixmap>> {
    match paint {
        EncodedPaint::Image(image) => match &image.source {
            ImageSource::Pixmap(pixmap) => Some(pixmap),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::PixmapImages;
    use crate::RenderError;
    use alloc::sync::Arc;
    use alloc::vec;
    use alloc::vec::Vec;
    use vello_common::encode::{EncodeExt, EncodedPaint};
    use vello_common::geometry::RectU16;
    use vello_common::image_cache::ImageCache;
    use vello_common::kurbo::Affine;
    use vello_common::multi_atlas::{AtlasConfig, AtlasId};
    use vello_common::paint::{Image, ImageId, ImageSource};
    use vello_common::peniko::ImageSampler;
    use vello_common::pixmap::Pixmap;

    #[derive(Default)]
    struct Log {
        uploads: Vec<ImageId>,
        clears: Vec<(AtlasId, RectU16)>,
    }

    fn paints(pixmaps: &[&Arc<Pixmap>]) -> Vec<EncodedPaint> {
        let mut encoded = vec![];
        for pixmap in pixmaps {
            Image {
                image: ImageSource::Pixmap(Arc::clone(pixmap)),
                sampler: ImageSampler::default(),
            }
            .encode_into(&mut encoded, Affine::IDENTITY, None);
        }
        encoded
    }

    fn render(
        images: &mut PixmapImages,
        image_cache: &mut ImageCache,
        log: &mut Log,
        pixmaps: &[&Arc<Pixmap>],
    ) {
        images
            .prepare(
                image_cache,
                &paints(pixmaps),
                log,
                |log, atlas_id, region| {
                    log.clears.push((atlas_id, region));
                    Ok::<(), RenderError>(())
                },
                |log, _, image_id, _| {
                    log.uploads.push(image_id);
                    Ok(())
                },
            )
            .unwrap();
    }

    #[test]
    fn pixmap_is_uploaded_once_and_freed_after_two_frames_without_it() {
        let mut image_cache = ImageCache::new_with_config(AtlasConfig::default());
        let mut images = PixmapImages::default();
        let mut log = Log::default();
        let a = Arc::new(Pixmap::new(4, 3));
        let b = Arc::new(Pixmap::new(2, 2));

        render(&mut images, &mut image_cache, &mut log, &[&a, &a, &b]);
        assert_eq!(log.uploads.len(), 2);
        let a_id = images.get(&a).unwrap();
        assert!(image_cache.get(a_id).is_some());

        // Kept while the previous render drew it.
        render(&mut images, &mut image_cache, &mut log, &[&b]);
        assert!(log.clears.is_empty());
        assert_eq!(images.get(&a), Some(a_id));

        // Drawn again after one frame without it: not uploaded again.
        render(&mut images, &mut image_cache, &mut log, &[&a]);
        assert_eq!(log.uploads.len(), 2);
        assert!(log.clears.is_empty());

        render(&mut images, &mut image_cache, &mut log, &[]);
        render(&mut images, &mut image_cache, &mut log, &[]);
        assert_eq!(log.clears.len(), 2);
        assert!(
            log.clears
                .contains(&(AtlasId::new(0), RectU16::new(0, 0, 4, 3)))
        );
        assert_eq!(images.get(&a), None);
        assert!(image_cache.get(a_id).is_none());
    }

    #[test]
    fn dropped_pixmap_is_freed_and_new_pixmap_is_uploaded() {
        let mut image_cache = ImageCache::new_with_config(AtlasConfig::default());
        let mut images = PixmapImages::default();
        let mut log = Log::default();

        let a = Arc::new(Pixmap::new(4, 4));
        let a_address = Arc::as_ptr(&a);
        render(&mut images, &mut image_cache, &mut log, &[&a]);
        drop(a);

        // The entry holds the allocation, so a new pixmap can't take the old one's address.
        let b = Arc::new(Pixmap::new(4, 4));
        assert_ne!(Arc::as_ptr(&b), a_address);
        render(&mut images, &mut image_cache, &mut log, &[&b]);
        render(&mut images, &mut image_cache, &mut log, &[&b]);
        assert_eq!(log.uploads.len(), 2);
        assert_eq!(log.clears.len(), 1);
        assert_eq!(images.entries.len(), 1);
    }

    #[test]
    fn failed_upload_releases_its_region() {
        let mut image_cache = ImageCache::new_with_config(AtlasConfig::default());
        let mut images = PixmapImages::default();
        let a = Arc::new(Pixmap::new(4, 4));
        let result = images.prepare(
            &mut image_cache,
            &paints(&[&a]),
            &mut (),
            |_, _, _| Ok::<(), RenderError>(()),
            |_, _, image_id, _| Err(RenderError::MissingImage(image_id)),
        );
        assert!(result.is_err());
        assert_eq!(images.get(&a), None);
        let stats = image_cache.atlas_manager().atlas_stats();
        assert!(stats.iter().all(|(_, stats)| stats.allocated_count == 0));
    }
}
