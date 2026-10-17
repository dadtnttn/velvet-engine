//! Runtime localization catalogs and persistent language preference.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::game::SceneView;

const CATALOG_FORMAT: &str = "demon-queen-locale";
const CATALOG_SCHEMA: u32 = 1;
const SETTINGS_FORMAT: &str = "demon-queen-settings";
const SETTINGS_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Spanish,
    English,
    PortugueseBrazil,
    German,
    French,
    Italian,
    Polish,
    Russian,
    ChineseSimplified,
    ChineseTraditional,
    Japanese,
    Korean,
    Hebrew,
    Hindi,
    Thai,
}

impl Language {
    /// Locale used by a clean installation. A persisted user choice still wins.
    pub const DEFAULT: Self = Self::English;

    pub const ALL: [Self; 15] = [
        Self::English,
        Self::Spanish,
        Self::PortugueseBrazil,
        Self::German,
        Self::French,
        Self::Italian,
        Self::Polish,
        Self::Russian,
        Self::ChineseSimplified,
        Self::ChineseTraditional,
        Self::Japanese,
        Self::Korean,
        Self::Hebrew,
        Self::Hindi,
        Self::Thai,
    ];

    /// The language a clean installation starts in: the one Windows is displayed in when the game
    /// speaks it, English otherwise (a saved choice still wins).
    pub fn system_default() -> Self {
        // QA: `VELVET_UI_LANGID=0x0419` starts as if Windows were in that language (its LANGID).
        if let Some(id) = std::env::var("VELVET_UI_LANGID")
            .ok()
            .and_then(|v| parse_langid(&v))
        {
            return Self::from_windows_langid(id).unwrap_or(Self::DEFAULT);
        }
        #[cfg(windows)]
        {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetUserDefaultUILanguage() -> u16;
            }
            // SAFETY: a plain Win32 call with no arguments, returning the user's UI language id.
            let id = unsafe { GetUserDefaultUILanguage() };
            if let Some(language) = Self::from_windows_langid(id) {
                return language;
            }
        }
        #[cfg(not(windows))]
        {
            if let Some(language) = std::env::var("LANG")
                .ok()
                .and_then(|tag| Self::from_locale_tag(&tag))
            {
                return language;
            }
        }
        Self::DEFAULT
    }

    /// The game's language for a Windows language id (the primary language in its low 10 bits, the
    /// country above). Portugal reads the Brazilian text; Taiwan, Hong Kong and Macao read
    /// Traditional Chinese, the other Chinese Simplified.
    pub fn from_windows_langid(id: u16) -> Option<Self> {
        Some(match id & 0x3ff {
            0x0a => Self::Spanish,
            0x09 => Self::English,
            0x16 => Self::PortugueseBrazil,
            0x07 => Self::German,
            0x0c => Self::French,
            0x10 => Self::Italian,
            0x15 => Self::Polish,
            0x19 => Self::Russian,
            0x04 => match id >> 10 {
                // Taiwan, Hong Kong, Macao and zh-Hant itself.
                0x01 | 0x03 | 0x05 | 0x1f => Self::ChineseTraditional,
                _ => Self::ChineseSimplified,
            },
            0x11 => Self::Japanese,
            0x12 => Self::Korean,
            0x0d => Self::Hebrew,
            0x39 => Self::Hindi,
            0x1e => Self::Thai,
            _ => return None,
        })
    }

    /// The same for a locale tag such as `de_DE.UTF-8` or `zh-TW` (`LANG` outside Windows).
    #[cfg(test)]
    pub fn from_locale_tag(tag: &str) -> Option<Self> {
        let tag = tag
            .split(['.', '@'])
            .next()?
            .to_ascii_lowercase()
            .replace('_', "-");
        let mut parts = tag.split('-');
        Some(match parts.next()? {
            "es" => Self::Spanish,
            "en" => Self::English,
            "pt" => Self::PortugueseBrazil,
            "de" => Self::German,
            "fr" => Self::French,
            "it" => Self::Italian,
            "pl" => Self::Polish,
            "ru" => Self::Russian,
            "zh" if parts.any(|p| matches!(p, "tw" | "hk" | "mo" | "hant")) => {
                Self::ChineseTraditional
            }
            "zh" => Self::ChineseSimplified,
            "ja" => Self::Japanese,
            "ko" => Self::Korean,
            "he" | "iw" => Self::Hebrew,
            "hi" => Self::Hindi,
            "th" => Self::Thai,
            _ => return None,
        })
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Spanish => "es",
            Self::English => "en",
            Self::PortugueseBrazil => "pt-BR",
            Self::German => "de",
            Self::French => "fr",
            Self::Italian => "it",
            Self::Polish => "pl",
            Self::Russian => "ru",
            Self::ChineseSimplified => "zh-Hans",
            Self::ChineseTraditional => "zh-Hant",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::Hebrew => "he",
            Self::Hindi => "hi",
            Self::Thai => "th",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Spanish => "Español",
            Self::English => "English",
            Self::PortugueseBrazil => "Português (Brasil)",
            Self::German => "Deutsch",
            Self::French => "Français",
            Self::Italian => "Italiano",
            Self::Polish => "Polski",
            Self::Russian => "Русский",
            Self::ChineseSimplified => "简体中文",
            Self::ChineseTraditional => "繁體中文",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Hebrew => "עברית",
            Self::Hindi => "हिन्दी",
            Self::Thai => "ไทย",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    format: String,
    schema: u32,
    code: String,
    #[serde(default)]
    ui: BTreeMap<String, String>,
    #[serde(default)]
    rooms: BTreeMap<String, String>,
    #[serde(default)]
    characters: BTreeMap<String, String>,
    #[serde(default)]
    nodes: BTreeMap<String, String>,
    #[serde(default)]
    choices: BTreeMap<String, String>,
}

