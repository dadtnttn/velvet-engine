//! The user's language, for a game's first-run default (a saved choice should still win).

/// The system's language as a BCP 47 tag (`es-ES`, `pt-BR`, `en`), when the platform says:
///
/// - Android: the configuration's language and country (`AConfiguration`);
/// - Windows: the first of the user's preferred UI languages;
/// - elsewhere: `LC_ALL`, `LC_MESSAGES` or `LANG` (`es_ES.UTF-8` becomes `es-ES`; `C` and
///   `POSIX` say nothing).
pub fn system_locale() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        let app = crate::android::app()?;
        let config = app.config();
        let language = config.language()?;
        Some(match config.country() {
            Some(country) => format!("{language}-{country}"),
            None => language,
        })
    }
    #[cfg(windows)]
    {
        windows_ui_language()
    }
    #[cfg(not(any(target_os = "android", windows)))]
    {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find_map(|value| posix_to_bcp47(&value))
    }
}

/// `es_ES.UTF-8@euro` → `es-ES`; `C`, `POSIX` and empty values → `None`.
pub fn posix_to_bcp47(value: &str) -> Option<String> {
    let tag = value.split(['.', '@']).next()?.trim();
    if tag.is_empty() || tag == "C" || tag == "POSIX" {
        return None;
    }
    Some(tag.replace('_', "-"))
}

#[cfg(windows)]
fn windows_ui_language() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserPreferredUILanguages(
            flags: u32,
            languages: *mut u32,
            buffer: *mut u16,
            buffer_len: *mut u32,
        ) -> i32;
    }
    const MUI_LANGUAGE_NAME: u32 = 0x8;
    let mut buffer = [0u16; 512];
    let mut len = buffer.len() as u32;
    let mut count = 0u32;
    // SAFETY: the buffer and its length in characters are valid for the call, which writes a
    // double-NUL-terminated list of language names into it.
    let ok = unsafe {
        GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, buffer.as_mut_ptr(), &mut len)
    };
    if ok == 0 || count == 0 {
        return None;
    }
    let first: Vec<u16> = buffer.iter().copied().take_while(|&c| c != 0).collect();
    (!first.is_empty()).then(|| String::from_utf16_lossy(&first))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_locales_become_tags() {
        assert_eq!(posix_to_bcp47("es_ES.UTF-8").as_deref(), Some("es-ES"));
        assert_eq!(posix_to_bcp47("pt_BR@latin").as_deref(), Some("pt-BR"));
        assert_eq!(posix_to_bcp47("ja").as_deref(), Some("ja"));
        assert_eq!(posix_to_bcp47("C.UTF-8"), None);
        assert_eq!(posix_to_bcp47("POSIX"), None);
        assert_eq!(posix_to_bcp47(""), None);
        // Whatever the machine says, it is a tag without POSIX decorations.
        if let Some(tag) = system_locale() {
            assert!(!tag.contains('_') && !tag.contains('.'), "{tag}");
        }
    }
}
