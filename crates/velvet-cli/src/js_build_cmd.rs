//! `velvet build --runtime js`: packages a JavaScript game folder as a desktop release.
//!
//! A release is the folder `<project>/release/<id>-<version>/`, holding:
//!
//! - the Velvet Desktop host, copied as `<id>.exe` (no extension on Linux and macOS). On Windows the
//!   game's icon is written into the file's resources;
//! - `game/`, with the web files, `velvet-game.json` and the icon;
//! - the files that `release.include` names, at the top;
//! - `build-info.json`: the game, the engine version, the git commit when there is one, and the
//!   build time in UTC;
//! - `SHA256SUMS`: a checksum of every file above, in the format of `sha256sum`.
//!
//! With `release.archive` set to `"zip"`, or with `--zip`, the folder is also written as
//! `release/<id>-<version>.zip`, a zip of the folder whose entries start with the folder's name.
//!
//! The web files are the project folder itself, or its `dist/` after `pnpm run build` when the
//! project's `package.json` has a `build` script. The host is built with
//! `cargo build -p velvet-desktop --release` in the engine workspace, unless `--host` names a
//! prebuilt executable. Started with no arguments, the host runs `game/` (see the velvet-desktop
//! README). `SOURCE_DATE_EPOCH`, when it is set, is the build time, so the same build gives the same
//! `build-info.json` and archive.
//!
//! The two steps that run other programs are parameters of [`assemble`], so the checks and the
//! copy rules are tested on temporary folders.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
// (the icon is only built into Windows executables, and checked by the tests)
#[cfg(any(windows, test))]
use image::codecs::png::PngEncoder;
#[cfg(any(windows, test))]
use image::imageops::{self, FilterType};
#[cfg(any(windows, test))]
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use serde_json::Value;
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime as ZipDateTime, ZipWriter};

use velvet_desktop::cli::PACKAGED_GAME_DIR;
use velvet_desktop::icon::{self, IconImage};
use velvet_desktop::manifest::{Archive, ReleaseHints, MANIFEST_FILE};
use velvet_desktop::Manifest;

use crate::workspace_cmd;

/// The folder under the project where releases are written.
pub const RELEASE_DIR: &str = "release";

/// The engine folder that holds the host crate. The engine's workspace lists it as a member.
const HOST_CRATE: &str = "crates/velvet-desktop";

/// The host's binary target name, as cargo reports it.
const HOST_BIN: &str = "velvet-desktop";

/// The file at the top of a release that records how it was built.
pub const BUILD_INFO_FILE: &str = "build-info.json";

/// The file at the top of a release that lists a checksum of every file, as `sha256sum` writes it.
pub const CHECKSUM_FILE: &str = "SHA256SUMS";

/// The sides, in pixels, of the images in the program's icon, largest first. Windows picks the size
/// it needs.
#[cfg(any(windows, test))]
const ICON_SIZES: [u32; 5] = [256, 64, 48, 32, 16];

/// An icon whose side is shorter than this is still built, with a warning: it will look soft.
const ICON_WARN_SIDE: u32 = 256;

/// The resource id of the first icon image in the program's file. The other images follow it.
#[cfg(any(windows, test))]
const FIRST_ICON_ID: u16 = 1;

/// The resource id of the icon group, which names the images of the program's icon.
#[cfg(windows)]
const GROUP_ICON_ID: u16 = 1;

/// The type in an icon header: 1 for icons (2 is for cursors).
#[cfg(any(windows, test))]
const ICON_TYPE: u16 = 1;

/// The language of the icon resources: neutral, which is `MAKELANGID(LANG_NEUTRAL, SUBLANG_NEUTRAL)`.
#[cfg(windows)]
const NEUTRAL_LANGUAGE: u16 = 0;

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
    /// Also write the release as a zip archive, `release/<id>-<version>.zip`, whatever the
    /// manifest's `release.archive` says.
    #[arg(long)]
    pub zip: bool,
}

/// Options for [`assemble`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReleaseOptions {
    /// Copy source maps into the game folder.
    pub sourcemaps: bool,
    /// Replace a release that already exists.
    pub force: bool,
    /// Also write the zip archive, as `--zip` does.
    pub zip: bool,
    /// The build time, in whole seconds since 1970 (UTC), which `SOURCE_DATE_EPOCH` sets. None
    /// takes the clock.
    pub source_date_epoch: Option<u64>,
}

/// What [`assemble`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The release folder, `<project>/release/<id>-<version>`.
    pub dir: PathBuf,
    /// The host executable inside `dir`.
    pub host: PathBuf,
    /// The zip archive beside `dir`, when one was written.
    pub archive: Option<PathBuf>,
    /// How many files the release folder holds, the host included.
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
            if js.host.is_some() || js.sourcemaps || js.force || js.zip {
                bail!("--host, --sourcemaps, --force and --zip only apply to `--runtime js`");
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
        zip: js.zip,
        source_date_epoch: source_date_epoch()?,
    };
    let release = match &js.host {
        Some(host) => assemble(&project, options, run_pnpm_build, || Ok(host.clone()))?,
        None => assemble(&project, options, run_pnpm_build, || {
            build_host(&engine_root(&project)?)
        })?,
    };
    println!("release folder: {}", release.dir.display());
    println!("  {} files, {}", release.files, size_text(release.bytes));
    if let Some(archive) = &release.archive {
        println!("  archive: {}", archive.display());
    }
    println!("  run with no arguments: {}", release.host.display());
    Ok(())
}

