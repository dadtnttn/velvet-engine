//! The language of the player's system, so a game can start in it.

/// The user's interface language as a BCP 47 tag ("es-MX", "zh-TW"), or ""
/// when the system does not say.
pub fn system_language() -> String {
    #[cfg(windows)]
    {
        if let Some(tag) = windows_ui_language() {
            return tag;
        }
    }
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(var) {
            let tag = value
                .split(['.', '@'])
                .next()
                .unwrap_or("")
                .replace('_', "-");
            if !tag.is_empty() && tag != "C" && tag != "POSIX" {
                return tag;
            }
        }
    }
    String::new()
}

#[cfg(windows)]
fn windows_ui_language() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
        fn LCIDToLocaleName(locale: u32, name: *mut u16, len: i32, flags: u32) -> i32;
    }
    // LOCALE_NAME_MAX_LENGTH wide characters, terminator included.
    let mut buf = [0u16; 85];
    // SAFETY: both calls only read their arguments; the buffer holds as many
    // UTF-16 units as we tell LCIDToLocaleName it may write.
    let written = unsafe {
        let lang = GetUserDefaultUILanguage();
        LCIDToLocaleName(u32::from(lang), buf.as_mut_ptr(), buf.len() as i32, 0)
    };
    (written > 1).then(|| String::from_utf16_lossy(&buf[..written as usize - 1]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_posix_language_variables() {
        // Only the shape of the answer: the test machine's language varies.
        let tag = super::system_language();
        assert!(!tag.contains('.') && !tag.contains('_'));
    }
}
