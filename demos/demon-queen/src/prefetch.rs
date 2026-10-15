//! Models read ahead, on other threads.
//!
//! Reading a zone means decoding its GLB files (images and all), which takes the CPU a good part
//! of a second a file. [`prefetch`] starts that on worker threads; [`load`] takes the decoded
//! model when it is ready (waiting for it if it is still on its way) or reads it there and then.
//! Zones prefetch every file they need before uploading any (so the files decode side by side),
//! and the world prefetches the zone behind a door while the Queen walks up to it.

use super::*;
use std::sync::Mutex;
use std::thread::JoinHandle;

type Job = JoinHandle<std::result::Result<GltfModel, String>>;

fn jobs() -> &'static Mutex<HashMap<PathBuf, Job>> {
    static JOBS: OnceLock<Mutex<HashMap<PathBuf, Job>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Starts reading these files on worker threads (a few at a time), unless already started.
pub(crate) fn prefetch(paths: impl IntoIterator<Item = PathBuf>) {
    let mut jobs = jobs().lock().unwrap_or_else(|e| e.into_inner());
    let todo: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| p.exists() && !jobs.contains_key(p))
        .collect();
    if todo.is_empty() {
        return;
    }
    // A few threads share the list (the biggest files first, so they finish together).
    let mut todo = todo;
    todo.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).map(|m| m.len()).unwrap_or(0)));
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(2, 6);
    let queue = std::sync::Arc::new(Mutex::new(std::collections::VecDeque::from(todo.clone())));
    let results: std::sync::Arc<
        Mutex<HashMap<PathBuf, std::sync::mpsc::Sender<std::result::Result<GltfModel, String>>>>,
    > = std::sync::Arc::new(Mutex::new(HashMap::new()));
    for path in &todo {
        let (tx, rx) = std::sync::mpsc::channel();
        results
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(path.clone(), tx);
        // Each file's job waits for its own result from whichever worker reads it.
        let job = std::thread::spawn(move || {
            rx.recv()
                .unwrap_or_else(|_| Err("prefetch worker gone".into()))
        });
        jobs.insert(path.clone(), job);
    }
    for _ in 0..workers.min(todo.len()) {
        let queue = queue.clone();
        let results = results.clone();
        std::thread::spawn(move || loop {
            let Some(path) = queue.lock().unwrap_or_else(|e| e.into_inner()).pop_front() else {
                break;
            };
            let model = GltfModel::load(&path).map_err(|e| e.to_string());
            // Its textures' mip chains (and their compression) made here too, not on the frame.
            if let Ok(model) = &model {
                prepare_textures(model, &path);
            }
            if let Some(tx) = results
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&path)
            {
                let _ = tx.send(model);
            }
        });
    }
}

/// The model of `path`: the prefetched one (waiting for it if need be), else read now.
pub(crate) fn load(path: &Path) -> Result<GltfModel> {
    let job = jobs()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(path);
    let mut model = match job {
        Some(job) => match job.join() {
            Ok(Ok(model)) => model,
            Ok(Err(error)) => return Err(anyhow::anyhow!("{error}")),
            Err(_) => return Err(anyhow::anyhow!("prefetch of {} panicked", path.display())),
        },
        None => GltfModel::load(path)?,
    };
    // Its signs in the player's language (it may have been read ahead in another).
    super::sign_textures::note_model(path, &mut model);
    Ok(model)
}

/// Textures prepared ahead, by the key `upload_texture_pair` caches them by.
/// (Each with the file it came from, so a zone walked away from lets its own go.)
type Prepared = HashMap<TextureCacheKey, (PathBuf, velvet_render::PreparedTexture3D)>;

fn prepared() -> &'static Mutex<Prepared> {
    static PREPARED: OnceLock<Mutex<Prepared>> = OnceLock::new();
    PREPARED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Prepares every (albedo, normal) pair a model's meshes use.
fn prepare_textures(model: &GltfModel, path: &Path) {
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255; 4],
    };
    for mesh in &model.meshes {
        let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
        let normal = mesh
            .normal_texture_index
            .and_then(|i| model.textures.get(i));
        if albedo.is_none() && normal.is_none() {
            continue;
        }
        let key = texture_cache_key(albedo.unwrap_or(&white), normal);
        if prepared()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&key)
        {
            continue;
        }
        if let Ok(p) = velvet_render::PreparedTexture3D::new(albedo.unwrap_or(&white), normal) {
            prepared()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(key, (path.to_path_buf(), p));
        }
    }
}

/// A texture pair prepared ahead, if one was (taken: it is uploaded once).
pub(crate) fn take_prepared(key: &TextureCacheKey) -> Option<velvet_render::PreparedTexture3D> {
    prepared()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(key)
        .map(|(_, p)| p)
}

/// How much is held ahead: models read (or being read) and texture pairs prepared, not yet taken.
pub(crate) fn held() -> (usize, usize) {
    (
        jobs().lock().unwrap_or_else(|e| e.into_inner()).len(),
        prepared().lock().unwrap_or_else(|e| e.into_inner()).len(),
    )
}

/// Drops models read ahead but never used (the Queen turned back from the door).
pub(crate) fn forget(prefix: &Path) {
    let mut jobs = jobs().lock().unwrap_or_else(|e| e.into_inner());
    jobs.retain(|p, _| !p.starts_with(prefix));
    drop(jobs);
    prepared()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|_, (p, _)| !p.starts_with(prefix));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefetched_models_are_the_same_as_loaded_ones() {
        let dir = runtime_3d_root().join("baths");
        let files: Vec<PathBuf> = ["baths_life.glb", "baths_water.glb", "spa_girl.glb"]
            .iter()
            .map(|f| dir.join(f))
            .filter(|p| p.exists())
            .collect();
        if files.is_empty() {
            return;
        }
        prefetch(files.clone());
        for f in &files {
            let a = load(f).unwrap();
            let b = GltfModel::load(f).unwrap();
            assert_eq!(a.meshes.len(), b.meshes.len());
            assert_eq!(a.mesh_names, b.mesh_names);
        }
        // Loading again (nothing prefetched) reads it directly.
        assert!(load(&files[0]).is_ok());
    }
}
