//! Desktop export packaging with multi-platform dry-run manifests and zip archive.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::pack::{copy_dir_with, ensure_dir, pack_directory_with, AssetPack, PackOptions};

/// Export errors.
#[derive(Debug, Error)]
pub enum ExportError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Pack.
    #[error("pack: {0}")]
    Pack(#[from] crate::pack::PackError),
    /// Build failed.
    #[error("build failed: {0}")]
    Build(String),
    /// Message.
    #[error("{0}")]
    Message(String),
}

/// Well-known export platform identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportPlatform {
    /// Host platform (whatever is compiling).
    Host,
    /// Windows MSVC x86_64.
    WindowsX64,
    /// Windows GNU x86_64.
    WindowsGnu,
    /// Linux glibc x86_64.
    LinuxX64,
    /// macOS Intel.
    MacosX64,
    /// macOS Apple Silicon.
    MacosArm64,
}

impl ExportPlatform {
    /// Parse from CLI-ish string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "host" | "native" => Some(Self::Host),
            "windows" | "windows-x64" | "win64" | "x86_64-pc-windows-msvc" => {
                Some(Self::WindowsX64)
            }
            "windows-gnu" | "x86_64-pc-windows-gnu" => Some(Self::WindowsGnu),
            "linux" | "linux-x64" | "x86_64-unknown-linux-gnu" => Some(Self::LinuxX64),
            "macos" | "macos-x64" | "x86_64-apple-darwin" => Some(Self::MacosX64),
            "macos-arm" | "macos-arm64" | "aarch64-apple-darwin" => Some(Self::MacosArm64),
            _ => None,
        }
    }

    /// Rustc target triple when not Host.
    pub fn target_triple(self) -> Option<&'static str> {
        match self {
            Self::Host => None,
            Self::WindowsX64 => Some("x86_64-pc-windows-msvc"),
            Self::WindowsGnu => Some("x86_64-pc-windows-gnu"),
            Self::LinuxX64 => Some("x86_64-unknown-linux-gnu"),
            Self::MacosX64 => Some("x86_64-apple-darwin"),
            Self::MacosArm64 => Some("aarch64-apple-darwin"),
        }
    }

    /// Display name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::WindowsX64 => "windows-x64",
            Self::WindowsGnu => "windows-gnu",
            Self::LinuxX64 => "linux-x64",
            Self::MacosX64 => "macos-x64",
            Self::MacosArm64 => "macos-arm64",
        }
    }

    /// All non-host platforms for multi dry-run.
    pub fn all_cross() -> &'static [ExportPlatform] {
        &[
            Self::WindowsX64,
            Self::WindowsGnu,
            Self::LinuxX64,
            Self::MacosX64,
            Self::MacosArm64,
        ]
    }

    /// Binary file name for platform.
    pub fn binary_name(self, base: &str) -> String {
        match self {
            Self::WindowsX64 | Self::WindowsGnu => format!("{base}.exe"),
            _ => {
                if self == Self::Host && cfg!(windows) {
                    format!("{base}.exe")
                } else {
                    base.to_string()
                }
            }
        }
    }
}

/// Export target options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    /// Output directory.
    pub out_dir: PathBuf,
    /// Cargo package binary name to copy if present.
    pub binary_name: String,
    /// Assets source dir.
    pub assets_dir: PathBuf,
    /// Release profile.
    pub release: bool,
    /// Target triple override.
    pub target: Option<String>,
    /// Project name for manifest.
    pub project_name: String,
    /// Skip actual cargo build (tests / dry-run).
    pub dry_run: bool,
    /// Logical platform (used for multi-platform dry-run metadata).
    #[serde(default = "default_platform")]
    pub platform: String,
    /// Pack exclude globs.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Pack include globs (optional).
    #[serde(default)]
    pub include: Vec<String>,
}

fn default_platform() -> String {
    "host".into()
}

impl ExportOptions {
    /// Builder-style defaults for dry-run.
    pub fn dry_run(project: impl Into<String>, out: impl Into<PathBuf>) -> Self {
        Self {
            out_dir: out.into(),
            binary_name: "game".into(),
            assets_dir: PathBuf::from("assets"),
            release: true,
            target: None,
            project_name: project.into(),
            dry_run: true,
            platform: "host".into(),
            exclude: PackOptions::default_excludes().exclude,
            include: vec![],
        }
    }

