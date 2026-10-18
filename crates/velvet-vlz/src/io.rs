//! Positional reads: any number of threads read one open file at independent offsets without
//! sharing a cursor (pread on Unix, overlapped offsets on Windows, a lock elsewhere).

use std::fs::File;
use std::io;

#[derive(Debug)]
pub(crate) struct PositionalFile {
    #[cfg(any(unix, windows))]
    file: File,
    #[cfg(not(any(unix, windows)))]
    file: std::sync::Mutex<File>,
    len: u64,
}

impl PositionalFile {
    pub(crate) fn new(file: File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        #[cfg(not(any(unix, windows)))]
        let file = std::sync::Mutex::new(file);
        Ok(Self { file, len })
    }

    pub(crate) fn len(&self) -> u64 {
        self.len
    }

    pub(crate) fn read_exact_at(&self, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
        if offset
            .checked_add(buf.len() as u64)
            .is_none_or(|end| end > self.len)
        {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "read past the end of the archive",
            ));
        }
        while !buf.is_empty() {
            let n = self.read_at(buf, offset)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "archive shrank while reading",
                ));
            }
            buf = &mut buf[n..];
            offset += n as u64;
        }
        Ok(())
    }

    #[cfg(unix)]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::os::unix::fs::FileExt;
        loop {
            match self.file.read_at(buf, offset) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                other => return other,
            }
        }
    }

    #[cfg(windows)]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::os::windows::fs::FileExt;
        loop {
            match self.file.seek_read(buf, offset) {
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                other => return other,
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = self.file.lock().unwrap_or_else(|p| p.into_inner());
        file.seek(SeekFrom::Start(offset))?;
        file.read(buf)
    }
}