/// Builds the release folder for the game in `project` (an absolute path).
///
/// `run_web_build` runs `pnpm run build` in the project; it is called only when the project's
/// `package.json` has a `build` script. `host` gives the executable to copy as the game's host.
/// Both are parameters, so that tests can replace the commands.
///
/// Everything that can be checked is checked before the first file is written: the manifest, its
/// release hints, the icon, the included files, and whether the release already exists.
pub fn assemble(
    project: &Path,
    options: ReleaseOptions,
    run_web_build: impl FnOnce(&Path) -> Result<()>,
    host: impl FnOnce() -> Result<PathBuf>,
) -> Result<Release> {
    let manifest = load_manifest(project)?;
    let name = release_name(&manifest)?;
    let hints = ReleaseHints::from_value(manifest.release.as_ref())?;
    let icon = read_icon(project, &manifest)?;
    check_includes(project, &manifest.id, &hints.include)?;
    check_kept(
        &manifest,
        icon.as_ref().map(|plan| plan.relative.as_str()),
        &hints.exclude,
    )?;
    let archive = options.zip || hints.archive == Archive::Zip;

    let release_root = project.join(RELEASE_DIR);
    let dir = release_root.join(&name);
    let zip_path = release_root.join(format!("{name}.zip"));
    if !options.force {
        for taken in [&dir, &zip_path] {
            if taken.exists() {
                bail!(
                    "{} already exists; pass --force to replace it",
                    taken.display()
                );
            }
        }
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

    let plan = Plan {
        project,
        web_root: &web_root,
        manifest: &manifest,
        hints: &hints,
        host_exe: &host_exe,
        host_name: host_file_name(&manifest.id, cfg!(windows)),
        icon,
        sourcemaps: options.sourcemaps,
        built_at: options.source_date_epoch.unwrap_or_else(now_seconds),
        git_commit: git_commit(project),
    };

    // The release is written beside its final place and moved in at the end, so a failed copy
    // never leaves a half-written release behind.
    fs::create_dir_all(&release_root)
        .with_context(|| format!("cannot create {}", release_root.display()))?;
    let staging = release_root.join(format!(".{name}.partial"));
    let archive_partial = release_root.join(format!(".{name}.zip.partial"));
    // Whatever an earlier build left behind is cleared first.
    remove_folder(&staging).with_context(|| format!("cannot clear {}", staging.display()))?;
    remove_file(&archive_partial)
        .with_context(|| format!("cannot clear {}", archive_partial.display()))?;
    let placed = stage(&plan, &staging)
        .and_then(|()| {
            if archive {
                write_zip(&staging, &archive_partial, &name, plan.built_at)
            } else {
                Ok(())
            }
        })
        .and_then(|()| {
            place_release(
                &staging,
                &dir,
                &zip_path,
                archive.then_some(archive_partial.as_path()),
            )
        });
    if let Err(error) = placed {
        // Whichever step failed, the partial files go. If removing them fails too, the next build
        // clears them before it starts.
        let _ = remove_folder(&staging);
        let _ = remove_file(&archive_partial);
        return Err(error);
    }

    let (files, bytes) = measure(&dir)?;
    Ok(Release {
        host: dir.join(&plan.host_name),
        dir,
        archive: archive.then_some(zip_path),
        files,
        bytes,
    })
}

/// Everything that a release is written from, gathered before the first file is copied.
struct Plan<'a> {
    project: &'a Path,
    web_root: &'a Path,
    manifest: &'a Manifest,
    hints: &'a ReleaseHints,
    host_exe: &'a Path,
    host_name: String,
    icon: Option<IconPlan>,
    sourcemaps: bool,
    /// The build time, in seconds since 1970 (UTC).
    built_at: u64,
    /// The commit that the game folder's repository has checked out, when `git` answers.
    git_commit: Option<String>,
}

/// The icon of a release: the path that the manifest names, relative to the game folder, and the
/// decoded image.
struct IconPlan {
    relative: String,
    image: IconImage,
}

/// How many times a file operation is tried in all. A move or a removal can fail for a moment when
/// Windows antivirus holds one of the files.
const FILE_OP_ATTEMPTS: u32 = 5;

/// The wait between two tries of a file operation.
const FILE_OP_WAIT: Duration = Duration::from_millis(200);

/// Puts the finished folder, and the archive when there is one, in place. An older release of the
/// same name is replaced.
fn place_release(
    staging: &Path,
    dir: &Path,
    archive: &Path,
    archive_partial: Option<&Path>,
) -> Result<()> {
    remove_folder(dir).with_context(|| format!("cannot replace {}", dir.display()))?;
    remove_file(archive).with_context(|| format!("cannot replace {}", archive.display()))?;
    move_or_copy_folder(staging, dir)
        .with_context(|| format!("cannot move the release into {}", dir.display()))?;
    if let Some(partial) = archive_partial {
        retrying(|| fs::rename(partial, archive))
            .with_context(|| format!("cannot move the archive into {}", archive.display()))?;
    }
    Ok(())
}