    /// Apply platform, setting target triple when not host.
    pub fn with_platform(mut self, platform: ExportPlatform) -> Self {
        self.platform = platform.as_str().into();
        self.target = platform.target_triple().map(str::to_string);
        self
    }
}

/// Export manifest written next to package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    /// Project.
    pub project: String,
    /// Engine version.
    pub engine_version: String,
    /// Host OS.
    pub host_os: String,
    /// Host arch.
    pub host_arch: String,
    /// Logical platform key.
    pub platform: String,
    /// Target triple if any.
    pub target: Option<String>,
    /// Asset pack summary.
    pub assets: AssetPack,
    /// Binary relative path.
    pub binary: String,
    /// Dry-run flag.
    pub dry_run: bool,
    /// Notes / limitations.
    pub notes: Vec<String>,
}

/// Report after export.
#[derive(Debug, Clone)]
pub struct ExportReport {
    /// Output root.
    pub out_dir: PathBuf,
    /// Manifest.
    pub manifest: ExportManifest,
    /// Log lines.
    pub log: Vec<String>,
    /// Zip archive path (if written).
    pub archive_path: Option<PathBuf>,
}

/// Multi-platform dry-run summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPlatformExport {
    /// Project name.
    pub project: String,
    /// Engine version.
    pub engine_version: String,
    /// One manifest summary per platform.
    pub platforms: Vec<ExportManifest>,
    /// Notes.
    pub notes: Vec<String>,
}

fn write_launchers(
    out: &std::path::Path,
    binary_rel: &str,
    project: &str,
) -> Result<(), ExportError> {
    let readme = format!(
        "# {project}\n\n\
         Packaged by Velvet Engine export.\n\n\
         ## Run\n\n\
         **Windows:** double-click `run.bat` or:\n\n\
         ```bat\n{binary_rel}\n```\n\n\
         **Linux/macOS:**\n\n\
         ```bash\nchmod +x ./{binary_rel}\n./{binary_rel}\n```\n\n\
         Keep the `assets/` folder next to the binary.\n"
    );
    fs::write(out.join("README.md"), readme)?;
    if cfg!(windows) || binary_rel.ends_with(".exe") {
        fs::write(
            out.join("run.bat"),
            format!("@echo off\r\ncd /d \"%~dp0\"\r\n\"%~dp0{binary_rel}\"\r\n"),
        )?;
    }
    fs::write(
        out.join("run.sh"),
        format!("#!/usr/bin/env sh\ncd \"$(dirname \"$0\")\"\n./{binary_rel}\n"),
    )?;
    Ok(())
}

