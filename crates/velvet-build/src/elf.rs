//! Just enough ELF reading to check an Android shared library before it is packaged: its
//! machine, the alignment of its loadable segments (Google Play requires 16 KB on 64-bit
//! devices) and the libraries it needs at load time.

use std::path::Path;

/// What [`inspect_elf`] found in a shared library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfInfo {
    /// 64-bit (ELFCLASS64) or 32-bit.
    pub is_64: bool,
    /// The `e_machine` field (183 = AArch64, 62 = x86-64, 40 = ARM, 3 = x86).
    pub machine: u16,
    /// The smallest `p_align` of the `PT_LOAD` segments.
    pub min_load_align: u64,
    /// `DT_NEEDED` entries: libraries the dynamic loader must find.
    pub needed: Vec<String>,
}

impl ElfInfo {
    /// The Android ABI name of the machine (`arm64-v8a`, `x86_64`, `armeabi-v7a`, `x86`).
    pub fn android_abi(&self) -> Option<&'static str> {
        match (self.machine, self.is_64) {
            (183, true) => Some("arm64-v8a"),
            (62, true) => Some("x86_64"),
            (40, false) => Some("armeabi-v7a"),
            (3, false) => Some("x86"),
            _ => None,
        }
    }

    /// Whether every loadable segment is aligned to 16 KB pages.
    pub fn is_16k_aligned(&self) -> bool {
        self.min_load_align >= 16 * 1024
    }
}

/// Reads an ELF file.
pub fn inspect_elf_file(path: &Path) -> Result<ElfInfo, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    inspect_elf(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads ELF bytes (little-endian only, as every Android ABI is).
pub fn inspect_elf(bytes: &[u8]) -> Result<ElfInfo, String> {
    if bytes.len() < 52 || &bytes[..4] != b"\x7fELF" {
        return Err("not an ELF file".into());
    }
    let is_64 = match bytes[4] {
        1 => false,
        2 => true,
        _ => return Err("unknown ELF class".into()),
    };
    if bytes[5] != 1 {
        return Err("big-endian ELF".into());
    }
    let r = Reader { bytes, is_64 };
    let machine = r.u16(18)?;
    let (phoff, phentsize, phnum) = if is_64 {
        (r.u64(32)?, r.u16(54)? as u64, r.u16(56)? as u64)
    } else {
        (r.u32(28)? as u64, r.u16(42)? as u64, r.u16(44)? as u64)
    };
    let mut min_load_align = u64::MAX;
    let mut dynamic: Option<(u64, u64)> = None;
    let mut loads: Vec<(u64, u64, u64)> = Vec::new(); // (vaddr, offset, filesz)
    for i in 0..phnum {
        let at = phoff + i * phentsize;
        let p_type = r.u32(at)?;
        let (offset, vaddr, filesz, align) = if is_64 {
            (
                r.u64(at + 8)?,
                r.u64(at + 16)?,
                r.u64(at + 32)?,
                r.u64(at + 48)?,
            )
        } else {
            (
                r.u32(at + 4)? as u64,
                r.u32(at + 8)? as u64,
                r.u32(at + 16)? as u64,
                r.u32(at + 28)? as u64,
            )
        };
        match p_type {
            1 => {
                min_load_align = min_load_align.min(align);
                loads.push((vaddr, offset, filesz));
            }
            2 => dynamic = Some((offset, filesz)),
            _ => {}
        }
    }
    if min_load_align == u64::MAX {
        return Err("no loadable segments".into());
    }
    let mut needed = Vec::new();
    if let Some((dyn_off, dyn_size)) = dynamic {
        let entry = if is_64 { 16 } else { 8 };
        let mut strtab_vaddr = None;
        let mut needed_offsets = Vec::new();
        let mut at = dyn_off;
        while at + entry <= dyn_off + dyn_size {
            let (tag, value) = if is_64 {
                (r.u64(at)?, r.u64(at + 8)?)
            } else {
                (r.u32(at)? as u64, r.u32(at + 4)? as u64)
            };
            match tag {
                0 => break,
                1 => needed_offsets.push(value),
                5 => strtab_vaddr = Some(value),
                _ => {}
            }
            at += entry;
        }
        // DT_STRTAB holds a virtual address: map it back to a file offset through PT_LOAD.
        if let Some(vaddr) = strtab_vaddr {
            let file_offset = loads
                .iter()
                .find(|(start, _, size)| vaddr >= *start && vaddr < start + size)
                .map(|(start, offset, _)| vaddr - start + offset)
                .ok_or("the string table is outside the loadable segments")?;
            for name in needed_offsets {
                needed.push(r.c_str(file_offset + name)?);
            }
        }
    }
    Ok(ElfInfo {
        is_64,
        machine,
        min_load_align,
        needed,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    #[allow(dead_code)]
    is_64: bool,
}

impl Reader<'_> {
    fn slice(&self, at: u64, len: usize) -> Result<&[u8], String> {
        let start = usize::try_from(at).map_err(|_| "offset too large")?;
        self.bytes
            .get(start..start + len)
            .ok_or_else(|| format!("truncated ELF (offset {at})"))
    }
    fn u16(&self, at: u64) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.slice(at, 2)?.try_into().unwrap()))
    }
    fn u32(&self, at: u64) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.slice(at, 4)?.try_into().unwrap()))
    }
    fn u64(&self, at: u64) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.slice(at, 8)?.try_into().unwrap()))
    }
    fn c_str(&self, at: u64) -> Result<String, String> {
        let start = usize::try_from(at).map_err(|_| "offset too large")?;
        let rest = self.bytes.get(start..).ok_or("string outside the file")?;
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or("unterminated string")?;
        Ok(String::from_utf8_lossy(&rest[..end]).into_owned())
    }
}

