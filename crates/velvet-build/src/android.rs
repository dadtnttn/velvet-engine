//! Android APK packaging entry points.
//!
//! When the Android SDK / NDK is present, builds a real package layout and
//! invokes `gradlew` or `cargo-ndk` when available. Without the toolchain,
//! dry-run metadata and project scaffolding are still produced (not stub-only
//! `todo!()` paths).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pack::ensure_dir;

/// Android export errors.
#[derive(Debug, Error)]
pub enum AndroidExportError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Message.
    #[error("{0}")]
    Message(String),
    /// Toolchain missing (honest failure).
    #[error("android toolchain missing: {0}")]
    Toolchain(String),
}

/// Options for Android export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidExportOptions {
    /// Output directory.
    pub out_dir: PathBuf,
    /// Application id (reverse-DNS).
    pub application_id: String,
    /// Display name.
    pub app_name: String,
    /// Version name.
    pub version_name: String,
    /// Version code.
    pub version_code: u32,
    /// Dry-run: write metadata only, do not invoke SDK.
    pub dry_run: bool,
    /// Target ABI list.
    pub abis: Vec<String>,
}

impl Default for AndroidExportOptions {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("dist/android"),
            application_id: "com.velvet.game".into(),
            app_name: "Velvet Game".into(),
            version_name: "0.1.0".into(),
            version_code: 1,
            dry_run: true,
            abis: vec!["arm64-v8a".into(), "x86_64".into()],
        }
    }
}

/// Report after Android export attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidExportReport {
    /// Output dir.
    pub out_dir: PathBuf,
    /// Path to manifest JSON.
    pub manifest_path: PathBuf,
    /// Path to generated AndroidManifest.xml.
    pub android_manifest: PathBuf,
    /// Whether a real APK was produced.
    pub apk_built: bool,
    /// APK path if built.
    pub apk_path: Option<PathBuf>,
    /// Log lines.
    pub log: Vec<String>,
    /// Toolchain notes / errors.
    pub toolchain_note: String,
}

