//! The world's signs follow the player's language while it runs. A model's pictures with words on
//! them are swapped for the language's own as the file loads (`i18n::image_override`); this keeps
//! track of which of them went to the GPU as which texture, and on a change of language draws
//! them again in place (the file's own picture for Spanish).
//!
//! Models are noted as they are read (`prefetch::load`): each sign's file and image, its picture
//! as the file has it, and what it shows now (by the hash its upload is cached by). A model read
//! ahead in the old language is put right there. Uploads (`upload_texture_pair`) are matched to
//! the signs by that hash.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use velvet_render::{GltfModel, TextureData3D, TextureHandle};

use super::RenderState;

struct Sign {
    path: PathBuf,
    image: usize,
    width: u32,
    height: u32,
    /// The picture as the file has it (Spanish).
    original: Vec<u8>,
    /// The hash of what it shows now.
    shown: u64,
}

#[derive(Default)]
struct Signs {
    signs: Vec<Sign>,
    /// Uploaded textures and the sign each shows.
    uploaded: Vec<(TextureHandle, usize)>,
}

static SIGNS: Mutex<Option<Signs>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Signs) -> R) -> R {
    let mut guard = SIGNS.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Signs::default))
}

fn hash(rgba: &Vec<u8>) -> u64 {
    // As `texture_cache_key` hashes an albedo.
    let mut hasher = DefaultHasher::new();
    rgba.hash(&mut hasher);
    hasher.finish()
}

/// The sign's picture in the player's language now.
fn in_language(path: &Path, image: usize, width: u32, height: u32, original: &[u8]) -> Vec<u8> {
    demon_queen_dungeon::i18n::image_override(path, image, width, height)
        .map(|rgba| rgba.into_raw())
        .unwrap_or_else(|| original.to_vec())
}

/// A model just read: its signs noted, and put in the player's language if it was read in another.
pub(crate) fn note_model(path: &Path, model: &mut GltfModel) {
    for (texture, &image) in model.texture_sources.iter().enumerate() {
        if !demon_queen_dungeon::i18n::is_sign(path, image) {
            continue;
        }
        let Some(data) = model.textures.get_mut(texture) else {
            continue;
        };
        let original = velvet_render::image_original(path, image)
            .map(|rgba| rgba.into_raw())
            .unwrap_or_else(|| data.rgba.clone());
        let want = in_language(path, image, data.width, data.height, &original);
        if want.len() == data.rgba.len() && want != data.rgba {
            data.rgba = want;
        }
        let shown = hash(&data.rgba);
        with(|s| {
            if !s.signs.iter().any(|g| g.path == path && g.image == image && g.shown == shown) {
                s.signs.push(Sign {
                    path: path.to_path_buf(),
                    image,
                    width: data.width,
                    height: data.height,
                    original,
                    shown,
                });
            }
        });
    }
}

/// An albedo went to the GPU as `handle`: a sign's, when its hash is one.
pub(crate) fn note_upload(albedo_hash: u64, with_normal: bool, handle: TextureHandle) {
    if with_normal {
        return;
    }
    with(|s| {
        if let Some(sign) = s.signs.iter().position(|g| g.shown == albedo_hash) {
            if !s.uploaded.iter().any(|(h, _)| *h == handle) {
                s.uploaded.push((handle, sign));
            }
        }
    });
}

/// A new renderer: the old one's textures are gone.
pub(crate) fn forget_uploads() {
    with(|s| s.uploaded.clear());
}

impl RenderState {
    /// Every sign on the GPU drawn again in the player's language; how many were.
    pub(crate) fn relocalize_signs(&mut self) -> usize {
        let todo: Vec<(TextureHandle, TextureData3D, usize, u64)> = with(|s| {
            s.uploaded
                .iter()
                .filter_map(|&(handle, index)| {
                    let g = s.signs.get(index)?;
                    let rgba = in_language(&g.path, g.image, g.width, g.height, &g.original);
                    let shown = hash(&rgba);
                    (shown != g.shown).then(|| {
                        (
                            handle,
                            TextureData3D {
                                width: g.width,
                                height: g.height,
                                rgba,
                            },
                            index,
                            shown,
                        )
                    })
                })
                .collect()
        });
        let mut done = 0;
        for (handle, texture, index, shown) in todo {
            if self
                .renderer
                .replace_texture(&self.device, &self.queue, handle, &texture)
                .is_ok()
            {
                done += 1;
                with(|s| {
                    if let Some(g) = s.signs.get_mut(index) {
                        g.shown = shown;
                    }
                });
            }
        }
        if done > 0 {
            eprintln!("[i18n] {done} carteles redibujados en el idioma nuevo");
        }
        done
    }
}