impl Catalog {
    fn load(language: Language) -> Result<Self> {
        let path = locales_root().join(format!("{}.json", language.code()));
        let bytes: Cow<'static, [u8]> = if crate::portable::enabled() {
            Cow::Borrowed(
                crate::portable::locale_bytes(language.code()).ok_or_else(|| {
                    anyhow!("el portable no contiene el idioma {}", language.code())
                })?,
            )
        } else {
            Cow::Owned(
                fs::read(&path)
                    .with_context(|| format!("no se pudo leer el idioma {}", path.display()))?,
            )
        };
        let catalog: Self = serde_json::from_slice(&bytes)
            .with_context(|| format!("catálogo inválido para {}", language.code()))?;
        if catalog.format != CATALOG_FORMAT || catalog.schema != CATALOG_SCHEMA {
            bail!("catálogo incompatible en {}", path.display());
        }
        if catalog.code != language.code() {
            bail!(
                "el catálogo {} declara el código {:?}",
                path.display(),
                catalog.code
            );
        }
        Ok(catalog)
    }
}

// The file also keeps the other preferences (`exploration3d::prefs`): unknown keys are theirs.
#[derive(Debug, Serialize, Deserialize)]
struct SettingsFile {
    format: String,
    schema: u32,
    language: String,
    #[serde(default)]
    reduced_motion: Option<bool>,
}

pub struct Localizer {
    language: Language,
    spanish: Catalog,
    english: Catalog,
    active: Catalog,
}

impl Localizer {
    pub fn load() -> Result<Self> {
        // VELVET_LANG=<code> (captures, tests) over the saved choice.
        let language = std::env::var("VELVET_LANG")
            .ok()
            .and_then(|code| Language::from_code(&code))
            .or_else(saved_language_outside_tests)
            .unwrap_or_else(Language::system_default);
        Self::for_language(language).or_else(|error| {
            eprintln!("idioma {} no disponible: {error:#}", language.code());
            Self::for_language(Language::DEFAULT)
        })
    }

    pub fn for_language(language: Language) -> Result<Self> {
        let spanish = Catalog::load(Language::Spanish)?;
        let english = Catalog::load(Language::English)?;
        let active = match language {
            Language::Spanish => spanish.clone(),
            Language::English => english.clone(),
            _ => Catalog::load(language)?,
        };
        // Everything the game draws follows the same language (its text catalogs).
        demon_queen_dungeon::i18n::set_locale_dir(locales_root());
        velvet_guion::set_translator(demon_queen_dungeon::i18n::tr_string);
        velvet_render::set_image_override(demon_queen_dungeon::i18n::image_override);
        demon_queen_dungeon::i18n::set_language(language.code());
        Ok(Self {
            language,
            spanish,
            english,
            active,
        })
    }