/// Libraries every Android device provides to apps (the NDK's stable system libraries).
pub const ANDROID_SYSTEM_LIBRARIES: &[&str] = &[
    "libc.so",
    "libm.so",
    "libdl.so",
    "liblog.so",
    "libz.so",
    "libandroid.so",
    "libEGL.so",
    "libGLESv1_CM.so",
    "libGLESv2.so",
    "libGLESv3.so",
    "libvulkan.so",
    "libjnigraphics.so",
    "libOpenSLES.so",
    "libaaudio.so",
    "libamidi.so",
    "libmediandk.so",
    "libnativewindow.so",
    "libneuralnetworks.so",
    "libcamera2ndk.so",
    "libsync.so",
    "libbinder_ndk.so",
    "libicu.so",
    "libstdc++.so",
];

/// Lowest API level at which a system library can be loaded by apps, when later than 21.
pub fn system_library_min_api(name: &str) -> Option<u32> {
    match name {
        "libaaudio.so" | "libnativewindow.so" | "libsync.so" => Some(26),
        "libneuralnetworks.so" => Some(27),
        "libcamera2ndk.so" | "libmediandk.so" => Some(24),
        "libamidi.so" | "libbinder_ndk.so" => Some(29),
        "libicu.so" => Some(31),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal ELF64 AArch64 shared object: two PT_LOAD segments (16 KB aligned) and a
    /// PT_DYNAMIC listing two DT_NEEDED entries.
    fn tiny_elf(align: u64) -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2; // 64-bit
        b[5] = 1; // little-endian
        b[18..20].copy_from_slice(&183u16.to_le_bytes());
        b[32..40].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
        b[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        b[56..58].copy_from_slice(&3u16.to_le_bytes()); // e_phnum
        let ph = |b: &mut Vec<u8>, i: usize, t: u32, off: u64, vaddr: u64, size: u64, al: u64| {
            let at = 64 + i * 56;
            b[at..at + 4].copy_from_slice(&t.to_le_bytes());
            b[at + 8..at + 16].copy_from_slice(&off.to_le_bytes());
            b[at + 16..at + 24].copy_from_slice(&vaddr.to_le_bytes());
            b[at + 32..at + 40].copy_from_slice(&size.to_le_bytes());
            b[at + 48..at + 56].copy_from_slice(&al.to_le_bytes());
        };
        ph(&mut b, 0, 1, 0, 0, 0x300, align);
        ph(&mut b, 1, 1, 0x300, 0x4300, 0x100, align);
        ph(&mut b, 2, 2, 0x300, 0x4300, 0x40, 8);
        // Dynamic section at file 0x300: NEEDED(1), NEEDED(11), STRTAB(vaddr 0x200), NULL.
        let dynamic = [(1u64, 1u64), (1, 11), (5, 0x200), (0, 0)];
        for (i, (tag, val)) in dynamic.iter().enumerate() {
            let at = 0x300 + i * 16;
            b[at..at + 8].copy_from_slice(&tag.to_le_bytes());
            b[at + 8..at + 16].copy_from_slice(&val.to_le_bytes());
        }
        // String table at file 0x200 (vaddr 0x200 in the first segment): "\0liblog.so\0libc++_shared.so\0".
        b[0x201..0x201 + 9].copy_from_slice(b"liblog.so");
        b[0x20b..0x20b + 16].copy_from_slice(b"libc++_shared.so");
        b
    }

    #[test]
    fn reads_alignment_and_needed_libraries() {
        let info = inspect_elf(&tiny_elf(0x4000)).unwrap();
        assert!(info.is_64);
        assert_eq!(info.android_abi(), Some("arm64-v8a"));
        assert!(info.is_16k_aligned());
        assert_eq!(info.needed, vec!["liblog.so", "libc++_shared.so"]);
        let old = inspect_elf(&tiny_elf(0x1000)).unwrap();
        assert!(!old.is_16k_aligned());
        assert!(inspect_elf(b"not an elf at all, really not one, no no no no no").is_err());
        assert_eq!(system_library_min_api("libaaudio.so"), Some(26));
        assert!(ANDROID_SYSTEM_LIBRARIES.contains(&"libvulkan.so"));
    }
}