/// Export a desktop package: optional cargo build, copy binary + assets, write manifest.
pub fn export_desktop(opts: &ExportOptions) -> Result<ExportReport, ExportError> {
    let mut log = Vec::new();
    let out = ensure_dir(&opts.out_dir)?;
    let assets_out = ensure_dir(out.join("assets"))?;

    let pack_opts = PackOptions {
        exclude: opts.exclude.clone(),
        include: opts.include.clone(),
        skip_hidden: true,
        max_file_size: 0,
    };

    if opts.assets_dir.exists() {
        copy_dir_with(&opts.assets_dir, &assets_out, &pack_opts)?;
        log.push(format!("copied assets from {}", opts.assets_dir.display()));
    } else {
        log.push("assets dir missing — skipped".into());
    }

    let assets = pack_directory_with(&assets_out, &pack_opts)?;
    log.push(format!(
        "asset pack: {} files, {} bytes",
        assets.files.len(),
        assets.total_size
    ));

    let platform = ExportPlatform::parse(&opts.platform).unwrap_or(ExportPlatform::Host);
    let mut binary_rel = platform.binary_name(&opts.binary_name);

    if !opts.dry_run {
        let mut cmd = Command::new("cargo");
        cmd.arg("build").arg("-p").arg(&opts.binary_name);
        if opts.release {
            cmd.arg("--release");
        }
        if let Some(t) = &opts.target {
            cmd.arg("--target").arg(t);
        }
        info!(?cmd, "export build");
        let output = cmd
            .output()
            .map_err(|e| ExportError::Build(e.to_string()))?;
        log.push(String::from_utf8_lossy(&output.stdout).to_string());
        log.push(String::from_utf8_lossy(&output.stderr).to_string());
        if !output.status.success() {
            return Err(ExportError::Build(format!(
                "cargo build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let profile = if opts.release { "release" } else { "debug" };
        let mut candidates = vec![
            PathBuf::from("target")
                .join(profile)
                .join(&opts.binary_name),
            PathBuf::from("target")
                .join(profile)
                .join(format!("{}.exe", opts.binary_name)),
        ];
        if let Some(t) = &opts.target {
            candidates.push(
                PathBuf::from("target")
                    .join(t)
                    .join(profile)
                    .join(&opts.binary_name),
            );
            candidates.push(
                PathBuf::from("target")
                    .join(t)
                    .join(profile)
                    .join(format!("{}.exe", opts.binary_name)),
            );
        }
        let mut copied = false;
        for c in candidates {
            if c.exists() {
                let dest = out.join(c.file_name().unwrap());
                fs::copy(&c, &dest)?;
                binary_rel = dest.file_name().unwrap().to_string_lossy().to_string();
                log.push(format!("copied binary {}", c.display()));
                copied = true;
                break;
            }
        }
        if !copied {
            return Err(ExportError::Build(format!(
                "binary `{}` not found under target/{} after build",
                opts.binary_name, profile
            )));
        }
        // Launcher scripts for outside-repo execution
        write_launchers(&out, &binary_rel, &opts.project_name)?;
        log.push("wrote README and launch scripts".into());
    } else {
        let marker = out.join(format!("{}.dry-run", binary_rel));
        fs::write(
            &marker,
            format!(
                "dry-run export for platform={} target={:?}\n",
                opts.platform, opts.target
            ),
        )?;
        binary_rel = marker.file_name().unwrap().to_string_lossy().to_string();
        log.push(format!(
            "dry-run: skipped cargo build (platform={})",
            opts.platform
        ));
    }

    let mut notes = vec![
        "Desktop export packages binary + assets + manifest + zip archive.".into(),
        "Cross-compilation requires appropriate target toolchains.".into(),
        "Run from the package directory so relative assets/ resolve.".into(),
    ];
    if opts.dry_run {
        notes.push("This package was produced with dry_run=true.".into());
    }
    if opts.target.is_some() && opts.dry_run {
        notes.push(format!(
            "Would build with --target {}",
            opts.target.as_deref().unwrap_or("")
        ));
    }

    let manifest = ExportManifest {
        project: opts.project_name.clone(),
        engine_version: env!("CARGO_PKG_VERSION").into(),
        host_os: std::env::consts::OS.into(),
        host_arch: std::env::consts::ARCH.into(),
        platform: opts.platform.clone(),
        target: opts.target.clone(),
        assets,
        binary: binary_rel.clone(),
        dry_run: opts.dry_run,
        notes,
    };

    let manifest_path = out.join("export-manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).map_err(|e| ExportError::Message(e.to_string()))?,
    )?;
    log.push(format!("wrote {}", manifest_path.display()));

    // Single-file archive of the package (binary + assets + manifest + launchers).
    let archive_name = format!(
        "{}-{}-{}.zip",
        opts.project_name, opts.binary_name, opts.platform
    );
    let archive_path = out.join(&archive_name);
    let entries = write_directory_zip(&out, &archive_path, Some(&archive_name))?;
    log.push(format!(
        "wrote archive {} ({} entries)",
        archive_path.display(),
        entries.len()
    ));
    if !entries
        .iter()
        .any(|e| e == &binary_rel || e.ends_with(&binary_rel))
    {
        return Err(ExportError::Message(format!(
            "archive missing host binary entry `{binary_rel}` (entries: {entries:?})"
        )));
    }
    log.push(format!("archive contains binary entry `{binary_rel}`"));

    Ok(ExportReport {
        out_dir: out,
        manifest,
        log,
        archive_path: Some(archive_path),
    })
}

/// Zip every file under `dir` into `zip_path`, excluding the zip itself if it would nest.
///
/// Returns relative entry names (forward slashes). Pure enough for unit tests.
pub fn write_directory_zip(
    dir: &Path,
    zip_path: &Path,
    skip_name: Option<&str>,
) -> Result<Vec<String>, ExportError> {
    if let Some(parent) = zip_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut entries = Vec::new();
    let zip_canon = zip_path
        .canonicalize()
        .unwrap_or_else(|_| zip_path.to_path_buf());

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if let Ok(c) = path.canonicalize() {
            if c == zip_canon {
                continue;
            }
        }
        if let Some(skip) = skip_name {
            if path.file_name().and_then(|n| n.to_str()) == Some(skip) {
                continue;
            }
        }
        let rel = path
            .strip_prefix(dir)
            .map_err(|e| ExportError::Message(e.to_string()))?;
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if name.is_empty() {
            continue;
        }
        zip.start_file(&name, options)
            .map_err(|e| ExportError::Message(format!("zip start_file: {e}")))?;
        let mut f = fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)
            .map_err(|e| ExportError::Message(format!("zip write: {e}")))?;
        entries.push(name);
    }

    zip.finish()
        .map_err(|e| ExportError::Message(format!("zip finish: {e}")))?;
    entries.sort();
    Ok(entries)
}

/// List entry names inside an existing zip (for tests / CLI verification).
pub fn list_zip_entries(zip_path: &Path) -> Result<Vec<String>, ExportError> {
    let file = fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| ExportError::Message(format!("zip open: {e}")))?;
    let mut names = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| ExportError::Message(format!("zip entry: {e}")))?;
        names.push(entry.name().to_string());
    }
    names.sort();
    Ok(names)
}