/// Moves the finished folder into place. A folder that another program watches cannot be renamed
/// on Windows: a dev server (Vite) watching the game folder holds a handle on every folder under
/// it, `release/` included, for as long as it runs. When the rename is still refused after the
/// retries, the folder is copied into place instead, and the copy it came from is removed if it
/// can be; the next build clears it otherwise.
fn move_or_copy_folder(from: &Path, to: &Path) -> io::Result<()> {
    match retrying(|| fs::rename(from, to)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            copy_tree(from, to)?;
            let _ = fs::remove_dir_all(from);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

/// Copies a folder and everything in it to `to`, which must not exist yet.
fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    for entry in WalkDir::new(from) {
        let entry = entry.map_err(io::Error::other)?;
        let relative = entry.path().strip_prefix(from).map_err(io::Error::other)?;
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Removes a folder with everything in it. A folder that is not there is not an error.
fn remove_folder(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    retrying(|| fs::remove_dir_all(path))
}

/// Removes a file. A file that is not there is not an error.
fn remove_file(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    retrying(|| fs::remove_file(path))
}

/// Runs a file operation again after a short wait when it fails, up to [`FILE_OP_ATTEMPTS`] tries
/// in all. The error of the last try is the one returned.
fn retrying<T>(mut operation: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 1;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if attempt >= FILE_OP_ATTEMPTS => return Err(error),
            Err(_) => {
                attempt += 1;
                thread::sleep(FILE_OP_WAIT);
            }
        }
    }
}

/// Writes the release contents into `staging`: the game folder, the host executable with its icon,
/// the included files, and the two build files.
fn stage(plan: &Plan, staging: &Path) -> Result<()> {
    let game = staging.join(PACKAGED_GAME_DIR);
    copy_web_files(plan.web_root, &game, plan.sourcemaps, &plan.hints.exclude)?;
    copy_file(&plan.project.join(MANIFEST_FILE), &game.join(MANIFEST_FILE))?;
    let host = staging.join(&plan.host_name);
    copy_file(plan.host_exe, &host)?;
    if let Some(icon) = &plan.icon {
        // The window reads its icon from the game folder, even when the web files come from `dist/`.
        copy_file(
            &plan.project.join(&icon.relative),
            &game.join(&icon.relative),
        )?;
        write_exe_icon(&host, &icon.image)?;
    }
    for path in &plan.hints.include {
        copy_file(&plan.project.join(path), &staging.join(path))?;
    }
    write_build_info(plan, staging)?;
    write_checksums(staging)
}

/// Copies the web files in `from` into `to`, under the rules of [`is_shipped`]. A file that an
/// exclude pattern names is left out.
///
/// A symbolic link to a file is copied as the file it points to. A link to a folder, or a broken
/// link, is refused: the walk does not follow links, so such a folder would be missing.
fn copy_web_files(from: &Path, to: &Path, sourcemaps: bool, exclude: &[String]) -> Result<()> {
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
            // A folder is made for the files that go into it (see `copy_file`). An empty folder is
            // copied as it is, but a folder whose files are all left out does not appear.
            if fs::read_dir(entry.path()).is_ok_and(|mut entries| entries.next().is_none()) {
                fs::create_dir_all(&target)
                    .with_context(|| format!("cannot create {}", target.display()))?;
            }
        } else if file_type.is_symlink() && !fs::metadata(entry.path()).is_ok_and(|m| m.is_file()) {
            bail!(
                "{} is a link to a folder, or a broken link, and a release cannot copy it",
                entry.path().display()
            );
        } else {
            if is_excluded(exclude, &relative_text(from, entry.path())?) {
                continue;
            }
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

/// Writes `build-info.json` at the top of `staging`.
fn write_build_info(plan: &Plan, staging: &Path) -> Result<()> {
    let info = serde_json::json!({
        "id": plan.manifest.id,
        "title": plan.manifest.title,
        "version": plan.manifest.version,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "git_commit": plan.git_commit,
        "built_at": rfc3339(plan.built_at),
    });
    let mut text = serde_json::to_string_pretty(&info)?;
    text.push('\n');
    let path = staging.join(BUILD_INFO_FILE);
    fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))
}

/// Writes `SHA256SUMS` at the top of `folder`: one line per file, `<sha256>  <path>`, in the layout
/// of `sha256sum`, sorted by path. It lists every file of the folder except itself.
fn write_checksums(folder: &Path) -> Result<()> {
    let mut files = Vec::new();
    for entry in WalkDir::new(folder) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative = relative_text(folder, entry.path())?;
            if relative != CHECKSUM_FILE {
                files.push((relative, entry.into_path()));
            }
        }
    }
    files.sort();
    let mut text = String::new();
    for (relative, path) in &files {
        writeln!(text, "{}  {relative}", sha256_of(path)?)?;
    }
    let path = folder.join(CHECKSUM_FILE);
    fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))
}

/// The SHA-256 of a file, in lower-case hexadecimal.
fn sha256_of(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).with_context(|| format!("cannot read {}", path.display()))?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Writes `folder` as a zip archive at `archive`. Its entries start with `prefix/`, so unzipping
/// makes the release folder. The entries are sorted, and they carry the build time, so the same
/// build gives the same archive.
fn write_zip(folder: &Path, archive: &Path, prefix: &str, built_at: u64) -> Result<()> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(folder) {
        let entry = entry?;
        if entry.depth() > 0 {
            let relative = relative_text(folder, entry.path())?;
            entries.push((
                relative,
                entry.path().to_path_buf(),
                entry.file_type().is_dir(),
            ));
        }
    }
    entries.sort();
    let file =
        File::create(archive).with_context(|| format!("cannot create {}", archive.display()))?;
    let mut zip = ZipWriter::new(file);
    let time = zip_time(built_at);
    for (relative, path, is_dir) in entries {
        let name = format!("{prefix}/{relative}");
        let options = zip_options(&path, is_dir, time);
        if is_dir {
            zip.add_directory(format!("{name}/"), options)?;
        } else {
            zip.start_file(name, options)?;
            let mut source =
                File::open(&path).with_context(|| format!("cannot read {}", path.display()))?;
            io::copy(&mut source, &mut zip)
                .with_context(|| format!("cannot read {}", path.display()))?;
        }
    }
    zip.finish()?;
    Ok(())
}

/// The options of one zip entry: deflate-compressed, stamped with the build time, and with the
/// file's permissions. Off Unix the permissions are 644 for a file and 755 for a folder.
fn zip_options(path: &Path, is_dir: bool, time: ZipDateTime) -> SimpleFileOptions {
    let default_mode = if is_dir { 0o755 } else { 0o644 };
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(time)
        .unix_permissions(unix_mode(path).unwrap_or(default_mode))
}

/// The permission bits of a file on Unix, or None elsewhere.
#[cfg(unix)]
fn unix_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions().mode() & 0o777)
}

/// The permission bits of a file on Unix, or None elsewhere.
#[cfg(not(unix))]
fn unix_mode(_path: &Path) -> Option<u32> {
    None
}

