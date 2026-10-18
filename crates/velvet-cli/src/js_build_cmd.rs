//! `velvet build --runtime js`: packages a JavaScript game folder as a desktop release.
//!
//! A release is the folder `<project>/release/<id>-<version>/`, holding:
//!
//! - the Velvet Desktop host, copied as `<id>.exe` (no extension on Linux and macOS);
//! - `game/`, with the web files and `velvet-game.json`.
//!
//! The web files are the project folder itself, or its `dist/` after `pnpm run build` when the
//! project's `package.json` has a `build` script. The host is built with
//! `cargo build -p velvet-desktop --release` in the engine workspace, unless `--host` names a
//! prebuilt executable. Started with no arguments, the host runs `game/` (see the velvet-desktop
//! README).
//!
//! The two steps that run other programs are parameters of [`assemble`], so the checks and the
//! copy rules are tested on temporary folders.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use walkdir::{DirEntry, WalkDir};

use velvet_desktop::cli::PACKAGED_GAME_DIR;
use velvet_desktop::manifest::MANIFEST_FILE;
use velvet_desktop::Manifest;

use crate::workspace_cmd;

/// The folder under the project where releases are written.
pub const RELEASE_DIR: &str = "release";

/// The engine folder that holds the host crate. The engine's workspace lists it as a member.
const HOST_CRATE: &str = "crates/velvet-desktop";

/// The host's binary target name, as cargo reports it.
const HOST_BIN: &str = "velvet-desktop";

/// The runtime that `velvet build` packages for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Runtime {
    /// The native Rust engine, built with cargo (the default).
    Rust,
    /// A JavaScript game folder, packaged with the Velvet Desktop host.
    Js,
}

/// Options that only `--runtime js` uses.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct JsBuildArgs {
    /// Copy this prebuilt velvet-desktop executable instead of building the host.
    #[arg(long, value_name = "PATH")]
    pub host: Option<PathBuf>,
    /// Also copy source maps (`*.map`) into the game folder.
    #[arg(long)]
    pub sourcemaps: bool,
    /// Replace the release folder when it already exists.
    #[arg(long)]
    pub force: bool,
}

/// Options for [`assemble`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReleaseOptions {
    /// Copy source maps into the game folder.
    pub sourcemaps: bool,
    /// Replace a release folder that already exists.
    pub force: bool,
}

/// What [`assemble`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The release folder, `<project>/release/<id>-<version>`.
    pub dir: PathBuf,
    /// The host executable inside `dir`.
    pub host: PathBuf,
    /// How many files the release holds, the host included.
    pub files: u64,
    /// Their total size in bytes.
    pub bytes: u64,
}

/// `velvet build`: the Rust build (`--runtime rust`, as before) or a JavaScript release.
pub fn cmd_build(
    path: PathBuf,
    release: bool,
    package: Option<String>,
    runtime: Runtime,
    js: JsBuildArgs,
) -> Result<()> {
    match runtime {
        Runtime::Rust => {
            if js.host.is_some() || js.sourcemaps || js.force {
                bail!("--host, --sourcemaps and --force only apply to `--runtime js`");
            }
            workspace_cmd::cmd_build(path, release, package)
        }
        Runtime::Js => {
            if package.is_some() {
                bail!("--package only applies to `--runtime rust`");
            }
            // The host is always a release build, so `--release` adds nothing here.
            cmd_build_js(&path, &js)
        }
    }
}

/// Packages the game folder at `project` and prints where the release went.
fn cmd_build_js(project: &Path, js: &JsBuildArgs) -> Result<()> {
    let project = std::path::absolute(project)
        .with_context(|| format!("cannot resolve {}", project.display()))?;
    let options = ReleaseOptions {
        sourcemaps: js.sourcemaps,
        force: js.force,
    };
    let release = match &js.host {
        Some(host) => assemble(&project, options, run_pnpm_build, || Ok(host.clone()))?,
        None => assemble(&project, options, run_pnpm_build, || {
            build_host(&engine_root(&project)?)
        })?,
    };
    println!("release folder: {}", release.dir.display());
    println!("  {} files, {}", release.files, size_text(release.bytes));
    println!("  run with no arguments: {}", release.host.display());
    Ok(())
}