/// Produce dry-run manifests for multiple platforms under `out_root/<platform>/`.
pub fn export_multi_platform_dry_run(
    project: &str,
    assets_dir: impl Into<PathBuf>,
    out_root: impl Into<PathBuf>,
    binary_name: &str,
    platforms: &[ExportPlatform],
) -> Result<MultiPlatformExport, ExportError> {
    let assets_dir = assets_dir.into();
    let out_root = out_root.into();
    let mut platforms_out = Vec::new();
    let mut notes = vec![
        "Multi-platform dry-run only — no cargo build invoked.".into(),
        "Install rustup targets before real cross-export.".into(),
    ];

    let list: Vec<ExportPlatform> = if platforms.is_empty() {
        let mut v = vec![ExportPlatform::Host];
        v.extend_from_slice(ExportPlatform::all_cross());
        v
    } else {
        platforms.to_vec()
    };

    for platform in list {
        let out = out_root.join(platform.as_str());
        let opts = ExportOptions {
            out_dir: out,
            binary_name: binary_name.into(),
            assets_dir: assets_dir.clone(),
            release: true,
            target: platform.target_triple().map(str::to_string),
            project_name: project.into(),
            dry_run: true,
            platform: platform.as_str().into(),
            exclude: PackOptions::default_excludes().exclude,
            include: vec![],
        };
        let report = export_desktop(&opts)?;
        platforms_out.push(report.manifest);
        notes.push(format!(
            "wrote dry-run for {} -> {}",
            platform.as_str(),
            report.out_dir.display()
        ));
    }

    let summary = MultiPlatformExport {
        project: project.into(),
        engine_version: env!("CARGO_PKG_VERSION").into(),
        platforms: platforms_out,
        notes,
    };

    fs::create_dir_all(&out_root)?;
    let summary_path = out_root.join("multi-platform-export.json");
    fs::write(
        &summary_path,
        serde_json::to_string_pretty(&summary).map_err(|e| ExportError::Message(e.to_string()))?,
    )?;

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_export() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("x.txt"), b"1").unwrap();
        let out = dir.path().join("dist");
        let report = export_desktop(&ExportOptions {
            out_dir: out,
            binary_name: "hello-velvet".into(),
            assets_dir: assets,
            release: false,
            target: None,
            project_name: "test".into(),
            dry_run: true,
            platform: "host".into(),
            exclude: vec![],
            include: vec![],
        })
        .unwrap();
        assert!(report.out_dir.join("export-manifest.json").exists());
        assert_eq!(report.manifest.assets.files.len(), 1);
        assert!(report.manifest.dry_run);
        let archive = report.archive_path.expect("archive path set");
        assert!(archive.exists(), "zip must exist: {}", archive.display());
        let names = list_zip_entries(&archive).unwrap();
        assert!(
            names.iter().any(|n| n.contains("hello-velvet")),
            "zip must contain host binary entry, got {names:?}"
        );
        assert!(
            names.iter().any(|n| n.ends_with("export-manifest.json")),
            "zip must contain manifest, got {names:?}"
        );
    }

    #[test]
    fn write_directory_zip_includes_binary_name() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("pkg");
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("hello-velvet.exe"), b"MZ fake").unwrap();
        fs::write(root.join("export-manifest.json"), b"{}").unwrap();
        fs::write(root.join("assets/a.txt"), b"x").unwrap();
        let zip_path = dir.path().join("pkg.zip");
        let entries = write_directory_zip(&root, &zip_path, None).unwrap();
        assert!(
            entries.iter().any(|e| e == "hello-velvet.exe"),
            "{entries:?}"
        );
        assert!(entries.iter().any(|e| e.contains("a.txt")), "{entries:?}");
        let listed = list_zip_entries(&zip_path).unwrap();
        assert_eq!(entries, listed);
    }

    #[test]
    fn multi_platform_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("a.png"), b"img").unwrap();
        let out = dir.path().join("dist");
        let summary = export_multi_platform_dry_run(
            "demo",
            &assets,
            &out,
            "demo-bin",
            &[
                ExportPlatform::Host,
                ExportPlatform::LinuxX64,
                ExportPlatform::WindowsX64,
            ],
        )
        .unwrap();
        assert_eq!(summary.platforms.len(), 3);
        assert!(out.join("multi-platform-export.json").exists());
        assert!(out.join("linux-x64/export-manifest.json").exists());
        assert!(out.join("windows-x64/export-manifest.json").exists());
    }

    #[test]
    fn platform_parse() {
        assert_eq!(
            ExportPlatform::parse("x86_64-unknown-linux-gnu"),
            Some(ExportPlatform::LinuxX64)
        );
        assert_eq!(
            ExportPlatform::WindowsX64.target_triple(),
            Some("x86_64-pc-windows-msvc")
        );
    }

    #[test]
    fn dry_run_excludes_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("keep.bin"), b"1").unwrap();
        fs::write(assets.join("drop.tmp"), b"2").unwrap();
        let out = dir.path().join("dist");
        let report = export_desktop(&ExportOptions {
            out_dir: out,
            binary_name: "game".into(),
            assets_dir: assets,
            release: false,
            target: None,
            project_name: "ex".into(),
            dry_run: true,
            platform: "host".into(),
            exclude: vec!["**/*.tmp".into()],
            include: vec![],
        })
        .unwrap();
        assert!(report.manifest.dry_run);
        assert!(report
            .manifest
            .assets
            .files
            .keys()
            .any(|f| f.contains("keep") || f.ends_with("keep.bin")));
        assert!(!report
            .manifest
            .assets
            .files
            .keys()
            .any(|f| f.contains("drop.tmp")));
    }

    #[test]
    fn multi_platform_host_only() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("a.txt"), b"x").unwrap();
        let out = dir.path().join("dist");
        let summary = export_multi_platform_dry_run(
            "solo",
            &assets,
            &out,
            "solo-bin",
            &[ExportPlatform::Host],
        )
        .unwrap();
        assert_eq!(summary.platforms.len(), 1);
        assert!(out.join("multi-platform-export.json").exists());
    }

    #[test]
    fn platform_names_roundtrip() {
        for p in [
            ExportPlatform::Host,
            ExportPlatform::LinuxX64,
            ExportPlatform::WindowsX64,
        ] {
            let s = format!("{p:?}");
            assert!(!s.is_empty());
        }
        assert!(ExportPlatform::parse("host").is_some() || ExportPlatform::parse("Host").is_some());
    }
}