/// The build time as a zip timestamp, in UTC. A time that zip cannot hold (before 1980 or after
/// 2107) is written as 1980-01-01.
fn zip_time(seconds: u64) -> ZipDateTime {
    let (year, month, day, hour, minute, second) = utc_parts(seconds);
    u16::try_from(year)
        .ok()
        .filter(|year| (1980..=2107).contains(year))
        .and_then(|year| {
            ZipDateTime::from_date_and_time(
                year,
                month as u8,
                day as u8,
                hour as u8,
                minute as u8,
                second as u8,
            )
            .ok()
        })
        .unwrap_or_default()
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

/// The path of `path` under `root`, with forward slashes: how a release names its files.
fn relative_text(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{} is not in {}", path.display(), root.display()))?;
    let mut parts = Vec::new();
    for part in relative.components() {
        let text = part
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("{} is not a UTF-8 path", path.display()))?;
        parts.push(text);
    }
    Ok(parts.join("/"))
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

/// Reads the icon that the manifest names, when there is one, and checks it: it must be a PNG in
/// the game folder, and square. An icon shorter than [`ICON_WARN_SIDE`] is built, with a warning.
fn read_icon(project: &Path, manifest: &Manifest) -> Result<Option<IconPlan>> {
    let Some(relative) = manifest.icon.clone() else {
        return Ok(None);
    };
    let path = project.join(&relative);
    if !path.is_file() {
        bail!("the icon `{relative}` is not a file in the game folder");
    }
    let image =
        icon::load(&path).with_context(|| format!("the icon `{relative}` cannot be used"))?;
    if image.width != image.height {
        bail!(
            "the icon `{relative}` must be square, but it is {} by {} pixels",
            image.width,
            image.height
        );
    }
    if image.width < ICON_WARN_SIDE {
        eprintln!(
            "warning: the icon `{relative}` is {} pixels: use at least {ICON_WARN_SIDE}, so the \
             program's icon stays sharp",
            image.width
        );
    }
    Ok(Some(IconPlan { relative, image }))
}

/// Checks `release.include`: every path is a file of the game folder, and none takes a name that
/// the release writes itself (the game folder, the build files, or the host).
fn check_includes(project: &Path, id: &str, include: &[String]) -> Result<()> {
    let exe = host_file_name(id, true);
    let plain = host_file_name(id, false);
    for path in include {
        let top = path.split('/').next().unwrap_or(path);
        let reserved = top == PACKAGED_GAME_DIR
            || [BUILD_INFO_FILE, CHECKSUM_FILE, exe.as_str(), plain.as_str()]
                .contains(&path.as_str());
        if reserved {
            bail!("release.include `{path}` is a name that the release writes itself");
        }
        if !project.join(path).is_file() {
            bail!("release.include `{path}` is not a file in the game folder");
        }
    }
    Ok(())
}

/// The entry page and the icon must stay in the release: `release.exclude` may not leave them out.
fn check_kept(manifest: &Manifest, icon: Option<&str>, exclude: &[String]) -> Result<()> {
    for path in std::iter::once(manifest.entry.as_str()).chain(icon) {
        if is_excluded(exclude, path) {
            bail!("release.exclude leaves out `{path}`, which the release needs");
        }
    }
    Ok(())
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

/// The commit that the game folder's repository has checked out (`HEAD`), when `git` answers.
///
/// A game that is not in a repository, and a machine without git, still builds: the commit is then
/// None, and `build-info.json` says `null`.
fn git_commit(project: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["rev-parse", "HEAD"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (commit.len() >= 40 && commit.chars().all(|c| c.is_ascii_hexdigit())).then_some(commit)
}

/// The build time from `SOURCE_DATE_EPOCH` (whole seconds since 1970, UTC), when it is set and not
/// empty. Otherwise None, and the build takes the clock.
fn source_date_epoch() -> Result<Option<u64>> {
    let Some(value) = std::env::var_os("SOURCE_DATE_EPOCH") else {
        return Ok(None);
    };
    let text = value.to_string_lossy();
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    parse_epoch(text).map(Some)
}

/// Parses a `SOURCE_DATE_EPOCH` value: whole seconds since 1970.
pub fn parse_epoch(text: &str) -> Result<u64> {
    text.parse()
        .map_err(|_| anyhow!("SOURCE_DATE_EPOCH must be whole seconds since 1970, not `{text}`"))
}

/// The current time in whole seconds since 1970.
fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// The UTC calendar date and clock time of `seconds` after 1970-01-01T00:00:00Z, as (year, month,
/// day, hour, minute, second).
fn utc_parts(seconds: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // The civil calendar from a count of days since 1970-01-01 (Howard Hinnant's algorithm).
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        year,
        month as u32,
        day as u32,
        (rest / 3_600) as u32,
        (rest % 3_600 / 60) as u32,
        (rest % 60) as u32,
    )
}

/// The UTC time as an RFC 3339 text, such as `2009-02-13T23:31:30Z`.
pub fn rfc3339(seconds: u64) -> String {
    let (year, month, day, hour, minute, second) = utc_parts(seconds);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Whether an exclude pattern names a file path. Both use forward slashes, and the path is relative
/// to the web folder.
fn is_excluded(patterns: &[String], path: &str) -> bool {
    patterns.iter().any(|pattern| glob_matches(pattern, path))
}

/// Whether a glob pattern matches a file path, both with forward slashes.
///
/// `*` matches any characters within one folder name, `?` one character, and a folder name that is
/// `**` any number of folders. The last name of a path is the file's name, so a pattern that ends in
/// `**` matches the files below its folder, and not the folder itself. Matching is case-sensitive.
pub fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    match_folders(&pattern, &path)
}

/// Matches the folder names of a pattern against those of a path.
fn match_folders(pattern: &[&str], path: &[&str]) -> bool {
    match pattern {
        [] => path.is_empty(),
        ["**"] => !path.is_empty(),
        ["**", rest @ ..] => (0..path.len()).any(|skip| match_folders(rest, &path[skip..])),
        [first, rest @ ..] => match path {
            [name, path_rest @ ..] => match_name(first, name) && match_folders(rest, path_rest),
            [] => false,
        },
    }
}

/// Matches one folder name, or a file name, against a pattern of `*` (any characters) and `?` (one
/// character).
fn match_name(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n) = (0, 0);
    // The last `*` seen, with the place it started in the pattern and how much of the name it has
    // taken. A failed match makes the star take one more character.
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, n));
            p += 1;
        } else if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if let Some((star_at, taken)) = star {
            p = star_at + 1;
            n = taken + 1;
            star = Some((star_at, taken + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

/// One image of the program's icon: its side in pixels, and the image as PNG bytes.
#[cfg(any(windows, test))]
struct IconPng {
    size: u32,
    png: Vec<u8>,
}

/// The icon scaled to each of [`ICON_SIZES`], each encoded as a PNG.
#[cfg(any(windows, test))]
fn icon_images(icon: &IconImage) -> Result<Vec<IconPng>> {
    let source = RgbaImage::from_raw(icon.width, icon.height, icon.rgba.clone())
        .ok_or_else(|| anyhow!("the icon's pixels do not match its size"))?;
    ICON_SIZES
        .iter()
        .map(|&size| -> Result<IconPng> {
            let scaled = imageops::resize(&source, size, size, FilterType::Lanczos3);
            let mut png = Vec::new();
            PngEncoder::new(&mut png).write_image(
                scaled.as_raw(),
                size,
                size,
                ExtendedColorType::Rgba8,
            )?;
            Ok(IconPng { size, png })
        })
        .collect()
}

/// The icon group resource: the icon header, then one 14-byte entry per image. An entry gives the
/// image's size, its colour depth, its byte count, and the resource id of its PNG. The layout is that
/// of an `.ico` file's directory, with the file offset replaced by the id.
#[cfg(any(windows, test))]
fn group_resource(images: &[IconPng]) -> Vec<u8> {
    let count = u16::try_from(images.len()).unwrap_or(u16::MAX);
    let mut bytes = Vec::with_capacity(6 + 14 * images.len());
    bytes.extend_from_slice(&0u16.to_le_bytes()); // reserved, always 0
    bytes.extend_from_slice(&ICON_TYPE.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    for (index, image) in images.iter().enumerate() {
        let dimension = dimension_byte(image.size);
        bytes.push(dimension); // width
        bytes.push(dimension); // height
        bytes.push(0); // colours: none, the image has 32 bits per pixel
        bytes.push(0); // reserved
        bytes.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        bytes.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        let length = u32::try_from(image.png.len()).unwrap_or(u32::MAX);
        bytes.extend_from_slice(&length.to_le_bytes());
        // The images are numbered from the first id, in the order of `images`.
        let id = FIRST_ICON_ID + u16::try_from(index).unwrap_or(0);
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    bytes
}

/// The width or height byte of an icon entry. A side of 256 pixels is written as 0, as the format
/// asks.
#[cfg(any(windows, test))]
fn dimension_byte(size: u32) -> u8 {
    u8::try_from(size).unwrap_or(0)
}

/// Writes the icon into the program's file: the images as RT_ICON resources, and the group that
/// names them as RT_GROUP_ICON. On Windows that is the file's icon, with a few tries when Windows
/// holds the file.
#[cfg(windows)]
fn write_exe_icon(exe: &Path, icon: &IconImage) -> Result<()> {
    let images = icon_images(icon)?;
    retrying(|| update_icon_resources(exe, &images))
        .with_context(|| format!("cannot write the icon into {}", exe.display()))
}

/// Off Windows a program's file has no icon resources, so nothing is written. The window still
/// gets its icon from the game folder.
#[cfg(not(windows))]
fn write_exe_icon(_exe: &Path, _icon: &IconImage) -> Result<()> {
    Ok(())
}

/// Replaces the icon resources of the program at `exe` with `images`. The changes are committed
/// only when every write succeeds.
#[cfg(windows)]
fn update_icon_resources(exe: &Path, images: &[IconPng]) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{FALSE, HANDLE, TRUE};
    use windows_sys::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW};

    let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: `path` is a NUL-terminated UTF-16 string that lives until the call returns.
    let handle: HANDLE = unsafe { BeginUpdateResourceW(path.as_ptr(), FALSE) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let written = write_icon_entries(handle, images);
    // Changes are discarded when a write failed. The handle is ended here, once.
    // SAFETY: `handle` came from BeginUpdateResourceW and is not used after this call.
    let ended = unsafe { EndUpdateResourceW(handle, if written.is_ok() { FALSE } else { TRUE }) };
    written?;
    if ended == FALSE {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Adds each image as an RT_ICON resource, and the group as RT_GROUP_ICON, to an open update.
#[cfg(windows)]
fn write_icon_entries(
    handle: windows_sys::Win32::Foundation::HANDLE,
    images: &[IconPng],
) -> io::Result<()> {
    use windows_sys::Win32::Foundation::FALSE;
    use windows_sys::Win32::System::LibraryLoader::UpdateResourceW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON};

    for (index, image) in images.iter().enumerate() {
        let id = FIRST_ICON_ID + u16::try_from(index).unwrap_or(0);
        let length = u32::try_from(image.png.len()).map_err(io::Error::other)?;
        // SAFETY: the data pointer and its length describe `image.png`, which lives until the call
        // returns.
        let ok = unsafe {
            UpdateResourceW(
                handle,
                RT_ICON,
                int_resource(id),
                NEUTRAL_LANGUAGE,
                image.png.as_ptr().cast(),
                length,
            )
        };
        if ok == FALSE {
            return Err(io::Error::last_os_error());
        }
    }
    let group = group_resource(images);
    let length = u32::try_from(group.len()).map_err(io::Error::other)?;
    // SAFETY: as above, for `group`.
    let ok = unsafe {
        UpdateResourceW(
            handle,
            RT_GROUP_ICON,
            int_resource(GROUP_ICON_ID),
            NEUTRAL_LANGUAGE,
            group.as_ptr().cast(),
            length,
        )
    };
    if ok == FALSE {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// A resource type or name given as a number, as the Windows `MAKEINTRESOURCE` macro gives it.
#[cfg(windows)]
fn int_resource(id: u16) -> *const u16 {
    usize::from(id) as *const u16
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

    /// A PNG of the given size, filled with one colour.
    fn write_png(path: &Path, width: u32, height: u32) {
        fs::create_dir_all(path.parent().expect("a file has a folder")).expect("folder");
        let image = RgbaImage::from_pixel(width, height, image::Rgba([200, 60, 20, 255]));
        image.save(path).expect("png");
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

    /// A copy of this test program, to stand in for the built host. On Windows its icon resources
    /// are written, so it has to be a real program.
    fn program_host(root: &Path) -> PathBuf {
        let host = root.join("built-velvet-desktop.exe");
        fs::copy(
            std::env::current_exe().expect("the test program's path"),
            &host,
        )
        .expect("copy");
        host
    }

    fn solid_icon(width: u32, height: u32) -> IconImage {
        IconImage {
            width,
            height,
            rgba: [200u8, 60, 20, 255].repeat((width * height) as usize),
        }
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
        assert!(release.archive.is_none());
        assert!(!project
            .join(RELEASE_DIR)
            .join(".demo-1.2.3.partial")
            .exists());

        let listed = [
            game.join("index.html"),
            game.join("main.js"),
            game.join(MANIFEST_FILE),
            game.join("assets/logo.txt"),
            game.join("assets/release/notes.txt"),
            release.host.clone(),
            release.dir.join(BUILD_INFO_FILE),
            release.dir.join(CHECKSUM_FILE),
        ];
        assert_eq!(
            release.files,
            listed.len() as u64,
            "five game files, the host, build-info.json and SHA256SUMS"
        );
        let expected_bytes: u64 = listed
            .iter()
            .map(|path| fs::metadata(path).expect("listed file").len())
            .sum();
        assert_eq!(release.bytes, expected_bytes);
    }

    #[test]
    fn the_build_files_describe_the_release_and_its_checksums() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(&project.join("pages").join("about.html"), "<p>about</p>");
        let host = fake_host(dir.path());
        let options = ReleaseOptions {
            source_date_epoch: Some(1_234_567_890),
            ..Default::default()
        };

        let release =
            assemble(&project, options, no_web_build, || Ok(host.clone())).expect("packaged");

        let text = fs::read_to_string(release.dir.join(BUILD_INFO_FILE)).expect("build-info");
        let info: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(info["id"], "demo");
        assert_eq!(info["title"], "Demo");
        assert_eq!(info["version"], "1.2.3");
        assert_eq!(info["engine_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(info["built_at"], "2009-02-13T23:31:30Z");
        // The temporary folder is not in a repository, so the commit is null.
        assert!(
            info["git_commit"].is_null()
                || info["git_commit"].as_str().is_some_and(|c| c.len() == 40)
        );

        let sums = fs::read_to_string(release.dir.join(CHECKSUM_FILE)).expect("SHA256SUMS");
        let mut listed = Vec::new();
        for line in sums.lines() {
            let (digest, path) = line.split_once("  ").expect("`digest  path`");
            let bytes = fs::read(release.dir.join(path)).expect("a listed file exists");
            assert_eq!(digest, format!("{:x}", Sha256::digest(&bytes)), "{path}");
            listed.push(path.to_string());
        }
        // Every file of the release is listed, except SHA256SUMS itself.
        let mut everything: Vec<String> = WalkDir::new(&release.dir)
            .into_iter()
            .map(|entry| entry.expect("walk"))
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| relative_text(&release.dir, entry.path()).expect("relative"))
            .filter(|path| path != CHECKSUM_FILE)
            .collect();
        everything.sort();
        assert_eq!(listed, everything);
        assert!(listed.contains(&BUILD_INFO_FILE.to_string()));
        assert!(listed.contains(&host_file_name("demo", cfg!(windows))));
    }

    #[test]
    fn source_maps_are_copied_only_when_asked_for() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(&project.join("main.js.map"), "{}");
        let host = fake_host(dir.path());
        let options = ReleaseOptions {
            sourcemaps: true,
            ..Default::default()
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
            force: true,
            ..Default::default()
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
    fn a_stale_staging_folder_is_cleared_before_the_build() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        let stale = project.join(RELEASE_DIR).join(".demo-1.2.3.partial");
        write(&stale.join("left-over.txt"), "from a build that stopped");
        let host = fake_host(dir.path());

        let release = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(host.clone())
        })
        .expect("packaged");

        assert!(!stale.exists(), "the stale folder is gone");
        assert!(!release.dir.join("left-over.txt").exists());
        assert!(release
            .dir
            .join(PACKAGED_GAME_DIR)
            .join("index.html")
            .is_file());
    }

    #[test]
    fn a_release_that_cannot_be_replaced_leaves_no_staging_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        // A file where the release folder should be: it cannot be removed as a folder.
        let blocked = project.join(RELEASE_DIR).join("demo-1.2.3");
        write(&blocked, "not a folder");
        let host = fake_host(dir.path());
        let forced = ReleaseOptions {
            force: true,
            ..Default::default()
        };

        let error = assemble(&project, forced, no_web_build, || Ok(host.clone()))
            .expect_err("the release cannot be replaced");
        assert!(error.to_string().contains("cannot replace"), "{error:#}");
        assert!(!project
            .join(RELEASE_DIR)
            .join(".demo-1.2.3.partial")
            .exists());
        assert!(blocked.is_file(), "the file in the way is untouched");
    }

    #[test]
    fn included_files_go_to_the_release_root_and_reserved_names_are_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join(MANIFEST_FILE),
            r#"{"id":"demo","title":"Demo","version":"1.2.3","release":{"include":["LICENSE","docs/credits.txt"]}}"#,
        );
        write(&project.join("LICENSE"), "license");
        write(&project.join("docs").join("credits.txt"), "credits");
        let host = fake_host(dir.path());
        let release = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(host.clone())
        })
        .expect("packaged");
        assert_eq!(
            fs::read_to_string(release.dir.join("LICENSE")).expect("license"),
            "license"
        );
        assert!(release.dir.join("docs").join("credits.txt").is_file());

        // Each of these is refused before anything is written, even though a release exists.
        for (include, reason) in [
            ("game/index.html", "writes itself"),
            ("build-info.json", "writes itself"),
            ("demo.exe", "writes itself"),
            ("demo", "writes itself"),
            ("missing.txt", "is not a file"),
        ] {
            write(
                &project.join(MANIFEST_FILE),
                &format!(
                    r#"{{"id":"demo","title":"Demo","version":"1.2.3","release":{{"include":["{include}"]}}}}"#
                ),
            );
            let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
                .expect_err("refused");
            assert!(
                format!("{error:#}").contains(reason),
                "{include}: {error:#}"
            );
        }
    }

    #[test]
    fn the_manifest_hints_leave_web_files_out_but_keep_the_entry_page() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join(MANIFEST_FILE),
            r#"{"id":"demo","title":"Demo","version":"1.2.3","release":{"exclude":["**/*.psd","dev/**"]}}"#,
        );
        write(&project.join("art").join("hero.psd"), "source art");
        write(&project.join("art").join("hero.png"), "picture");
        write(&project.join("dev").join("notes.txt"), "notes");
        write(&project.join("dev").join("deep").join("a.txt"), "deep");
        write(&project.join("devx.txt"), "kept");
        let host = fake_host(dir.path());
        let release = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(host.clone())
        })
        .expect("packaged");
        let game = release.dir.join(PACKAGED_GAME_DIR);
        assert!(game.join("art").join("hero.png").is_file());
        assert!(game.join("devx.txt").is_file());
        assert!(!game.join("art").join("hero.psd").exists());
        assert!(!game.join("dev").join("notes.txt").exists());
        assert!(!game.join("dev").join("deep").join("a.txt").exists());
        assert!(
            !game.join("dev").exists(),
            "a folder with only excluded files does not appear"
        );

        // An exclude that would leave out the entry page is refused.
        write(
            &project.join(MANIFEST_FILE),
            r#"{"id":"demo","title":"Demo","version":"1.2.3","release":{"exclude":["**/*.html"]}}"#,
        );
        let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(
            format!("{error:#}").contains("leaves out `index.html`"),
            "{error:#}"
        );
    }

    #[test]
    fn the_archive_is_written_when_the_manifest_or_the_flag_asks_for_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join(MANIFEST_FILE),
            r#"{"id":"demo","title":"Demo","version":"1.2.3","release":{"archive":"zip"}}"#,
        );
        let host = fake_host(dir.path());
        let release = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(host.clone())
        })
        .expect("packaged");
        let archive = release
            .archive
            .clone()
            .expect("the manifest asks for a zip");
        assert_eq!(archive, project.join(RELEASE_DIR).join("demo-1.2.3.zip"));
        let names: Vec<String> = zip::ZipArchive::new(File::open(&archive).expect("archive"))
            .expect("a zip")
            .file_names()
            .map(str::to_string)
            .collect();
        assert!(
            names.contains(&"demo-1.2.3/game/index.html".to_string()),
            "{names:?}"
        );
        assert!(
            names.contains(&"demo-1.2.3/build-info.json".to_string()),
            "{names:?}"
        );
        assert!(
            names.iter().all(|name| name.starts_with("demo-1.2.3/")),
            "{names:?}"
        );

        // The flag writes an archive when the manifest does not ask, and a release with no archive
        // leaves none behind once it replaces an older one.
        write(&project.join(MANIFEST_FILE), MANIFEST_TEXT);
        let forced = ReleaseOptions {
            zip: true,
            force: true,
            ..Default::default()
        };
        let zipped =
            assemble(&project, forced, no_web_build, || Ok(host.clone())).expect("packaged");
        assert!(zipped.archive.is_some_and(|path| path.is_file()));
        let plain = ReleaseOptions {
            force: true,
            ..Default::default()
        };
        let unzipped =
            assemble(&project, plain, no_web_build, || Ok(host.clone())).expect("packaged");
        assert!(unzipped.archive.is_none());
        assert!(!archive.exists(), "the older archive is replaced");
    }

    #[test]
    fn an_existing_archive_is_refused_unless_forced() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(&project.join(RELEASE_DIR).join("demo-1.2.3.zip"), "old");
        let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
            .expect_err("refused");
        assert!(error.to_string().contains("--force"), "{error:#}");
    }

    #[test]
    fn the_icon_is_copied_into_the_game_folder_and_built_into_the_program() {
        let dir = tempfile::tempdir().expect("temp dir");
        let project = static_project(dir.path());
        write(
            &project.join(MANIFEST_FILE),
            r#"{"id":"demo","title":"Demo","version":"1.2.3","icon":"art/icon.png"}"#,
        );
        write_png(&project.join("art").join("icon.png"), 300, 300);
        let host = program_host(dir.path());
        let release = assemble(&project, ReleaseOptions::default(), no_web_build, || {
            Ok(host.clone())
        })
        .expect("packaged");
        let copied = fs::read(
            release
                .dir
                .join(PACKAGED_GAME_DIR)
                .join("art")
                .join("icon.png"),
        )
        .expect("the window's icon is in the game folder");
        assert_eq!(
            copied,
            fs::read(project.join("art").join("icon.png")).expect("the source icon")
        );
        assert!(release.host.is_file());
    }

    #[test]
    fn a_broken_icon_stops_the_build_before_anything_is_written() {
        /// Makes the icon file of a case, in the project folder.
        type Setup = fn(&Path);
        let cases: [(&str, Setup, &str); 3] = [
            ("a missing icon", |_| {}, "is not a file in the game folder"),
            (
                "a text file",
                |path| write(path, "not a picture"),
                "cannot be used",
            ),
            (
                "a wide picture",
                |path| write_png(path, 300, 200),
                "must be square",
            ),
        ];
        for (case, setup, reason) in cases {
            let dir = tempfile::tempdir().expect("temp dir");
            let project = static_project(dir.path());
            write(
                &project.join(MANIFEST_FILE),
                r#"{"id":"demo","title":"Demo","version":"1.2.3","icon":"icon.png"}"#,
            );
            setup(&project.join("icon.png"));
            let error = assemble(&project, ReleaseOptions::default(), no_web_build, no_host)
                .expect_err("refused");
            assert!(format!("{error:#}").contains(reason), "{case}: {error:#}");
            assert!(
                !project.join(RELEASE_DIR).exists(),
                "{case}: nothing is written"
            );
        }
    }

    #[test]
    fn the_program_icon_has_five_sizes_and_a_group_that_names_them() {
        let images = icon_images(&solid_icon(300, 300)).expect("images");
        let sizes: Vec<u32> = images.iter().map(|image| image.size).collect();
        assert_eq!(sizes, [256, 64, 48, 32, 16]);
        for image in &images {
            assert_eq!(&image.png[..8], [137, 80, 78, 71, 13, 10, 26, 10], "a PNG");
            let decoded = icon::decode_png(&image.png).expect("decodes");
            assert_eq!((decoded.width, decoded.height), (image.size, image.size));
        }

        let group = group_resource(&images);
        assert_eq!(group.len(), 6 + 14 * 5);
        assert_eq!(
            group[..6],
            [0, 0, 1, 0, 5, 0],
            "reserved, icon type, five images"
        );
        // The first entry is the 256-pixel image: a side of 0, one colour plane, 32 bits, resource 1.
        assert_eq!(group[6..8], [0, 0]);
        assert_eq!(group[10..12], [1, 0]);
        assert_eq!(group[12..14], [32, 0]);
        let first_length = images[0].png.len() as u32;
        assert_eq!(group[14..18], first_length.to_le_bytes());
        assert_eq!(group[18..20], [1, 0]);
        // The second entry is the 64-pixel image, resource 2.
        assert_eq!(group[20..22], [64, 64]);
        assert_eq!(group[32..34], [2, 0]);
    }

    #[test]
    fn the_dimension_byte_writes_256_as_zero() {
        assert_eq!(dimension_byte(256), 0);
        assert_eq!(dimension_byte(64), 64);
        assert_eq!(dimension_byte(16), 16);
    }

    #[cfg(windows)]
    #[test]
    fn the_icon_resources_read_back_from_a_copy_of_a_program() {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
        use windows_sys::Win32::System::LibraryLoader::{
            FindResourceExW, LoadLibraryExW, LoadResource, LockResource, SizeofResource,
            LOAD_LIBRARY_AS_DATAFILE,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON};

        /// The bytes of one resource of a loaded module.
        fn resource_bytes(module: HMODULE, kind: *const u16, id: u16) -> Vec<u8> {
            // SAFETY: the module was loaded by LoadLibraryExW and stays loaded until the caller frees
            // it. Each call takes the values that the previous one returned.
            unsafe {
                let info = FindResourceExW(module, kind, int_resource(id), NEUTRAL_LANGUAGE);
                assert!(!info.is_null(), "resource {id} is there");
                let size = SizeofResource(module, info);
                let data = LoadResource(module, info);
                let pointer = LockResource(data).cast::<u8>();
                std::slice::from_raw_parts(pointer, size as usize).to_vec()
            }
        }

        let dir = tempfile::tempdir().expect("temp dir");
        let host = program_host(dir.path());
        let images = icon_images(&solid_icon(300, 300)).expect("images");
        update_icon_resources(&host, &images).expect("written");

        let path: Vec<u16> = host.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `path` is NUL-terminated and lives for the call.
        let module = unsafe {
            LoadLibraryExW(
                path.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_AS_DATAFILE,
            )
        };
        assert!(!module.is_null(), "the copy loads as a data file");
        assert_eq!(
            resource_bytes(module, RT_GROUP_ICON, GROUP_ICON_ID),
            group_resource(&images)
        );
        for (index, image) in images.iter().enumerate() {
            let id = FIRST_ICON_ID + index as u16;
            assert_eq!(resource_bytes(module, RT_ICON, id), image.png, "image {id}");
        }
        // SAFETY: the module was loaded above and is freed once.
        unsafe { FreeLibrary(module) };
    }

    #[test]
    fn a_file_operation_is_tried_again_when_it_fails_for_a_moment() {
        let tries = Cell::new(0);
        let value = retrying(|| {
            tries.set(tries.get() + 1);
            if tries.get() < 3 {
                Err(io::Error::other("held by another program"))
            } else {
                Ok(7)
            }
        })
        .expect("the third try works");
        assert_eq!(value, 7);
        assert_eq!(tries.get(), 3);
    }

    #[test]
    fn a_file_operation_gives_up_after_its_last_try() {
        let tries = Cell::new(0);
        let error = retrying(|| -> io::Result<()> {
            tries.set(tries.get() + 1);
            Err(io::Error::other("still held"))
        })
        .expect_err("it gives up");
        assert_eq!(tries.get(), FILE_OP_ATTEMPTS);
        assert!(error.to_string().contains("still held"));
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
        let zipped = JsBuildArgs {
            zip: true,
            ..Default::default()
        };
        let error =
            cmd_build(PathBuf::from("."), false, None, Runtime::Rust, zipped).expect_err("refused");
        assert!(error.to_string().contains("--zip"), "{error:#}");
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
    fn glob_patterns_match_file_paths_as_documented() {
        let cases = [
            ("**/*.psd", "hero.psd", true),
            ("**/*.psd", "art/hero.psd", true),
            ("**/*.psd", "art/hero.png", false),
            ("dev/**", "dev/notes.txt", true),
            ("dev/**", "dev/a/b.txt", true),
            ("dev/**", "dev", false),
            ("dev/**", "devx/a.txt", false),
            ("*.html", "index.html", true),
            ("*.html", "pages/index.html", false),
            ("a?c.txt", "abc.txt", true),
            ("a?c.txt", "ac.txt", false),
            ("Art/*", "art/x.png", false),
            ("file*.txt", "file-1.txt", true),
            ("file*.txt", "file.txt", true),
            ("img/*.png", "img/x/a.png", false),
            ("**", "anything/at/all.bin", true),
            ("a/**/b.txt", "a/b.txt", true),
            ("a/**/b.txt", "a/x/y/b.txt", true),
            ("a/**/b.txt", "a/x/c.txt", false),
        ];
        for (pattern, path, expected) in cases {
            assert_eq!(
                glob_matches(pattern, path),
                expected,
                "`{pattern}` against `{path}`"
            );
        }
    }

    #[test]
    fn utc_times_are_written_as_rfc_3339() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_234_567_890), "2009-02-13T23:31:30Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_735_689_599), "2024-12-31T23:59:59Z");
    }

    #[test]
    fn source_date_epoch_must_be_whole_seconds() {
        assert_eq!(parse_epoch("1234567890").expect("seconds"), 1_234_567_890);
        assert!(parse_epoch("2009-02-13").is_err());
        assert!(parse_epoch("-1").is_err());
    }

    #[test]
    fn the_sizes_are_read_in_bytes_kibibytes_or_mebibytes() {
        assert_eq!(size_text(0), "0 bytes");
        assert_eq!(size_text(1536), "1.5 KiB");
        assert_eq!(size_text(3 * 1024 * 1024), "3.00 MiB");
    }
}