/// Builds the release folder for the game in `project` (an absolute path).
///
/// `run_web_build` runs `pnpm run build` in the project; it is called only when the project's
/// `package.json` has a `build` script. `host` gives the executable to copy as the game's host.
/// Both are parameters, so that tests can replace the commands.
pub fn assemble(
    project: &Path,
    options: ReleaseOptions,
    run_web_build: impl FnOnce(&Path) -> Result<()>,
    host: impl FnOnce() -> Result<PathBuf>,
) -> Result<Release> {
    let manifest = load_manifest(project)?;
    let name = release_name(&manifest)?;
    let release_root = project.join(RELEASE_DIR);
    let dir = release_root.join(&name);
    if dir.exists() && !options.force {
        bail!(
            "{} already exists; pass --force to replace it",
            dir.display()
        );
    }

    let web_root = if has_build_script(project)? {
        run_web_build(project)?;
        let dist = project.join("dist");
        if !dist.is_dir() {
            bail!("the web build did not create {}", dist.display());
        }
        dist
    } else {
        project.to_path_buf()
    };
    if !web_root.join(&manifest.entry).is_file() {
        bail!(
            "the entry page `{}` is not in {}",
            manifest.entry,
            web_root.display()
        );
    }

    let host_exe = host()?;
    if !host_exe.is_file() {
        bail!("the host executable {} is not a file", host_exe.display());
    }
    let host_name = host_file_name(&manifest.id, cfg!(windows));

    // The release is written beside its final place and moved in at the end, so a failed copy
    // never leaves a half-written release behind.
    fs::create_dir_all(&release_root)
        .with_context(|| format!("cannot create {}", release_root.display()))?;
    let staging = release_root.join(format!(".{name}.partial"));
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("cannot clear {}", staging.display()))?;
    }
    if let Err(error) = stage(
        &staging,
        &web_root,
        project,
        &host_exe,
        &host_name,
        options.sourcemaps,
    ) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("cannot replace {}", dir.display()))?;
    }
    fs::rename(&staging, &dir)
        .with_context(|| format!("cannot move the release into {}", dir.display()))?;

    let (files, bytes) = measure(&dir)?;
    Ok(Release {
        host: dir.join(&host_name),
        dir,
        files,
        bytes,
    })
}

/// Writes the release contents into `staging`: the game folder and the host executable.
fn stage(
    staging: &Path,
    web_root: &Path,
    project: &Path,
    host_exe: &Path,
    host_name: &str,
    sourcemaps: bool,
) -> Result<()> {
    let game = staging.join(PACKAGED_GAME_DIR);
    copy_web_files(web_root, &game, sourcemaps)?;
    copy_file(&project.join(MANIFEST_FILE), &game.join(MANIFEST_FILE))?;
    copy_file(host_exe, &staging.join(host_name))
}

/// Copies the web files in `from` into `to`, under the rules of [`is_shipped`].
///
/// A symbolic link to a file is copied as the file it points to. A link to a folder, or a broken
/// link, is refused: the walk does not follow links, so such a folder would be missing.
fn copy_web_files(from: &Path, to: &Path, sourcemaps: bool) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("cannot create {}", to.display()))?;
    let walk = WalkDir::new(from)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| is_shipped(entry, sourcemaps));
    for entry in walk {
        let entry = entry?;
        let relative = entry.path().strip_prefix(from)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = to.join(relative);
        let file_type = entry.file_type();
        if file_type.is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("cannot create {}", target.display()))?;
        } else if file_type.is_symlink() && !fs::metadata(entry.path()).is_ok_and(|m| m.is_file()) {
            bail!(
                "{} is a link to a folder, or a broken link, and a release cannot copy it",
                entry.path().display()
            );
        } else {
            copy_file(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Whether a walked entry belongs in the game folder.
///
/// `node_modules` and `.git` folders are left out wherever they are, and the folder `release`
/// only at the top, where releases are written. Source maps are left out unless `sourcemaps`
/// is set.
fn is_shipped(entry: &DirEntry, sourcemaps: bool) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if !entry.file_type().is_dir() {
        return sourcemaps || entry.path().extension().and_then(|ext| ext.to_str()) != Some("map");
    }
    match entry.file_name().to_str() {
        Some("node_modules" | ".git") => false,
        Some("release") => entry.depth() > 1,
        _ => true,
    }
}

/// Copies one file, creating its folder first.
fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::copy(from, to)
        .with_context(|| format!("cannot copy {} to {}", from.display(), to.display()))?;
    Ok(())
}

