//! Sprite batching and sort.

use crate::sprite::{Sprite, SpriteInstance};
use crate::stats::RenderStats;
use crate::texture::{TextureId, TextureRegion, TextureStore};
use velvet_math::{Color, Rect, Vec2};

/// A single draw item before GPU upload.
#[derive(Debug, Clone)]
pub struct DrawCommand {
    /// Texture to bind.
    pub texture: TextureId,
    /// Instance payload.
    pub instance: SpriteInstance,
    /// Sort key: layer then texture.
    pub sort_key: u64,
}

/// Accumulates sprites for a frame and produces sorted draw lists.
#[derive(Debug, Default)]
pub struct SpriteBatch {
    commands: Vec<DrawCommand>,
    /// Stats for the current build.
    pub stats: RenderStats,
}

impl SpriteBatch {
    /// Create empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear for a new frame.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.stats = RenderStats::default();
    }

    /// Number of sprites queued.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Push a sprite using texture store for size/UV resolution.
    pub fn push_sprite(&mut self, sprite: &Sprite, textures: &TextureStore) {
        let (tex_w, tex_h) = textures
            .get(sprite.texture)
            .map(|i| (i.width as f32, i.height as f32))
            .unwrap_or((1.0, 1.0));
        let region = sprite.region.unwrap_or(TextureRegion::full(tex_w, tex_h));
        let size = sprite
            .size
            .unwrap_or(Vec2::new(region.width, region.height));
        let uv = region.to_uv(tex_w, tex_h);
        let instance = SpriteInstance::from_parts(
            sprite.transform,
            size,
            sprite.anchor,
            uv,
            sprite.tint,
            sprite.z,
            sprite.flip,
        );
        let sort_key = pack_sort_key(sprite.z, sprite.texture);
        self.commands.push(DrawCommand {
            texture: sprite.texture,
            instance,
            sort_key,
        });
        self.stats.sprites_submitted += 1;
    }

    /// Push raw colored quad without texture lookup (uses white UV).
    pub fn push_colored_quad(
        &mut self,
        texture: TextureId,
        transform: velvet_math::Transform2D,
        size: Vec2,
        tint: Color,
        z: f32,
    ) {
        let instance = SpriteInstance::from_parts(
            transform,
            size,
            Vec2::new(0.5, 0.5),
            Rect::from_pos_size(Vec2::ZERO, Vec2::ONE),
            tint,
            z,
            Default::default(),
        );
        self.commands.push(DrawCommand {
            texture,
            instance,
            sort_key: pack_sort_key(z, texture),
        });
        self.stats.sprites_submitted += 1;
    }

    /// Sort commands by z then texture id (stable batching).
    pub fn sort(&mut self) {
        self.commands.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    }

    /// Iterate sorted commands, grouping consecutive same-texture runs.
    pub fn batches(&self) -> BatchIter<'_> {
        BatchIter {
            commands: &self.commands,
            index: 0,
        }
    }

    /// All commands.
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Estimate draw calls after sort (one per texture run).
    pub fn estimate_draw_calls(&self) -> u32 {
        let mut calls = 0u32;
        let mut last: Option<TextureId> = None;
        for c in &self.commands {
            if last != Some(c.texture) {
                calls += 1;
                last = Some(c.texture);
            }
        }
        calls
    }

    /// Push instances from a particle batch as colored quads (CPU path).
    pub fn push_particles(&mut self, particles: &crate::particles::ParticleBatch) {
        let tex = particles.texture;
        for p in particles.particles() {
            if !p.alive() {
                continue;
            }
            let size = Vec2::splat(p.size);
            let mut transform = velvet_math::Transform2D::from_translation(p.position);
            transform.rotation = p.rotation;
            self.push_colored_quad(tex, transform, size, p.faded_color(), 0.0);
            self.stats.record_particles(1);
        }
    }

    /// Merge another batch's commands (does not re-sort).
    pub fn extend_from(&mut self, other: &SpriteBatch) {
        self.commands.extend(other.commands.iter().cloned());
        self.stats.sprites_submitted = self
            .stats
            .sprites_submitted
            .saturating_add(other.stats.sprites_submitted);
    }

    /// Push with an explicit packed sort key (multi-camera / y-sort pipelines).
    pub fn push_with_key(&mut self, texture: TextureId, instance: SpriteInstance, sort_key: u64) {
        self.commands.push(DrawCommand {
            texture,
            instance,
            sort_key,
        });
        self.stats.sprites_submitted += 1;
    }

    /// Sort using expanded keys already stored on commands.
    pub fn sort_stable(&mut self) {
        self.commands.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    }
}

fn pack_sort_key(z: f32, texture: TextureId) -> u64 {
    crate::sort_key::pack_z_texture(z, texture)
}

/// A consecutive run of instances sharing a texture.
#[derive(Debug)]
pub struct TextureBatch<'a> {
    /// Texture.
    pub texture: TextureId,
    /// Instances.
    pub instances: &'a [DrawCommand],
}

/// Iterator over texture batches.
pub struct BatchIter<'a> {
    commands: &'a [DrawCommand],
    index: usize,
}

impl<'a> Iterator for BatchIter<'a> {
    type Item = TextureBatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.commands.len() {
            return None;
        }
        let tex = self.commands[self.index].texture;
        let start = self.index;
        while self.index < self.commands.len() && self.commands[self.index].texture == tex {
            self.index += 1;
        }
        Some(TextureBatch {
            texture: tex,
            instances: &self.commands[start..self.index],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::TextureInfo;
    use velvet_math::Transform2D;

    #[test]
    fn batches_reduce_draw_calls() {
        let mut store = TextureStore::new();
        let t1 = TextureId::allocate();
        let t2 = TextureId::allocate();
        store.register(TextureInfo {
            id: t1,
            width: 8,
            height: 8,
            label: "a".into(),
        });
        store.register(TextureInfo {
            id: t2,
            width: 8,
            height: 8,
            label: "b".into(),
        });

        let mut batch = SpriteBatch::new();
        for i in 0..10 {
            let tex = if i % 2 == 0 { t1 } else { t2 };
            batch.push_sprite(&Sprite::at(tex, Vec2::ZERO, Vec2::ONE), &store);
        }
        // Unsorted: many runs. Sorted: 2 runs.
        assert!(batch.estimate_draw_calls() >= 2);
        batch.sort();
        assert_eq!(batch.estimate_draw_calls(), 2);
        assert_eq!(batch.batches().count(), 2);
    }

    #[test]
    fn z_order_sort() {
        let mut batch = SpriteBatch::new();
        let t = TextureId::allocate();
        batch.push_colored_quad(t, Transform2D::IDENTITY, Vec2::ONE, Color::WHITE, 5.0);
        batch.push_colored_quad(t, Transform2D::IDENTITY, Vec2::ONE, Color::WHITE, 1.0);
        batch.sort();
        assert!(batch.commands()[0].instance.z() <= batch.commands()[1].instance.z());
    }
}
