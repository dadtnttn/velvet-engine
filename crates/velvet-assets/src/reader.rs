//! A seekable reader over one asset, for streaming large resources (music, video, archives)
//! instead of loading them whole.

use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};

/// Reads one asset from wherever its source keeps it: a byte range of a file (a loose file on
/// desktop, or an uncompressed entry inside an Android APK, read through the file descriptor the
/// asset manager hands out) or bytes already in memory (compressed entries, memory sources).
///
/// It is `Send + Sync + 'static`, so it can be handed to decoders that run on other threads
/// (for example `rodio::Decoder`).
#[derive(Debug)]
pub struct AssetReader {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    /// `len` bytes of `file` starting at `start`; `pos` is relative to `start`.
    File {
        file: File,
        start: u64,
        len: u64,
        pos: u64,
    },
    Memory(Cursor<Vec<u8>>),
}

impl AssetReader {
    /// A reader over bytes in memory.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self {
            inner: Inner::Memory(Cursor::new(bytes)),
        }
    }

    /// A reader over a whole file.
    pub fn from_file(file: File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        Self::from_file_range(file, 0, len)
    }

    /// A reader over `len` bytes of `file` starting at `start` (an asset stored inside a
    /// package file).
    pub fn from_file_range(mut file: File, start: u64, len: u64) -> io::Result<Self> {
        file.seek(SeekFrom::Start(start))?;
        Ok(Self {
            inner: Inner::File {
                file,
                start,
                len,
                pos: 0,
            },
        })
    }

    /// Length of the asset in bytes.
    pub fn len(&self) -> u64 {
        match &self.inner {
            Inner::File { len, .. } => *len,
            Inner::Memory(c) => c.get_ref().len() as u64,
        }
    }

    /// Whether the asset is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the bytes are streamed from a file (true) or held in memory (false).
    pub fn is_streamed(&self) -> bool {
        matches!(self.inner, Inner::File { .. })
    }

    /// Reads everything that is left.
    pub fn into_bytes(mut self) -> io::Result<Vec<u8>> {
        if let Inner::Memory(cursor) = self.inner {
            if cursor.position() == 0 {
                return Ok(cursor.into_inner());
            }
            self.inner = Inner::Memory(cursor);
        }
        let mut out = Vec::with_capacity(self.len() as usize);
        self.read_to_end(&mut out)?;
        Ok(out)
    }
}

impl Read for AssetReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            Inner::File { file, len, pos, .. } => {
                let left = len.saturating_sub(*pos);
                if left == 0 || buf.is_empty() {
                    return Ok(0);
                }
                let take = (buf.len() as u64).min(left) as usize;
                let n = file.read(&mut buf[..take])?;
                *pos += n as u64;
                Ok(n)
            }
            Inner::Memory(cursor) => cursor.read(buf),
        }
    }
}

impl Seek for AssetReader {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        match &mut self.inner {
            Inner::File {
                file,
                start,
                len,
                pos,
            } => {
                let target = match to {
                    SeekFrom::Start(n) => Some(n),
                    SeekFrom::End(d) => len.checked_add_signed(d),
                    SeekFrom::Current(d) => pos.checked_add_signed(d),
                };
                let target = target.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "seek before the start of the asset",
                    )
                })?;
                file.seek(SeekFrom::Start(*start + target))?;
                *pos = target;
                Ok(target)
            }
            Inner::Memory(cursor) => cursor.seek(to),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn memory_and_file_ranges_read_the_same() {
        let data: Vec<u8> = (0u8..=200).collect();
        let mut memory = AssetReader::from_bytes(data[50..150].to_vec());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.bin");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&data)
            .unwrap();
        let mut ranged =
            AssetReader::from_file_range(std::fs::File::open(&path).unwrap(), 50, 100).unwrap();
        assert!(ranged.is_streamed());
        assert_eq!(ranged.len(), 100);

        for reader in [&mut memory, &mut ranged] {
            let mut head = [0u8; 4];
            reader.read_exact(&mut head).unwrap();
            assert_eq!(head, [50, 51, 52, 53]);
            assert_eq!(reader.seek(SeekFrom::End(-2)).unwrap(), 98);
            let mut rest = Vec::new();
            reader.read_to_end(&mut rest).unwrap();
            assert_eq!(rest, vec![148, 149]);
            assert_eq!(reader.seek(SeekFrom::Current(-10)).unwrap(), 90);
            assert!(reader.seek(SeekFrom::Current(-1000)).is_err());
        }
        // Reading past the range never leaks bytes that belong to the next entry.
        ranged.seek(SeekFrom::Start(99)).unwrap();
        let mut tail = [0u8; 8];
        assert_eq!(ranged.read(&mut tail).unwrap(), 1);
        assert_eq!(tail[0], 149);
        assert_eq!(ranged.read(&mut tail).unwrap(), 0);
    }

    #[test]
    fn into_bytes_keeps_untouched_memory() {
        let reader = AssetReader::from_bytes(vec![1, 2, 3]);
        assert_eq!(reader.into_bytes().unwrap(), vec![1, 2, 3]);
        let mut reader = AssetReader::from_bytes(vec![1, 2, 3]);
        reader.seek(SeekFrom::Start(1)).unwrap();
        assert_eq!(reader.into_bytes().unwrap(), vec![2, 3]);
    }

    #[test]
    fn readers_can_cross_threads() {
        fn needs<T: Send + Sync + 'static>() {}
        needs::<AssetReader>();
    }
}