    pub const fn language(&self) -> Language {
        self.language
    }

    /// The English catalog, the fallback of every language but Spanish: Spanish's own text is the
    /// story's, so a Spanish name or node without an entry keeps the story's text.
    fn english_fallback(&self) -> Option<&Catalog> {
        (self.language != Language::Spanish).then_some(&self.english)
    }

    pub fn set_language(&mut self, language: Language) -> Result<()> {
        let active = match language {
            Language::Spanish => self.spanish.clone(),
            Language::English => self.english.clone(),
            _ => Catalog::load(language)?,
        };
        self.language = language;
        self.active = active;
        demon_queen_dungeon::i18n::set_language(language.code());
        save_language(language)
    }

    /// The text in the player's language, then in English, then in Spanish, then the key itself.
    pub fn ui<'a>(&'a self, key: &'a str) -> &'a str {
        self.active
            .ui
            .get(key)
            .or_else(|| {
                self.english_fallback()
                    .and_then(|english| english.ui.get(key))
            })
            .or_else(|| self.spanish.ui.get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }

    pub fn formatted(&self, key: &str, replacements: &[(&str, String)]) -> String {
        let mut value = self.ui(key).to_string();
        for (token, replacement) in replacements {
            value = value.replace(token, replacement);
        }
        value
    }

    pub fn room_name(&self, id: &str, fallback: &str) -> String {
        self.active
            .rooms
            .get(id)
            .or_else(|| {
                self.english_fallback()
                    .and_then(|english| english.rooms.get(id))
            })
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn character_name(&self, id: &str, fallback: &str) -> String {
        self.active
            .characters
            .get(id)
            .or_else(|| {
                self.english_fallback()
                    .and_then(|english| english.characters.get(id))
            })
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    /// The node's text in the player's language; without it, the English one (and only then the
    /// story's own text, `fallback`).
    pub fn node_text(&self, id: &str, fallback: &str) -> String {
        self.active
            .nodes
            .get(id)
            .or_else(|| {
                self.english_fallback()
                    .and_then(|english| english.nodes.get(id))
            })
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn localize_scene(&self, view: &mut SceneView) {
        // The language's catalog, then English's, then the game's text catalogs (guion scenes).
        let tr = demon_queen_dungeon::i18n::tr_string;
        view.room_name = tr(&self.room_name(&view.room_id, &view.room_name));
        view.speaker = tr(&self.character_name(&view.speaker_id, &view.speaker));
        view.text = tr(&self.node_text(&view.id, &view.text));
        for choice in &mut view.choices {
            if choice.id == "continue" {
                choice.label = self.ui("common.continue").to_string();
                continue;
            }
            let key = format!("{}.{}", view.id, choice.id);
            if let Some(label) = self.active.choices.get(&key).or_else(|| {
                self.english_fallback()
                    .and_then(|english| english.choices.get(&key))
            }) {
                choice.label.clone_from(label);
            } else {
                choice.label = tr(&choice.label);
            }
        }
    }
}

/// A Windows language id written `0x0419` or `1049`.
fn parse_langid(text: &str) -> Option<u16> {
    let text = text.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u16::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

fn locales_root() -> PathBuf {
    [
        // Beside the game (a release), wherever it is started from.
        crate::exploration3d_state::game_dir().join("data/locales"),
        PathBuf::from("demos/demon-queen/data/locales"),
        PathBuf::from("data/locales"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .unwrap_or_else(|| crate::exploration3d_state::source_root().join("data/locales"))
}

fn settings_path() -> PathBuf {
    // The tests never write the player's own settings.
    if cfg!(test) {
        return std::env::temp_dir()
            .join(format!("dq-test-settings-{}", std::process::id()))
            .join("settings.json");
    }
    if let Some(path) = std::env::var_os("DEMON_QUEEN_SETTINGS") {
        return PathBuf::from(path);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("DemonQueen")
            .join("settings.json");
    }
    crate::exploration3d_state::source_root().join("data/settings.json")
}

/// The player's saved language; the tests read their text in the source language instead,
/// whatever the machine they run on last chose.
fn saved_language_outside_tests() -> Option<Language> {
    if cfg!(test) {
        Some(Language::Spanish)
    } else {
        load_saved_language()
    }
}

fn load_saved_language() -> Option<Language> {
    let bytes = fs::read(settings_path()).ok()?;
    let settings: SettingsFile = serde_json::from_slice(&bytes).ok()?;
    if settings.format != SETTINGS_FORMAT || settings.schema != SETTINGS_SCHEMA {
        return None;
    }
    Language::from_code(&settings.language)
}

fn save_language(language: Language) -> Result<()> {
    write_settings(language, saved_reduced_motion())
}

pub fn saved_reduced_motion() -> Option<bool> {
    let settings: SettingsFile = serde_json::from_slice(&fs::read(settings_path()).ok()?).ok()?;
    (settings.format == SETTINGS_FORMAT && settings.schema == SETTINGS_SCHEMA)
        .then_some(settings.reduced_motion)
        .flatten()
}

pub fn save_reduced_motion(value: bool, language: Language) -> Result<()> {
    write_settings(language, Some(value))
}

/// The language and reduced motion into the settings file, keeping the other preferences.
fn write_settings(language: Language, reduced_motion: Option<bool>) -> Result<()> {
    let settings = serde_json::to_value(SettingsFile {
        format: SETTINGS_FORMAT.into(),
        schema: SETTINGS_SCHEMA,
        language: language.code().into(),
        reduced_motion,
    })?;
    crate::exploration3d::prefs::update_file(|map| {
        if let serde_json::Value::Object(ours) = settings {
            map.extend(ours);
        }
    })
    .with_context(|| format!("no se pudo guardar {}", settings_path().display()))
}

pub fn validate_catalogs(prologue_node_ids: &[String], choice_keys: &[String]) -> Result<()> {
    let spanish = Catalog::load(Language::Spanish)?;
    let english = Catalog::load(Language::English)?;
    // A language may leave a node or a choice untranslated: it then reads in English, so English
    // must have every one of them.
    for node in prologue_node_ids {
        if !english.nodes.contains_key(node) {
            bail!("en carece del nodo {node}");
        }
    }
    for choice in choice_keys {
        if !english.choices.contains_key(choice) {
            bail!("en carece de la selección {choice}");
        }
    }
    for language in Language::ALL {
        let catalog = Catalog::load(language)?;
        for key in spanish.ui.keys() {
            if !catalog.ui.contains_key(key) {
                bail!("{} carece de ui.{key}", language.code());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_declared_languages_have_catalog_files() {
        for language in Language::ALL {
            let catalog = Catalog::load(language).expect("declared locale must load");
            assert_eq!(catalog.code, language.code());
        }
    }

    #[test]
    fn clean_install_defaults_to_english_and_lists_it_first() {
        assert_eq!(Language::DEFAULT, Language::English);
        assert_eq!(Language::ALL[0], Language::English);
    }

    #[test]
    fn a_clean_install_starts_in_the_language_windows_is_in() {
        let cases = [
            (0x0c0a, Some(Language::Spanish)),            // es-ES
            (0x080a, Some(Language::Spanish)),            // es-MX
            (0x580a, Some(Language::Spanish)),            // es-419
            (0x0409, Some(Language::English)),            // en-US
            (0x0809, Some(Language::English)),            // en-GB
            (0x0416, Some(Language::PortugueseBrazil)),   // pt-BR
            (0x0816, Some(Language::PortugueseBrazil)),   // pt-PT
            (0x0407, Some(Language::German)),             // de-DE
            (0x0c07, Some(Language::German)),             // de-AT
            (0x040c, Some(Language::French)),             // fr-FR
            (0x0c0c, Some(Language::French)),             // fr-CA
            (0x0410, Some(Language::Italian)),            // it-IT
            (0x0415, Some(Language::Polish)),             // pl-PL
            (0x0419, Some(Language::Russian)),            // ru-RU
            (0x0804, Some(Language::ChineseSimplified)),  // zh-CN
            (0x1004, Some(Language::ChineseSimplified)),  // zh-SG
            (0x0404, Some(Language::ChineseTraditional)), // zh-TW
            (0x0c04, Some(Language::ChineseTraditional)), // zh-HK
            (0x1404, Some(Language::ChineseTraditional)), // zh-MO
            (0x0411, Some(Language::Japanese)),           // ja-JP
            (0x0412, Some(Language::Korean)),             // ko-KR
            (0x040d, Some(Language::Hebrew)),             // he-IL
            (0x0439, Some(Language::Hindi)),              // hi-IN
            (0x041e, Some(Language::Thai)),               // th-TH
            (0x0422, None),                               // uk-UA: English
            (0x041f, None),                               // tr-TR: English
            (0x0413, None),                               // nl-NL: English
        ];
        for (id, want) in cases {
            assert_eq!(Language::from_windows_langid(id), want, "LANGID {id:#06x}");
        }
        for language in Language::ALL {
            assert!(
                cases.iter().any(|&(_, want)| want == Some(language)),
                "{language:?} unreachable"
            );
        }
        assert_eq!(parse_langid("0x0419"), Some(0x0419));
        assert_eq!(parse_langid("1049"), Some(0x0419));
        assert_eq!(
            Language::from_locale_tag("de_DE.UTF-8"),
            Some(Language::German)
        );
        assert_eq!(
            Language::from_locale_tag("zh_TW.UTF-8"),
            Some(Language::ChineseTraditional)
        );
        assert_eq!(
            Language::from_locale_tag("zh-Hans-CN"),
            Some(Language::ChineseSimplified)
        );
        assert_eq!(
            Language::from_locale_tag("pt_PT"),
            Some(Language::PortugueseBrazil)
        );
        assert_eq!(Language::from_locale_tag("C"), None);
    }

    #[test]
    fn language_selector_uses_native_autonyms() {
        assert_eq!(Language::ChineseSimplified.display_name(), "简体中文");
        assert_eq!(Language::ChineseTraditional.display_name(), "繁體中文");
        assert_eq!(Language::Hindi.display_name(), "हिन्दी");
        assert_eq!(Language::Russian.display_name(), "Русский");
        assert_eq!(Language::Japanese.display_name(), "日本語");
        assert_eq!(Language::Korean.display_name(), "한국어");
    }

    #[test]
    fn formatter_replaces_runtime_counts() {
        let localizer = Localizer::for_language(Language::Spanish).unwrap();
        assert_eq!(
            localizer.formatted(
                "gallery.count",
                &[("{current}", "2".into()), ("{total}", "7".into())]
            ),
            "2 de 7"
        );
    }

    #[test]
    fn english_covers_the_released_prologue_for_the_fallback() {
        let path = crate::game::story_path();
        let (nodes, choices) = crate::game::localization_keys(&path).unwrap();
        assert_eq!(nodes.len(), 88);
        assert_eq!(choices.len(), 10);
        validate_catalogs(&nodes, &choices).unwrap();
    }

    #[test]
    fn a_node_the_language_lacks_reads_in_english() {
        let english = Catalog::load(Language::English).unwrap();
        let mut active = english.clone();
        active.nodes.clear();
        let localizer = Localizer {
            language: Language::French,
            spanish: Catalog::load(Language::Spanish).unwrap(),
            english: english.clone(),
            active,
        };
        assert_eq!(
            localizer.node_text("p01_buffet_intro", "texto de la historia"),
            english.nodes["p01_buffet_intro"].as_str()
        );
    }

    #[test]
    fn spanish_keeps_the_story_text_when_it_has_no_entry() {
        let spanish = Catalog::load(Language::Spanish).unwrap();
        let localizer = Localizer {
            language: Language::Spanish,
            spanish: spanish.clone(),
            english: Catalog::load(Language::English).unwrap(),
            active: spanish,
        };
        assert_eq!(
            localizer.node_text("p01_buffet_intro", "texto de la historia"),
            "texto de la historia"
        );
        assert_eq!(localizer.room_name("earth_buffet", "Buffet"), "Buffet");
    }
}