/// The number of files in `dir`, at any depth, and their total size in bytes.
fn measure(dir: &Path) -> Result<(u64, u64)> {
    let mut files = 0;
    let mut bytes = 0;
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata()?.len();
        }
    }
    Ok((files, bytes))
}

/// Reads and checks the project's `velvet-game.json`.
fn load_manifest(project: &Path) -> Result<Manifest> {
    if !project.join(MANIFEST_FILE).is_file() {
        bail!(
            "{} has no {MANIFEST_FILE}: a JavaScript game folder needs one",
            project.display()
        );
    }
    Ok(Manifest::load(project)?)
}

/// The release folder's name, `<id>-<version>`. The version must be safe in a folder name.
fn release_name(manifest: &Manifest) -> Result<String> {
    let safe = manifest
        .version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'));
    if !safe {
        bail!(
            "the version `{}` cannot name a folder: use letters, digits and `.`, `-`, `_` or `+`",
            manifest.version
        );
    }
    Ok(format!("{}-{}", manifest.id, manifest.version))
}

/// The host's file name in a release: `<id>.exe` on Windows, `<id>` elsewhere.
pub fn host_file_name(id: &str, windows: bool) -> String {
    if windows {
        format!("{id}.exe")
    } else {
        id.to_string()
    }
}

