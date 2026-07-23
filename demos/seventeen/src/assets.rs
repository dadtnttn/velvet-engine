use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const ASSET_DIR_ENV: &str = "SEVENTEEN_ASSET_DIR";
const NEO_ZERO_ARCHIVE: &str = "neo_zero_V1.0.zip";
const CHARACTER_SHEET: &str = "neo_zero_char_01.png";
const BOT_WHEEL_ARCHIVE: &str = "Bot Wheel.zip";

#[derive(Debug)]
pub struct BotWheelArt {
    pub idle: SpriteSheet,
    pub movement: SpriteSheet,
    pub shoot: SpriteSheet,
    pub dash: SpriteSheet,
    pub damaged: SpriteSheet,
    pub death: SpriteSheet,
    pub wake: SpriteSheet,
    pub charge: SpriteSheet,
}

#[derive(Debug)]
pub struct SpriteSheet {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct LocalArt {
    pub characters: Option<SpriteSheet>,
    pub bot_wheel: Option<BotWheelArt>,
    source_dir: Option<PathBuf>,
    warning: Option<String>,
}

impl LocalArt {
    pub fn discover() -> Self {
        let mut art = Self::default();
        let mut last_warning = None;
        for directory in candidate_directories() {
            if art.characters.is_none() {
                let archive = directory.join(NEO_ZERO_ARCHIVE);
                if archive.is_file() {
                    match load_png_from_zip(&archive, CHARACTER_SHEET) {
                        Ok(sheet) if sheet.width >= 96 && sheet.height >= 288 => {
                            art.characters = Some(sheet);
                            art.source_dir.get_or_insert_with(|| directory.clone());
                        }
                        Ok(sheet) => {
                            last_warning = Some(format!(
                                "{} tiene dimensiones inesperadas ({}x{})",
                                archive.display(),
                                sheet.width,
                                sheet.height
                            ));
                        }
                        Err(error) => last_warning = Some(format!("{error:#}")),
                    }
                }
            }

            if art.bot_wheel.is_none() {
                let archive = directory.join(BOT_WHEEL_ARCHIVE);
                if archive.is_file() {
                    match load_bot_wheel(&archive) {
                        Ok(bot_wheel) => {
                            art.bot_wheel = Some(bot_wheel);
                            art.source_dir.get_or_insert_with(|| directory.clone());
                        }
                        Err(error) => last_warning = Some(format!("{error:#}")),
                    }
                }
            }

            if art.neo_zero_loaded() && art.bot_wheel_loaded() {
                break;
            }
        }
        art.warning = last_warning;
        art
    }

    pub fn neo_zero_loaded(&self) -> bool {
        self.characters.is_some()
    }

    pub fn bot_wheel_loaded(&self) -> bool {
        self.bot_wheel.is_some()
    }

    pub fn status_label(&self) -> &'static str {
        match (self.neo_zero_loaded(), self.bot_wheel_loaded()) {
            (true, true) => "DIECISIETE // ACTIVO",
            (true, false) => "ARCHIVO // ACTIVO",
            (false, true) => "DIECISIETE // ACTIVO",
            (false, false) => "MODO // PROCEDURAL",
        }
    }

    pub fn startup_message(&self) -> String {
        if let Some(directory) = &self.source_dir {
            let names = match (self.neo_zero_loaded(), self.bot_wheel_loaded()) {
                (true, true) => "Neo Zero + Bot Wheel",
                (true, false) => "Neo Zero",
                (false, true) => "Bot Wheel",
                (false, false) => "arte local",
            };
            return format!("{names} cargado desde {}", directory.display());
        }
        if let Some(warning) = &self.warning {
            return format!("arte local omitido: {warning}");
        }
        format!("arte local no encontrado; usa {ASSET_DIR_ENV}=<carpeta> para cargar los paquetes")
    }
}

fn candidate_directories() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os(ASSET_DIR_ENV).filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(path));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(profile).join("Downloads").join("assets"));
    }
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join("assets"));
        candidates.push(current.join("demos").join("seventeen").join("assets"));
    }
    candidates.dedup();
    candidates
}

fn load_bot_wheel(archive: &Path) -> Result<BotWheelArt> {
    Ok(BotWheelArt {
        idle: load_bot_sheet(archive, "static idle.png", 1)?,
        movement: load_bot_sheet(archive, "move with FX.png", 8)?,
        shoot: load_bot_sheet(archive, "shoot with FX.png", 4)?,
        dash: load_bot_sheet(archive, "GAS dash with FX.png", 7)?,
        damaged: load_bot_sheet(archive, "damaged.png", 2)?,
        death: load_bot_sheet(archive, "death.png", 6)?,
        wake: load_bot_sheet(archive, "wake.png", 5)?,
        charge: load_bot_sheet(archive, "charge.png", 4)?,
    })
}

fn load_bot_sheet(archive: &Path, file_name: &str, frames: u32) -> Result<SpriteSheet> {
    let sheet = load_png_from_zip(archive, file_name)?;
    if sheet.width < 40 || sheet.height < 26 * frames {
        bail!(
            "{} en {} tiene dimensiones inesperadas ({}x{}; se esperaban {frames} cuadros)",
            file_name,
            archive.display(),
            sheet.width,
            sheet.height
        );
    }
    Ok(sheet)
}

fn load_png_from_zip(archive_path: &Path, file_name: &str) -> Result<SpriteSheet> {
    let file = File::open(archive_path)
        .with_context(|| format!("abrir paquete {}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("leer paquete {}", archive_path.display()))?;
    let mut bytes = None;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let entry_name = entry.name().replace('\\', "/");
        if entry_name
            .rsplit('/')
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(file_name))
        {
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;
            bytes = Some(buffer);
            break;
        }
    }
    let Some(bytes) = bytes else {
        bail!("{} no contiene {}", archive_path.display(), file_name);
    };
    let decoded = image::load_from_memory(&bytes)
        .with_context(|| format!("decodificar {file_name} desde {}", archive_path.display()))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    Ok(SpriteSheet {
        width,
        height,
        rgba: decoded.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_keep_the_environment_override_first() {
        let original = std::env::var_os(ASSET_DIR_ENV);
        unsafe { std::env::set_var(ASSET_DIR_ENV, "X:/seventeen-art") };
        assert_eq!(
            candidate_directories().first(),
            Some(&PathBuf::from("X:/seventeen-art"))
        );
        match original {
            Some(value) => unsafe { std::env::set_var(ASSET_DIR_ENV, value) },
            None => unsafe { std::env::remove_var(ASSET_DIR_ENV) },
        }
    }
}
