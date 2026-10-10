//! Compile-time bundle access for the single-file portable build.
//!
//! Normal development builds keep reading editable files from the repository.
//! `--features portable` switches the released game to bytes compiled into the
//! executable by `build.rs`.

#[cfg(feature = "portable")]
mod bundle {
    include!(concat!(env!("OUT_DIR"), "/demon_queen_portable.rs"));
}

/// Whether this binary carries its complete runtime data bundle.
pub const fn enabled() -> bool {
    cfg!(feature = "portable")
}

/// Authoritative prologue JSON embedded in the portable executable.
pub fn story_bytes() -> Option<&'static [u8]> {
    #[cfg(feature = "portable")]
    {
        Some(bundle::STORY_JSON)
    }
    #[cfg(not(feature = "portable"))]
    {
        None
    }
}

/// Main-menu VCSS embedded in the portable executable.
pub fn menu_styles() -> Option<&'static str> {
    #[cfg(feature = "portable")]
    {
        Some(bundle::MAIN_MENU_VCSS)
    }
    #[cfg(not(feature = "portable"))]
    {
        None
    }
}

/// One locale catalog by BCP-47-like game code.
pub fn locale_bytes(code: &str) -> Option<&'static [u8]> {
    #[cfg(feature = "portable")]
    {
        bundle::locale_bytes(code)
    }
    #[cfg(not(feature = "portable"))]
    {
        let _ = code;
        None
    }
}

/// One runtime image by normalized path relative to `data/assets`.
pub fn asset_bytes(path: &str) -> Option<&'static [u8]> {
    #[cfg(feature = "portable")]
    {
        bundle::asset_bytes(path)
    }
    #[cfg(not(feature = "portable"))]
    {
        let _ = path;
        None
    }
}

/// Number of PNG/SVG assets carried by the portable bundle.
pub const fn asset_count() -> usize {
    #[cfg(feature = "portable")]
    {
        bundle::ASSET_COUNT
    }
    #[cfg(not(feature = "portable"))]
    {
        0
    }
}

#[cfg(all(test, feature = "portable"))]
mod tests {
    use super::*;

    #[test]
    fn portable_bundle_contains_every_runtime_class() {
        assert!(enabled());
        assert!(story_bytes().is_some());
        assert!(menu_styles().is_some());
        assert!(asset_count() >= 100);
        assert!(asset_bytes("native_1080p/title/menu_bg_garden.png").is_some());
        assert!(asset_bytes("native_1080p/cg/p01_last_tray.png").is_some());
        assert!(asset_bytes("title/demon_queen_menu_overlay_native_705x1050.png").is_some());
        for code in [
            "en", "es", "de", "zh-Hans", "zh-Hant", "hi", "ru", "ja", "ko", "la", "eo",
        ] {
            assert!(
                locale_bytes(code).is_some(),
                "missing embedded locale {code}"
            );
        }
    }
}
