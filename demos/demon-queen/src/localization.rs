//! Runtime localization catalogs and persistent language preference.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    German,
    ChineseSimplified,
    ChineseTraditional,
    Hindi,
    Russian,
    Japanese,
    Korean,
    Latin,
    Esperanto,
}

impl Language {
    /// Locale used by a clean installation. A persisted user choice still wins.
    pub const DEFAULT: Self = Self::English;

    pub const ALL: [Self; 11] = [
        Self::English,
        Self::Spanish,
        Self::German,
        Self::ChineseSimplified,
        Self::ChineseTraditional,
        Self::Hindi,
        Self::Russian,
        Self::Japanese,
        Self::Korean,
        Self::Latin,
        Self::Esperanto,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Spanish => "es",
            Self::English => "en",
            Self::German => "de",
            Self::ChineseSimplified => "zh-Hans",
            Self::ChineseTraditional => "zh-Hant",
            Self::Hindi => "hi",
            Self::Russian => "ru",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::Latin => "la",
            Self::Esperanto => "eo",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Spanish => "Español",
            Self::English => "English",
            Self::German => "Deutsch",
            Self::ChineseSimplified => "简体中文",
            Self::ChineseTraditional => "繁體中文",
            Self::Hindi => "हिन्दी",
            Self::Russian => "Русский",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Latin => "Latine",
            Self::Esperanto => "Esperanto",
        }
    }

    /// Font profile needed to render the language's autonym in the selector.
    /// Latin and Cyrillic labels share the broadly covered default profile.
    pub const fn font_profile(self) -> &'static str {
        match self {
            Self::ChineseSimplified => "zh-Hans",
            Self::ChineseTraditional => "zh-Hant",
            Self::Hindi => "hi",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            _ => "en",
        }
    }

    pub const fn short_code(self) -> &'static str {
        match self {
            Self::Spanish => "ES",
            Self::English => "EN",
            Self::German => "DE",
            Self::ChineseSimplified => "ZH-CN",
            Self::ChineseTraditional => "ZH-TW",
            Self::Hindi => "HI",
            Self::Russian => "RU",
            Self::Japanese => "JA",
            Self::Korean => "KO",
            Self::Latin => "LA",
            Self::Esperanto => "EO",
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    active: Catalog,
}

impl Localizer {
    pub fn load() -> Result<Self> {
        let language = load_saved_language().unwrap_or(Language::DEFAULT);
        Self::for_language(language).or_else(|error| {
            eprintln!("idioma {} no disponible: {error:#}", language.code());
            Self::for_language(Language::DEFAULT)
        })
    }

    pub fn for_language(language: Language) -> Result<Self> {
        let spanish = Catalog::load(Language::Spanish)?;
        let active = if language == Language::Spanish {
            spanish.clone()
        } else {
            Catalog::load(language)?
        };
        Ok(Self {
            language,
            spanish,
            active,
        })
    }

    pub const fn language(&self) -> Language {
        self.language
    }

    pub fn set_language(&mut self, language: Language) -> Result<()> {
        let active = if language == Language::Spanish {
            self.spanish.clone()
        } else {
            Catalog::load(language)?
        };
        self.language = language;
        self.active = active;
        save_language(language)
    }

    pub fn ui<'a>(&'a self, key: &'a str) -> &'a str {
        self.active
            .ui
            .get(key)
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
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn character_name(&self, id: &str, fallback: &str) -> String {
        self.active
            .characters
            .get(id)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn node_text(&self, id: &str, fallback: &str) -> String {
        self.active
            .nodes
            .get(id)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn localize_scene(&self, view: &mut SceneView) {
        view.room_name = self.room_name(&view.room_id, &view.room_name);
        view.speaker = self.character_name(&view.speaker_id, &view.speaker);
        view.text = self.node_text(&view.id, &view.text);
        for choice in &mut view.choices {
            if choice.id == "continue" {
                choice.label = self.ui("common.continue").to_string();
                continue;
            }
            let key = format!("{}.{}", view.id, choice.id);
            if let Some(label) = self.active.choices.get(&key) {
                choice.label.clone_from(label);
            }
        }
    }
}

fn locales_root() -> PathBuf {
    [
        PathBuf::from("demos/demon-queen/data/locales"),
        PathBuf::from("data/locales"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/locales"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/locales"))
}

fn settings_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DEMON_QUEEN_SETTINGS") {
        return PathBuf::from(path);
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("DemonQueen")
            .join("settings.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/settings.json")
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
    let path = settings_path();
    let settings = SettingsFile {
        format: SETTINGS_FORMAT.to_string(),
        schema: SETTINGS_SCHEMA,
        language: language.code().to_string(),
        reduced_motion: saved_reduced_motion(),
    };
    write_json_atomic(&path, &settings)
}

pub fn saved_reduced_motion() -> Option<bool> {
    let settings: SettingsFile = serde_json::from_slice(&fs::read(settings_path()).ok()?).ok()?;
    (settings.format == SETTINGS_FORMAT && settings.schema == SETTINGS_SCHEMA)
        .then_some(settings.reduced_motion)
        .flatten()
}

pub fn save_reduced_motion(value: bool, language: Language) -> Result<()> {
    write_json_atomic(
        &settings_path(),
        &SettingsFile {
            format: SETTINGS_FORMAT.into(),
            schema: SETTINGS_SCHEMA,
            language: language.code().into(),
            reduced_motion: Some(value),
        },
    )
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("ruta sin carpeta padre: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("no se pudo crear {}", parent.display()))?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

pub fn validate_catalogs(prologue_node_ids: &[String], choice_keys: &[String]) -> Result<()> {
    let spanish = Catalog::load(Language::Spanish)?;
    for language in Language::ALL {
        let catalog = Catalog::load(language)?;
        for key in spanish.ui.keys() {
            if !catalog.ui.contains_key(key) {
                bail!("{} carece de ui.{key}", language.code());
            }
        }
        if language == Language::Spanish {
            continue;
        }
        for node in prologue_node_ids {
            if !catalog.nodes.contains_key(node) {
                bail!("{} carece del nodo {node}", language.code());
            }
        }
        for choice in choice_keys {
            if !catalog.choices.contains_key(choice) {
                bail!("{} carece de la selección {choice}", language.code());
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
    fn every_translation_covers_the_released_prologue() {
        let path = crate::game::story_path();
        let (nodes, choices) = crate::game::localization_keys(&path).unwrap();
        assert_eq!(nodes.len(), 88);
        assert_eq!(choices.len(), 10);
        validate_catalogs(&nodes, &choices).unwrap();
    }
}
