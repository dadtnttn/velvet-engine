//! VLZ archives as an asset source: renderers, audio and scenes keep asking `Assets` for paths
//! and never learn which codec, if any, a resource was stored with.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use velvet_vlz::{OpenOptions, VlzArchive, VlzError};

use crate::path::AssetPath;
use crate::source::{Source, SourceError};

/// A [`Source`] backed by one `.vlz` archive. Whole reads decode a resource's blocks (in
/// parallel for large ones); [`Source::read_range`] decodes only the blocks it covers.
///
/// Usually layered before loose files:
///
/// ```no_run
/// use std::sync::Arc;
/// use velvet_assets::{Assets, FileSource, LayeredSource, VlzSource};
///
/// let source = LayeredSource::new()
///     .with(Arc::new(VlzSource::open("game.vlz")?))
///     .with(Arc::new(FileSource::new("assets")));
/// let assets = Assets::with_source(Arc::new(source));
/// # Ok::<(), velvet_assets::SourceError>(())
/// ```
#[derive(Debug, Clone)]
pub struct VlzSource {
    archive: Arc<VlzArchive>,
    root: PathBuf,
}

impl VlzSource {
    /// Open an archive with default options.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        Self::open_with(path, OpenOptions::default())
    }

    /// Open an archive with options (read limits, decode threads, extra decoders).
    pub fn open_with(path: impl AsRef<Path>, opts: OpenOptions) -> Result<Self, SourceError> {
        let path = path.as_ref();
        let archive = VlzArchive::open_with(path, opts).map_err(to_source_error)?;
        Ok(Self {
            archive: Arc::new(archive),
            root: path.to_path_buf(),
        })
    }

    /// Wrap an archive already open (shared with tools or other sources).
    pub fn from_archive(archive: Arc<VlzArchive>) -> Self {
        let root = archive
            .path()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("vlz://"));
        Self { archive, root }
    }

    /// The archive.
    pub fn archive(&self) -> &Arc<VlzArchive> {
        &self.archive
    }
}

fn to_source_error(e: VlzError) -> SourceError {
    match e {
        VlzError::NotFound(name) => SourceError::NotFound(name),
        other => SourceError::Io(other.into()),
    }
}

impl Source for VlzSource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        self.archive
            .read(path.virtual_path.as_str())
            .map_err(to_source_error)
    }

    fn exists(&self, path: &AssetPath) -> bool {
        self.archive.contains(path.virtual_path.as_str())
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![self.root.clone()]
    }

    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        let id = self.archive.find(path.virtual_path.as_str())?;
        Some(self.archive.entry(id).raw_size)
    }

    fn read_range(
        &self,
        path: &AssetPath,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, SourceError> {
        let id = self
            .archive
            .find(path.virtual_path.as_str())
            .ok_or_else(|| SourceError::NotFound(path.to_string()))?;
        self.archive
            .read_range(id, offset, len)
            .map_err(to_source_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{FileSource, LayeredSource};
    use crate::{Assets, TextAsset};
    use velvet_vlz::{pack_to_file, PackInput, PackOptions};

    #[test]
    fn assets_load_from_a_vlz_with_loose_fallback() {
        let dir = std::env::temp_dir().join(format!("velvet-vlz-src-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("loose")).unwrap();
        std::fs::write(dir.join("loose/patch.txt"), "loose file").unwrap();
        std::fs::write(dir.join("loose/story.json"), "stale loose copy").unwrap();
        let body = "{\"scene\":\"intro\"}".repeat(1000);
        pack_to_file(
            vec![
                PackInput::bytes("story.json", body.clone().into_bytes()),
                PackInput::bytes(
                    "bin/data.bin",
                    (0..=255u8).cycle().take(100_000).collect::<Vec<_>>(),
                ),
            ],
            dir.join("game.vlz"),
            &PackOptions::default(),
        )
        .unwrap();

        let vlz = VlzSource::open(dir.join("game.vlz")).unwrap();
        let p = AssetPath::virtual_path("bin/data.bin");
        assert_eq!(vlz.size_hint(&p), Some(100_000));
        assert_eq!(vlz.read_range(&p, 256, 3).unwrap(), vec![0, 1, 2]);
        assert!(matches!(
            vlz.read(&"none".into()),
            Err(SourceError::NotFound(_))
        ));

        let source = LayeredSource::new()
            .with(Arc::new(vlz))
            .with(Arc::new(FileSource::new(dir.join("loose"))));
        let mut assets = Assets::with_source(Arc::new(source));
        let story = assets.load::<TextAsset>("story.json");
        assert_eq!(
            assets.get(story).unwrap().text,
            body,
            "the archive wins over loose files"
        );
        let patch = assets.load::<TextAsset>("patch.txt");
        assert_eq!(assets.get(patch).unwrap().text, "loose file");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