/// Whether the project's `package.json` has a `build` script. Without one, the project is static.
pub fn has_build_script(project: &Path) -> Result<bool> {
    let path = project.join("package.json");
    if !path.is_file() {
        return Ok(false);
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    // Windows editors often write a byte order mark, which serde_json does not accept.
    let package: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .with_context(|| format!("{} is not valid JSON", path.display()))?;
    Ok(package["scripts"]["build"].is_string())
}

/// Runs `pnpm run build` in `project`.
///
/// On Windows pnpm is a `.cmd` shim, so it is named with its extension. The arguments are fixed,
/// so the batch file gets nothing it could misread.
pub fn run_pnpm_build(project: &Path) -> Result<()> {
    let program = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let status = Command::new(program)
        .args(["run", "build"])
        .current_dir(project)
        .status()
        .with_context(|| format!("cannot start `{program} run build` (is pnpm installed?)"))?;
    if !status.success() {
        bail!(
            "`pnpm run build` failed in {} ({status})",
            project.display()
        );
    }
    Ok(())
}

/// The engine workspace for a build of `project`, searched from the project and the current folder.
fn engine_root(project: &Path) -> Result<PathBuf> {
    let mut starts = vec![project.to_path_buf()];
    if let Ok(current) = std::env::current_dir() {
        starts.push(current);
    }
    find_engine_root(&starts).ok_or_else(|| {
        anyhow!(
            "cannot find the engine workspace (a Cargo.toml with [workspace] that lists \
             {HOST_CRATE}) above {} or above the current folder; pass --host with a prebuilt \
             {HOST_BIN} executable",
            project.display()
        )
    })
}

/// The first engine workspace found by walking up from each folder in `starts`.
pub fn find_engine_root(starts: &[PathBuf]) -> Option<PathBuf> {
    starts
        .iter()
        .flat_map(|start| start.ancestors())
        .find(|dir| is_engine_root(dir))
        .map(Path::to_path_buf)
}

/// Whether `dir/Cargo.toml` declares a workspace that lists the host crate.
fn is_engine_root(dir: &Path) -> bool {
    let Ok(text) = fs::read_to_string(dir.join("Cargo.toml")) else {
        return false;
    };
    text.lines().any(|line| line.trim() == "[workspace]")
        && text.contains(&format!("\"{HOST_CRATE}\""))
}

/// Builds the host with `cargo build -p velvet-desktop --release` in `engine_root`.
///
/// Returns the path of the executable that cargo reports. The report is read from cargo's JSON
/// output, so a target directory set anywhere else still works.
pub fn build_host(engine_root: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args([
            "build",
            "-p",
            HOST_BIN,
            "--release",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(engine_root)
        .stderr(Stdio::inherit())
        .output()
        .context("cannot start cargo")?;
    if !output.status.success() {
        bail!(
            "cargo build -p {HOST_BIN} --release failed ({})",
            output.status
        );
    }
    let report = String::from_utf8_lossy(&output.stdout);
    find_built_executable(&report, HOST_BIN)
        .ok_or_else(|| anyhow!("cargo did not report the {HOST_BIN} executable"))
}

/// The executable that cargo's JSON report names for the binary `bin_name`.
pub fn find_built_executable(report: &str, bin_name: &str) -> Option<PathBuf> {
    report
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == bin_name
                && message["target"]["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .filter_map(|message| message["executable"].as_str().map(PathBuf::from))
        .next_back()
}

/// A size for people to read: bytes, KiB or MiB.
pub fn size_text(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let value = bytes as f64;
    if value >= KIB * KIB {
        format!("{:.2} MiB", value / (KIB * KIB))
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    const MANIFEST_TEXT: &str = r#"{"id":"demo","title":"Demo","version":"1.2.3"}"#;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("a file has a folder")).expect("folder");
        fs::write(path, text).expect("file");
    }

    /// A static game folder: the manifest and an index page.
    fn static_project(root: &Path) -> PathBuf {
        let project = root.join("demo");
        write(&project.join(MANIFEST_FILE), MANIFEST_TEXT);
        write(&project.join("index.html"), "<p>hi</p>");
        project
    }

    /// An executable to copy as the host, standing in for the built program.
    fn fake_host(root: &Path) -> PathBuf {
        let host = root.join("built-velvet-desktop");
        write(&host, "host bytes");
        host
    }

    fn no_web_build(_: &Path) -> Result<()> {
        panic!("the web build must not run here");
    }

    fn no_host() -> Result<PathBuf> {
        panic!("the host must not be needed here");
    }

    #[test]
    fn a_project_without_a_manifest_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(&dir.path().join("index.html"), "page");
        let error = assemble(dir.path(), ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(error.to_string().contains(MANIFEST_FILE), "{error:#}");
    }

    #[test]
    fn an_invalid_manifest_is_refused_with_its_reason() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(
            &dir.path().join(MANIFEST_FILE),
            r#"{"id":"Bad Id","title":"Demo"}"#,
        );
        let error = assemble(dir.path(), ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(error.to_string().contains("invalid id"), "{error:#}");
    }

    #[test]
    fn a_version_that_cannot_name_a_folder_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = dir.path().join("demo");
        write(
            &project.join(MANIFEST_FILE),
            r#"{"id":"demo","title":"Demo","version":"1.0/beta"}"#,
        );
        let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(
            error.to_string().contains("cannot name a folder"),
            "{error:#}"
        );
    }

    #[test]
    fn a_static_project_becomes_the_host_and_the_game_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(&project.join("main.js"), "console.log(1)");
        write(&project.join("main.js.map"), "{}");
        write(&project.join("assets").join("logo.txt"), "logo");
        write(
            &project.join("assets").join("release").join("notes.txt"),
            "notes",
        );
        write(
            &project.join("node_modules").join("pkg").join("index.js"),
            "module",
        );
        write(&project.join(".git").join("HEAD"), "ref");
        write(
            &project.join(RELEASE_DIR).join("demo-0.9.0").join("old.txt"),
            "old",
        );
        let host = fake_host(dir.path());

        let release = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(host.clone())
        })
        .expect("packaged");

        assert_eq!(release.dir, project.join(RELEASE_DIR).join("demo-1.2.3"));
        let game = release.dir.join(PACKAGED_GAME_DIR);
        for file in [
            "index.html",
            "main.js",
            "velvet-game.json",
            "assets/logo.txt",
            "assets/release/notes.txt",
        ] {
            assert!(game.join(file).is_file(), "{file} is in the game folder");
        }
        for left_out in ["main.js.map", "node_modules", ".git", "release"] {
            assert!(!game.join(left_out).exists(), "{left_out} is left out");
        }
        assert_eq!(
            release.host,
            release.dir.join(host_file_name("demo", cfg!(windows)))
        );
        assert!(release.host.is_file());
        assert!(!project
            .join(RELEASE_DIR)
            .join(".demo-1.2.3.partial")
            .exists());

        let expected_bytes = "<p>hi</p>".len()
            + "console.log(1)".len()
            + MANIFEST_TEXT.len()
            + "logo".len()
            + "notes".len()
            + "host bytes".len();
        assert_eq!(release.files, 6, "five game files and the host");
        assert_eq!(release.bytes, expected_bytes as u64);
    }

    #[test]
    fn source_maps_are_copied_only_when_asked_for() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(&project.join("main.js.map"), "{}");
        let host = fake_host(dir.path());
        let options = ReleaseOptions {
            sourcemaps: true,
            force: false,
        };
        let release =
            assemble(&project, options, no_web_build, || Ok(host.clone())).expect("packaged");
        assert!(release
            .dir
            .join(PACKAGED_GAME_DIR)
            .join("main.js.map")
            .is_file());
    }

    #[test]
    fn a_project_with_a_build_script_ships_its_dist_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join("package.json"),
            r#"{"scripts":{"build":"vite build"}}"#,
        );
        write(&project.join("src").join("main.ts"), "source");
        let host = fake_host(dir.path());
        let builds = Cell::new(0);

        let release = assemble(
            &project,
            ReleaseOptions::default(),
            |root: &Path| {
                builds.set(builds.get() + 1);
                write(&root.join("dist").join("index.html"), "built page");
                write(&root.join("dist").join("app.js"), "built code");
                Ok(())
            },
            || Ok(host.clone()),
        )
        .expect("packaged");

        assert_eq!(builds.get(), 1, "the web build runs once");
        let game = release.dir.join(PACKAGED_GAME_DIR);
        assert_eq!(
            fs::read_to_string(game.join("index.html")).expect("page"),
            "built page"
        );
        assert!(game.join("app.js").is_file());
        assert!(game.join(MANIFEST_FILE).is_file());
        assert!(!game.join("src").exists());
        assert!(!game.join("package.json").exists());
    }

    #[test]
    fn only_a_build_script_makes_a_project_a_web_build() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(!has_build_script(dir.path()).expect("no package.json"));
        write(
            &dir.path().join("package.json"),
            r#"{"scripts":{"test":"vitest"}}"#,
        );
        assert!(!has_build_script(dir.path()).expect("no build script"));
        write(
            &dir.path().join("package.json"),
            "\u{feff}{\"scripts\":{\"build\":\"vite build\"}}",
        );
        assert!(has_build_script(dir.path()).expect("a byte order mark is accepted"));
        write(&dir.path().join("package.json"), "{ broken");
        assert!(has_build_script(dir.path()).is_err());
    }

    #[test]
    fn a_failed_web_build_leaves_nothing_behind() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join("package.json"),
            r#"{"scripts":{"build":"vite build"}}"#,
        );
        let host = fake_host(dir.path());
        let error = assemble(
            &project,
            ReleaseOptions::default(),
            |_: &Path| bail!("vite failed"),
            || Ok(host.clone()),
        )
        .expect_err("the build failed");
        assert!(format!("{error:#}").contains("vite failed"));
        assert!(!project.join(RELEASE_DIR).exists());
    }

    #[test]
    fn a_web_build_without_dist_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join("package.json"),
            r#"{"scripts":{"build":"vite build"}}"#,
        );
        let host = fake_host(dir.path());
        let error = assemble(
            &project,
            ReleaseOptions::default(),
            |_: &Path| Ok(()),
            || Ok(host.clone()),
        )
        .expect_err("refused");
        assert!(error.to_string().contains("did not create"), "{error:#}");
    }

    #[test]
    fn a_missing_entry_page_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = dir.path().join("demo");
        write(&project.join(MANIFEST_FILE), MANIFEST_TEXT);
        let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(error.to_string().contains("entry page"), "{error:#}");
    }

    #[test]
    fn a_missing_host_is_refused_before_anything_is_written() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        let missing = dir.path().join("missing.exe");
        let error = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(missing.clone())
        })
        .expect_err("refused");
        assert!(error.to_string().contains("not a file"), "{error:#}");
        assert!(!project.join(RELEASE_DIR).exists());
    }

    #[test]
    fn an_existing_release_is_refused_unless_forced() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        let old = project.join(RELEASE_DIR).join("demo-1.2.3");
        write(&old.join("old.txt"), "old");
        let host = fake_host(dir.path());

        let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(error.to_string().contains("--force"), "{error:#}");
        assert!(
            old.join("old.txt").is_file(),
            "the old release is untouched"
        );

        let forced = ReleaseOptions {
            sourcemaps: false,
            force: true,
        };
        let release =
            assemble(&project, forced, no_web_build, || Ok(host.clone())).expect("replaced");
        assert!(!old.join("old.txt").exists());
        assert!(release
            .dir
            .join(PACKAGED_GAME_DIR)
            .join("index.html")
            .is_file());
    }

    #[test]
    fn js_only_options_are_refused_for_the_rust_runtime() {
        let js = JsBuildArgs {
            sourcemaps: true,
            ..Default::default()
        };
        let error =
            cmd_build(PathBuf::from("."), false, None, Runtime::Rust, js).expect_err("refused");
        assert!(error.to_string().contains("--runtime js"), "{error:#}");
    }

    #[test]
    fn package_is_refused_for_the_js_runtime() {
        let error = cmd_build(
            PathBuf::from("."),
            false,
            Some("velvet-cli".into()),
            Runtime::Js,
            JsBuildArgs::default(),
        )
        .expect_err("refused");
        assert!(error.to_string().contains("--runtime rust"), "{error:#}");
    }

    #[test]
    fn the_host_is_named_after_the_game_with_the_platform_extension() {
        assert_eq!(host_file_name("hello-game", true), "hello-game.exe");
        assert_eq!(host_file_name("hello-game", false), "hello-game");
    }

    #[test]
    fn the_host_executable_is_read_from_cargo_json() {
        let report = concat!(
            "   Compiling velvet-desktop v0.3.0\n",
            r#"{"reason":"compiler-artifact","target":{"kind":["lib"],"name":"velvet_desktop"},"executable":null}"#,
            "\n",
            r#"{"reason":"compiler-artifact","target":{"kind":["bin"],"name":"velvet-desktop"},"executable":"/work/target/release/velvet-desktop.exe"}"#,
            "\n",
            r#"{"reason":"build-finished","success":true}"#,
            "\n",
        );
        assert_eq!(
            find_built_executable(report, HOST_BIN),
            Some(PathBuf::from("/work/target/release/velvet-desktop.exe"))
        );
        assert_eq!(find_built_executable(report, "other"), None);
    }

    #[test]
    fn the_engine_is_found_above_a_project_inside_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let engine = dir.path().join("engine");
        write(
            &engine.join("Cargo.toml"),
            "[workspace]\nmembers = [\n    \"crates/velvet-desktop\",\n]\n",
        );
        write(
            &engine
                .join("crates")
                .join("velvet-desktop")
                .join("Cargo.toml"),
            "[package]\n",
        );
        let project = engine
            .join("crates")
            .join("velvet-desktop")
            .join("examples")
            .join("hello-game");
        fs::create_dir_all(&project).expect("project");
        assert_eq!(find_engine_root(&[project]), Some(engine));
    }

    #[test]
    fn a_workspace_without_the_host_is_not_the_engine() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(
            &dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/other\"]\n",
        );
        assert_eq!(
            find_engine_root(&[dir.path().join("sub")]),
            None,
            "a workspace that does not list the host is not the engine"
        );
    }

    #[test]
    fn sizes_are_read_in_bytes_kibibytes_or_mebibytes() {
        assert_eq!(size_text(0), "0 bytes");
        assert_eq!(size_text(1536), "1.5 KiB");
        assert_eq!(size_text(3 * 1024 * 1024), "3.00 MiB");
    }
}