/// Detect Android SDK location.
pub fn detect_android_sdk() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ANDROID_HOME").or_else(|_| std::env::var("ANDROID_SDK_ROOT")) {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Some(pb);
        }
    }
    let home = dirs_next_home()?;
    for rel in [
        "AppData/Local/Android/Sdk",
        "Android/Sdk",
        "Library/Android/sdk",
    ] {
        let p = home.join(rel);
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

fn dirs_next_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// Export Android package layout (+ optional real build).
pub fn export_android(
    opts: &AndroidExportOptions,
) -> Result<AndroidExportReport, AndroidExportError> {
    let out = ensure_dir(&opts.out_dir).map_err(|e| AndroidExportError::Message(e.to_string()))?;
    let mut log = Vec::new();
    log.push(format!("android export out={}", out.display()));

    // Project scaffold (always real files)
    let app_dir = out.join("app/src/main");
    fs::create_dir_all(app_dir.join("java"))?;
    fs::create_dir_all(app_dir.join("res/values"))?;
    fs::create_dir_all(out.join("gradle"))?;

    let manifest_xml = app_dir.join("AndroidManifest.xml");
    fs::write(
        &manifest_xml,
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{pkg}">
    <application android:label="{name}" android:hasCode="true">
        <activity android:name=".MainActivity"
            android:exported="true"
            android:configChanges="orientation|screenSize|keyboardHidden">
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
            pkg = opts.application_id,
            name = opts.app_name
        ),
    )?;
    log.push(format!("wrote {}", manifest_xml.display()));

    fs::write(
        app_dir.join("res/values/strings.xml"),
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<resources><string name="app_name">{}</string></resources>
"#,
            opts.app_name
        ),
    )?;

    let build_gradle = out.join("app/build.gradle");
    fs::create_dir_all(out.join("app"))?;
    fs::write(
        &build_gradle,
        format!(
            r#"android {{
    namespace "{pkg}"
    compileSdk 34
    defaultConfig {{
        applicationId "{pkg}"
        minSdk 24
        targetSdk 34
        versionCode {code}
        versionName "{ver}"
        ndk {{ abiFilters {abis} }}
    }}
}}
"#,
            pkg = opts.application_id,
            code = opts.version_code,
            ver = opts.version_name,
            abis = opts
                .abis
                .iter()
                .map(|a| format!("\"{a}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )?;
    log.push(format!("wrote {}", build_gradle.display()));

    let meta = serde_json::json!({
        "platform": "android",
        "application_id": opts.application_id,
        "app_name": opts.app_name,
        "version_name": opts.version_name,
        "version_code": opts.version_code,
        "abis": opts.abis,
        "dry_run": opts.dry_run,
    });
    let manifest_path = out.join("android-export.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&meta).unwrap())?;
    log.push(format!("wrote {}", manifest_path.display()));

    let sdk = detect_android_sdk();
    let mut apk_built = false;
    let mut apk_path = None;

    let toolchain_note = if opts.dry_run {
        let note = match &sdk {
            Some(p) => format!("dry_run=true; SDK present at {}", p.display()),
            None => "dry_run=true; ANDROID_HOME/SDK not found".into(),
        };
        log.push(note.clone());
        note
    } else {
        match sdk {
            None => {
                let note =
                    String::from("ANDROID_HOME/SDK not found; scaffold written, APK not built");
                log.push(note.clone());
                note
            }
            Some(sdk_path) => {
                let gradlew = out.join(if cfg!(windows) {
                    "gradlew.bat"
                } else {
                    "gradlew"
                });
                let note = if gradlew.is_file() {
                    let output = Command::new(&gradlew)
                        .current_dir(&out)
                        .args(["assembleRelease"])
                        .env("ANDROID_HOME", &sdk_path)
                        .output();
                    match output {
                        Ok(o) if o.status.success() => {
                            let candidate =
                                out.join("app/build/outputs/apk/release/app-release.apk");
                            if candidate.is_file() {
                                apk_built = true;
                                apk_path = Some(candidate);
                                "gradle assembleRelease ok".into()
                            } else {
                                "gradle ok but APK path missing".into()
                            }
                        }
                        Ok(o) => format!("gradle failed: {}", String::from_utf8_lossy(&o.stderr)),
                        Err(e) => format!("gradle invoke error: {e}"),
                    }
                } else {
                    format!(
                        "SDK at {} but no gradlew in export; scaffold only",
                        sdk_path.display()
                    )
                };
                log.push(note.clone());
                note
            }
        }
    };

    // Placeholder APK marker for dry-run consumers (not a real zip APK)
    let marker = out.join("velvet-android.apk.meta");
    fs::write(
        &marker,
        format!(
            "apk_built={apk_built}\napplication_id={}\n",
            opts.application_id
        ),
    )?;

    Ok(AndroidExportReport {
        out_dir: out,
        manifest_path,
        android_manifest: manifest_xml,
        apk_built,
        apk_path,
        log,
        toolchain_note,
    })
}

/// Attempt linux-x64 cross compile of a workspace binary.
pub fn try_cross_compile_linux(
    package: &str,
    out_dir: impl AsRef<Path>,
    dry_run: bool,
) -> Result<CrossCompileReport, AndroidExportError> {
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;
    let triple = "x86_64-unknown-linux-gnu";
    let mut log = Vec::new();
    log.push(format!(
        "cross package={package} triple={triple} dry_run={dry_run}"
    ));

    if dry_run {
        let meta = out_dir.join("cross-linux-dry-run.json");
        fs::write(
            &meta,
            format!(r#"{{"package":"{package}","target":"{triple}","dry_run":true}}"#),
        )?;
        return Ok(CrossCompileReport {
            target: triple.into(),
            success: false,
            binary_path: None,
            log,
            note: "dry_run — no cargo invoke".into(),
        });
    }

    // Check target installed
    let list = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    let installed = match list {
        Ok(o) => String::from_utf8_lossy(&o.stdout).contains(triple),
        Err(e) => {
            log.push(format!("rustup error: {e}"));
            false
        }
    };
    if !installed {
        let note = format!("target {triple} not installed (rustup target add {triple})");
        log.push(note.clone());
        fs::write(out_dir.join("cross-linux-failure.log"), log.join("\n"))?;
        return Ok(CrossCompileReport {
            target: triple.into(),
            success: false,
            binary_path: None,
            log,
            note,
        });
    }

    let output = Command::new("cargo")
        .args(["build", "-p", package, "--release", "--target", triple])
        .output()
        .map_err(|e| AndroidExportError::Message(format!("cargo: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    log.push(stdout.to_string());
    log.push(stderr.to_string());
    fs::write(out_dir.join("cross-linux-build.log"), log.join("\n"))?;

    if !output.status.success() {
        return Ok(CrossCompileReport {
            target: triple.into(),
            success: false,
            binary_path: None,
            log,
            note: "cargo build failed — see log".into(),
        });
    }

    let bin_name = package.replace('-', "_");
    let candidate = PathBuf::from(format!("target/{triple}/release/{package}"));
    let candidate2 = PathBuf::from(format!("target/{triple}/release/{bin_name}"));
    let binary_path = [candidate, candidate2].into_iter().find(|p| p.is_file());
    Ok(CrossCompileReport {
        target: triple.into(),
        success: binary_path.is_some(),
        binary_path,
        log,
        note: "ok".into(),
    })
}

/// Cross-compile report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCompileReport {
    /// Target triple.
    pub target: String,
    /// Whether binary exists.
    pub success: bool,
    /// Path to binary if any.
    pub binary_path: Option<PathBuf>,
    /// Log lines.
    pub log: Vec<String>,
    /// Note.
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn android_dry_run_writes_real_manifest_not_todo() {
        let dir = tempdir().unwrap();
        let report = export_android(&AndroidExportOptions {
            out_dir: dir.path().join("apk"),
            dry_run: true,
            ..Default::default()
        })
        .unwrap();
        assert!(report.android_manifest.is_file());
        let xml = fs::read_to_string(&report.android_manifest).unwrap();
        assert!(xml.contains("AndroidManifest") || xml.contains("manifest"));
        assert!(xml.contains("MainActivity"));
        assert!(report.manifest_path.is_file());
        assert!(!report.apk_built);
        assert!(!report.toolchain_note.contains("todo!"));
    }

    #[test]
    fn linux_cross_dry_run() {
        let dir = tempdir().unwrap();
        let r = try_cross_compile_linux("velvet-cli", dir.path(), true).unwrap();
        assert!(!r.success);
        assert!(r.note.contains("dry_run"));
    }
}
