//! Android apps without Gradle: the game's library built with the NDK (`cargo rustc
//! --crate-type cdylib`), resources linked by `aapt2`, the APK written here (native libraries and
//! media stored, the rest deflated), aligned by `zipalign`, signed by `apksigner`; app bundles
//! through `bundletool` and `jarsigner`. Everything lands in
//! `<target>/velvet-android/<package>/<profile>/`.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;

use velvet_assets::AssetIndex;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::android_config::{
    validate, version_code_from, AndroidAbi, AndroidAppConfig, AssetSpec, DEFAULT_STORED_EXTENSIONS,
};
use crate::android_manifest::{self, ManifestValues};
use crate::android_toolchain::AndroidToolchain;
use crate::elf::{self, ElfInfo};

/// Errors are messages for the person running the build.
pub type AndroidResult<T> = Result<T, String>;

/// The local file a release key may be described in (git-ignored; see
/// `docs/platforms/ANDROID.md`).
pub const SIGNING_FILE: &str = "android-signing.local.properties";
/// Environment variable with the release keystore's path.
pub const KEYSTORE_ENV: &str = "VELVET_ANDROID_KEYSTORE";
/// Environment variable with the release key's alias.
pub const KEY_ALIAS_ENV: &str = "VELVET_ANDROID_KEY_ALIAS";
/// Environment variable with the keystore password (passed to the signers by name, never as text).
pub const STORE_PASS_ENV: &str = "VELVET_KEYSTORE_PASS";
/// Environment variable with the key password (optional: the keystore password is tried first).
pub const KEY_PASS_ENV: &str = "VELVET_KEY_PASS";

/// A crate with Android settings, as cargo describes it.
#[derive(Debug, Clone)]
pub struct AndroidProject {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Its Cargo.toml.
    pub manifest_path: PathBuf,
    /// Its folder.
    pub dir: PathBuf,
    /// The workspace's folder.
    pub workspace_root: PathBuf,
    /// Cargo's target folder.
    pub target_dir: PathBuf,
    /// The library target's name (the `.so` is `lib<name>.so`).
    pub lib_name: String,
    /// `[package.metadata.velvet.android]`.
    pub config: AndroidAppConfig,
}

fn cargo_bin() -> std::ffi::OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

impl AndroidProject {
    /// Reads the package through `cargo metadata`: the one named `package`, else the one whose
    /// Cargo.toml is `manifest_path`, else the one in the current folder.
    pub fn load(manifest_path: Option<&Path>, package: Option<&str>) -> AndroidResult<Self> {
        let mut cmd = Command::new(cargo_bin());
        cmd.args(["metadata", "--no-deps", "--format-version", "1"]);
        if let Some(path) = manifest_path {
            cmd.arg("--manifest-path").arg(path);
        }
        let output = cmd.output().map_err(|e| format!("cargo metadata: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let meta: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
        let packages = meta["packages"]
            .as_array()
            .ok_or("cargo metadata listed no packages")?;
        let here = std::env::current_dir()
            .map(|d| d.join("Cargo.toml"))
            .unwrap_or_default();
        let manifest_of =
            |p: &serde_json::Value| PathBuf::from(p["manifest_path"].as_str().unwrap_or(""));
        let chosen = packages.iter().find(|p| match package {
            Some(name) => p["name"].as_str() == Some(name),
            None => match manifest_path {
                Some(path) => same_file(&manifest_of(p), path),
                None => same_file(&manifest_of(p), &here),
            },
        });
        let Some(pkg) = chosen else {
            let with: Vec<&str> = packages
                .iter()
                .filter(|p| !p["metadata"]["velvet"]["android"].is_null())
                .filter_map(|p| p["name"].as_str())
                .collect();
            return Err(format!(
                "which package? pass -p <name>{}",
                if with.is_empty() {
                    String::new()
                } else {
                    format!(" (with Android settings: {})", with.join(", "))
                }
            ));
        };
        let name = pkg["name"].as_str().unwrap_or_default().to_string();
        let manifest_path = manifest_of(pkg);
        let settings = &pkg["metadata"]["velvet"]["android"];
        if settings.is_null() {
            return Err(format!(
                "{} has no [package.metadata.velvet.android] (run `velvet android init -p {name}` for a template)",
                manifest_path.display()
            ));
        }
        let config: AndroidAppConfig = serde_json::from_value(settings.clone())
            .map_err(|e| format!("[package.metadata.velvet.android] of {name}: {e}"))?;
        let problems = validate(&config);
        if !problems.is_empty() {
            return Err(format!(
                "[package.metadata.velvet.android] of {name}:\n  - {}",
                problems.join("\n  - ")
            ));
        }
        let lib_name = pkg["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|t| {
                t["kind"].as_array().is_some_and(|kinds| {
                    kinds.iter().any(|k| {
                        matches!(
                            k.as_str(),
                            Some("lib" | "rlib" | "cdylib" | "dylib" | "staticlib")
                        )
                    })
                })
            })
            .and_then(|t| t["name"].as_str())
            .map(|n| n.replace('-', "_"))
            .ok_or_else(|| {
                format!(
                    "{name} has no library target: put the game in src/lib.rs (see docs/platforms/ANDROID.md)"
                )
            })?;
        Ok(Self {
            version: pkg["version"].as_str().unwrap_or("0.1.0").to_string(),
            dir: manifest_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            manifest_path,
            workspace_root: PathBuf::from(meta["workspace_root"].as_str().unwrap_or(".")),
            target_dir: PathBuf::from(meta["target_directory"].as_str().unwrap_or("target")),
            lib_name,
            name,
            config,
        })
    }

    /// The launcher label.
    pub fn label(&self) -> String {
        self.config
            .label
            .clone()
            .unwrap_or_else(|| self.name.clone())
    }

    /// `versionName`.
    pub fn version_name(&self) -> String {
        self.config
            .version_name
            .clone()
            .unwrap_or_else(|| self.version.clone())
    }

    /// `versionCode`.
    pub fn version_code(&self) -> AndroidResult<u32> {
        match self.config.version_code {
            Some(code) => Ok(code),
            None => version_code_from(&self.version_name()).ok_or_else(|| {
                format!(
                    "set version_code: none can be derived from version {}",
                    self.version_name()
                )
            }),
        }
    }

    /// The native library's file name in the APK.
    pub fn so_name(&self) -> String {
        let name = self.config.lib_name.as_deref().unwrap_or(&self.lib_name);
        format!("lib{name}.so")
    }

    /// The ABIs of the configuration.
    pub fn abis(&self) -> Vec<AndroidAbi> {
        self.config
            .abis
            .iter()
            .filter_map(|a| AndroidAbi::parse(a))
            .collect()
    }

    /// Where this package's Android outputs go for a profile.
    pub fn out_dir(&self, profile: &str) -> PathBuf {
        self.target_dir
            .join("velvet-android")
            .join(&self.name)
            .join(profile)
    }

    /// The extensions stored uncompressed.
    pub fn stored_extensions(&self) -> Vec<String> {
        match &self.config.store_uncompressed {
            Some(list) => list
                .iter()
                .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
                .collect(),
            None => DEFAULT_STORED_EXTENSIONS
                .iter()
                .map(|e| e.to_string())
                .collect(),
        }
    }
}

/// How to build the libraries.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Cargo profile (`release`, `dev`, `fast-release`...).
    pub profile: String,
    /// Parallel jobs for cargo (`-j`).
    pub jobs: Option<u32>,
    /// ABIs to build (default: the configuration's).
    pub abis: Option<Vec<AndroidAbi>>,
}

/// One built native library, checked and ready to package.
#[derive(Debug, Clone)]
pub struct BuiltLibrary {
    /// Its ABI.
    pub abi: AndroidAbi,
    /// The file packaged (stripped when the configuration says so).
    pub packaged: PathBuf,
    /// Cargo's output, with its symbols.
    pub unstripped: PathBuf,
    /// What its ELF header says.
    pub info: ElfInfo,
    /// Other libraries it needs that the APK must carry (`libc++_shared.so`).
    pub extra: Vec<PathBuf>,
}

fn profile_dir(profile: &str) -> &str {
    match profile {
        "dev" | "test" => "debug",
        "release" | "bench" => "release",
        other => other,
    }
}

fn run(cmd: &mut Command, what: &str) -> AndroidResult<()> {
    let status = cmd.status().map_err(|e| format!("{what}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{what} failed ({status})"))
    }
}

fn run_output(cmd: &mut Command, what: &str) -> AndroidResult<String> {
    let output = cmd.output().map_err(|e| format!("{what}: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format!(
            "{what} failed ({}):\n{}{}",
            output.status,
            stdout,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Rust targets rustup has installed (empty when rustup is not there).
pub fn installed_rust_targets(dir: &Path) -> Vec<String> {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Builds the game's library for each ABI with the NDK and checks it (machine, 16 KB pages,
/// libraries it needs).
pub fn build_libraries(
    project: &AndroidProject,
    tools: &AndroidToolchain,
    options: &BuildOptions,
) -> AndroidResult<Vec<BuiltLibrary>> {
    let abis = options.abis.clone().unwrap_or_else(|| project.abis());
    let installed = installed_rust_targets(&project.workspace_root);
    if !installed.is_empty() {
        let missing: Vec<&str> = abis
            .iter()
            .map(|a| a.rust_target())
            .filter(|t| !installed.iter().any(|i| i == t))
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "Rust targets missing: rustup target add {}",
                missing.join(" ")
            ));
        }
    }
    let mut built = Vec::new();
    for abi in abis {
        built.push(build_library(project, tools, abi, options)?);
    }
    Ok(built)
}

fn build_library(
    project: &AndroidProject,
    tools: &AndroidToolchain,
    abi: AndroidAbi,
    options: &BuildOptions,
) -> AndroidResult<BuiltLibrary> {
    let config = &project.config;
    let target = abi.rust_target();
    eprintln!(
        "[velvet android] building {} for {} ({target}, API {}, profile {})",
        project.name,
        abi.name(),
        config.min_sdk,
        options.profile
    );
    let mut cmd = Command::new(cargo_bin());
    cmd.arg("rustc")
        .arg("--manifest-path")
        .arg(&project.manifest_path)
        .args(["--lib", "--crate-type", "cdylib", "--target", target])
        .args(["--profile", &options.profile]);
    if let Some(jobs) = options.jobs {
        cmd.args(["-j", &jobs.to_string()]);
    }
    let features: Vec<&str> = config
        .cargo_features
        .iter()
        .chain(
            config
                .abi_cargo_features
                .get(abi.name())
                .into_iter()
                .flatten(),
        )
        .map(String::as_str)
        .collect();
    if !features.is_empty() {
        cmd.args(["--features", &features.join(",")]);
    }
    if config.no_default_features {
        cmd.arg("--no-default-features");
    }
    // 16 KB pages for Google Play (NDK r28 links so by default; older NDKs need it said).
    // C++ dependencies are compiled against the NDK's libc++: link its static archives into
    // the library, so the app stays one shared library. Some build scripts ask for `stdc++`
    // on every target whose name contains "linux" (meshopt 0.6 does), and Android's
    // libstdc++.so is a stub without exception support: without these archives the loader
    // refuses the library at startup (`__gxx_personality_v0` not found).
    // `-Bsymbolic`: the library's references to its own symbols bind to them (nothing on Android
    // interposes on an app's library), so assembly in a dependency can reach the library's data
    // with PC-relative addressing: rav1d's ARM assembly reads tables Rust exports, which the
    // linker otherwise refuses in a shared library ("recompile with -fPIC").
    cmd.args([
        "--",
        "-C",
        "link-arg=-Wl,-z,max-page-size=16384",
        "-C",
        "link-arg=-Wl,-z,common-page-size=16384",
        "-C",
        "link-arg=-Wl,-Bsymbolic",
        "-C",
        "link-arg=-lc++_static",
        "-C",
        "link-arg=-lc++abi",
    ]);
    cmd.current_dir(&project.workspace_root);
    for (key, value) in tools.cargo_env(abi, config.min_sdk) {
        cmd.env(key, value);
    }
    run(&mut cmd, &format!("cargo rustc for {target}"))?;

    let unstripped = project
        .target_dir
        .join(target)
        .join(profile_dir(&options.profile))
        .join(format!("lib{}.so", project.lib_name));
    if !unstripped.is_file() {
        return Err(format!("cargo built no {}", unstripped.display()));
    }
    let staged_dir = project
        .out_dir(&options.profile)
        .join("lib")
        .join(abi.name());
    fs::create_dir_all(&staged_dir).map_err(|e| e.to_string())?;
    let packaged = staged_dir.join(project.so_name());
    if config.strip {
        run(
            Command::new(tools.llvm_tool("llvm-strip"))
                .arg("--strip-debug")
                .arg(&unstripped)
                .arg("-o")
                .arg(&packaged),
            "llvm-strip",
        )?;
    } else {
        fs::copy(&unstripped, &packaged).map_err(|e| e.to_string())?;
    }
    let info = elf::inspect_elf_file(&packaged)?;
    if info.android_abi() != Some(abi.name()) {
        return Err(format!(
            "{} is not an {} library (machine {})",
            packaged.display(),
            abi.name(),
            info.machine
        ));
    }
    if abi.is_64_bit() && !info.is_16k_aligned() {
        return Err(format!(
            "{} has {}-byte segment alignment; Google Play needs 16 KB (use NDK r28 or newer)",
            packaged.display(),
            info.min_load_align
        ));
    }
    let nm = Command::new(tools.llvm_tool("llvm-nm"))
        .args(["-D", "--undefined-only"])
        .arg(&packaged)
        .output()
        .map_err(|e| format!("llvm-nm: {e}"))?;
    if !nm.status.success() {
        return Err(format!(
            "llvm-nm failed on {}: {}",
            packaged.display(),
            String::from_utf8_lossy(&nm.stderr).trim()
        ));
    }
    // A library that needs libc++_shared.so gets those symbols from the copy packaged below.
    let shared_cxx = info.needed.iter().any(|n| n == "libc++_shared.so");
    let missing = if shared_cxx {
        Vec::new()
    } else {
        missing_cxx_runtime(&String::from_utf8_lossy(&nm.stdout))
    };
    if !missing.is_empty() {
        return Err(format!(
            "{} would not load on Android: it imports C++ runtime symbols that no system library \
             has ({}). A dependency links `stdc++`; libc++ must be linked statically",
            packaged.display(),
            missing.join(", ")
        ));
    }
    let mut extra = Vec::new();
    for needed in &info.needed {
        if needed == "libc++_shared.so" {
            let lib = tools.libcxx_shared(abi);
            let copy = staged_dir.join("libc++_shared.so");
            fs::copy(&lib, &copy).map_err(|e| format!("{}: {e}", lib.display()))?;
            extra.push(copy);
        } else if elf::ANDROID_SYSTEM_LIBRARIES.contains(&needed.as_str()) {
            if let Some(api) = elf::system_library_min_api(needed) {
                if config.min_sdk < api {
                    return Err(format!(
                        "the library needs {needed}, which Android has from API {api}: raise min_sdk to {api}"
                    ));
                }
            }
        } else {
            return Err(format!(
                "the library needs {needed}, which is neither an Android system library nor packaged"
            ));
        }
    }
    eprintln!(
        "[velvet android] {}: {} bytes, {}-byte pages, needs {}",
        packaged.display(),
        fs::metadata(&packaged).map(|m| m.len()).unwrap_or(0),
        info.min_load_align,
        info.needed.join(" ")
    );
    Ok(BuiltLibrary {
        abi,
        packaged,
        unstripped,
        info,
        extra,
    })
}

/// C++ runtime symbols (libc++ / libc++abi) that a library imports with no version, from
/// `llvm-nm -D --undefined-only` output. Versioned imports (`__cxa_atexit@LIBC`) come from
/// Android system libraries; unversioned ones like `__gxx_personality_v0` or `std::__ndk1`
/// names exist in no system library, so the loader refuses the library at startup.
fn missing_cxx_runtime(nm_undefined: &str) -> Vec<String> {
    nm_undefined
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (kind, name) = (fields.next()?, fields.next()?);
            if kind != "U" || name.contains('@') {
                return None;
            }
            let cxx = name.starts_with("__gxx_personality")
                || name.starts_with("__cxa_")
                || name.contains("__ndk1")
                || name.contains("10__cxxabiv1");
            cxx.then(|| name.to_string())
        })
        .collect()
}

/// One file packed under `assets/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFile {
    /// Its path under `assets/` (`/` separators).
    pub path: String,
    /// The file on disk.
    pub source: PathBuf,
    /// Its size.
    pub size: u64,
}

/// The files of a list of asset folders (paths relative to `dir`), checked for clashes. Hidden
/// files (a name starting with `.`) are left out.
pub fn collect_assets(dir: &Path, specs: &[AssetSpec]) -> AndroidResult<Vec<AssetFile>> {
    let mut by_path: BTreeMap<String, AssetFile> = BTreeMap::new();
    for spec in specs {
        let root = dir.join(spec.from());
        let to = spec.to();
        if root.is_file() {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let path = if to.is_empty() { name } else { to.clone() };
            insert_asset(&mut by_path, path, root.clone())?;
            continue;
        }
        if !root.is_dir() {
            return Err(format!("asset folder {} does not exist", root.display()));
        }
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(true)
            .sort_by_file_name()
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&root)
                .map_err(|e| e.to_string())?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if rel.iter().any(|part| part.starts_with('.')) {
                continue;
            }
            let rel = rel.join("/");
            if spec
                .exclude()
                .iter()
                .any(|glob| crate::pack::path_matches(&rel, glob))
            {
                continue;
            }
            let path = if to.is_empty() {
                rel
            } else {
                format!("{to}/{rel}")
            };
            insert_asset(&mut by_path, path, entry.path().to_path_buf())?;
        }
    }
    Ok(by_path.into_values().collect())
}

fn insert_asset(
    by_path: &mut BTreeMap<String, AssetFile>,
    path: String,
    source: PathBuf,
) -> AndroidResult<()> {
    if path == AssetIndex::FILE_NAME {
        return Err(format!("{path} is reserved for the packaged index"));
    }
    let size = fs::metadata(&source).map_err(|e| e.to_string())?.len();
    if size >= u32::MAX as u64 {
        return Err(format!("{} is 4 GB or larger", source.display()));
    }
    if let Some(old) = by_path.insert(
        path.clone(),
        AssetFile {
            path: path.clone(),
            source: source.clone(),
            size,
        },
    ) {
        return Err(format!(
            "assets/{path} comes from both {} and {}",
            old.source.display(),
            source.display()
        ));
    }
    Ok(())
}

/// The index of the packaged files (all of them, base and asset packs: the system merges
/// install-time packs into the same asset manager).
pub fn asset_index<'a>(files: impl IntoIterator<Item = &'a AssetFile>) -> AssetIndex {
    let mut index = AssetIndex::new();
    for file in files {
        index.insert(&file.path, file.size);
    }
    index
}

fn is_stored(path: &str, stored: &[String]) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| stored.iter().any(|s| s.eq_ignore_ascii_case(ext)))
}

/// Writes the manifest and resources and links them with `aapt2`: a binary APK, or the protobuf
/// form an app bundle wants.
pub fn link_resources(
    project: &AndroidProject,
    tools: &AndroidToolchain,
    work: &Path,
    debuggable: bool,
    proto: bool,
) -> AndroidResult<PathBuf> {
    let config = &project.config;
    let _ = fs::remove_dir_all(work);
    fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let version_name = project.version_name();
    let lib_name = project
        .so_name()
        .trim_start_matches("lib")
        .trim_end_matches(".so")
        .to_string();
    let manifest = work.join("AndroidManifest.xml");
    fs::write(
        &manifest,
        android_manifest::manifest_xml(&ManifestValues {
            config,
            version_code: project.version_code()?,
            version_name: &version_name,
            lib_name: &lib_name,
            debuggable,
        }),
    )
    .map_err(|e| e.to_string())?;
    let res = work.join("res");
    let icon = config.icon.as_ref().map(|i| project.dir.join(i));
    android_manifest::write_resources(&res, config, &project.label(), icon.as_deref())?;
    let compiled = work.join("compiled-res.zip");
    run_output(
        Command::new(tools.aapt2())
            .arg("compile")
            .arg("--dir")
            .arg(&res)
            .arg("-o")
            .arg(&compiled),
        "aapt2 compile",
    )?;
    let linked = work.join(if proto {
        "linked-proto.apk"
    } else {
        "linked.apk"
    });
    let mut cmd = Command::new(tools.aapt2());
    cmd.arg("link")
        .arg("-o")
        .arg(&linked)
        .arg("--manifest")
        .arg(&manifest)
        .arg("-I")
        .arg(&tools.android_jar)
        .arg("--auto-add-overlay")
        .arg(&compiled);
    if proto {
        cmd.arg("--proto-format");
    }
    run_output(&mut cmd, "aapt2 link")?;
    Ok(linked)
}

/// What went into a package.
#[derive(Debug, Clone, Default)]
pub struct WrittenPackage {
    /// Entries written.
    pub entries: usize,
    /// Asset files.
    pub assets: usize,
    /// Asset bytes (before compression).
    pub asset_bytes: u64,
    /// Asset files stored uncompressed.
    pub stored_assets: usize,
}

/// How entries of an existing zip are renamed into a new one (`None`: left out).
type Rename = fn(&str) -> Option<String>;

fn write_package(
    out: &Path,
    copy_from: Option<(&Path, Rename)>,
    libs: &[BuiltLibrary],
    assets: &[AssetFile],
    index: Option<&AssetIndex>,
    stored: &[String],
) -> AndroidResult<WrittenPackage> {
    let zip_error = |e: zip::result::ZipError| e.to_string();
    let file = File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(file));
    let mut written = WrittenPackage::default();
    let store = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    if let Some((linked, rename)) = copy_from {
        let mut archive = zip::ZipArchive::new(File::open(linked).map_err(|e| e.to_string())?)
            .map_err(zip_error)?;
        for i in 0..archive.len() {
            let entry = archive.by_index_raw(i).map_err(zip_error)?;
            let Some(name) = rename(entry.name()) else {
                continue;
            };
            zip.raw_copy_file_rename(entry, name).map_err(zip_error)?;
            written.entries += 1;
        }
    }
    for lib in libs {
        for path in std::iter::once(&lib.packaged).chain(&lib.extra) {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            // Stored: Android maps native libraries straight from the APK.
            zip.start_file(format!("lib/{}/{name}", lib.abi.name()), store)
                .map_err(zip_error)?;
            io::copy(&mut File::open(path).map_err(|e| e.to_string())?, &mut zip)
                .map_err(|e| e.to_string())?;
            written.entries += 1;
        }
    }
    for asset in assets {
        let keep = is_stored(&asset.path, stored);
        zip.start_file(
            format!("assets/{}", asset.path),
            if keep { store } else { deflate },
        )
        .map_err(zip_error)?;
        io::copy(
            &mut File::open(&asset.source)
                .map_err(|e| format!("{}: {e}", asset.source.display()))?,
            &mut zip,
        )
        .map_err(|e| e.to_string())?;
        written.entries += 1;
        written.assets += 1;
        written.asset_bytes += asset.size;
        written.stored_assets += usize::from(keep);
    }
    if let Some(index) = index {
        zip.start_file(format!("assets/{}", AssetIndex::FILE_NAME), deflate)
            .map_err(zip_error)?;
        io::Write::write_all(&mut zip, index.to_text().as_bytes()).map_err(|e| e.to_string())?;
        written.entries += 1;
    }
    zip.finish().map_err(zip_error)?;
    if written.entries > u16::MAX as usize {
        return Err(format!(
            "{} entries: more than a ZIP without ZIP64 holds; pack small files into an archive",
            written.entries
        ));
    }
    Ok(written)
}

/// The key a package is signed with.
#[derive(Clone)]
pub enum SigningKey {
    /// The Android debug key (`~/.android/debug.keystore`, alias `androiddebugkey`, password
    /// `android`, as the Android docs publish them): installable for testing, not for stores.
    Debug {
        /// The debug keystore.
        keystore: PathBuf,
    },
    /// The developer's release (upload) key.
    Release {
        /// Keystore file.
        keystore: PathBuf,
        /// Key alias.
        alias: String,
        /// Keystore password, when known (else the signer asks on the terminal).
        store_password: Option<String>,
        /// Key password, when it differs from the keystore's.
        key_password: Option<String>,
    },
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug { keystore } => {
                f.debug_struct("Debug").field("keystore", keystore).finish()
            }
            Self::Release {
                keystore,
                alias,
                store_password,
                key_password,
            } => f
                .debug_struct("Release")
                .field("keystore", keystore)
                .field("alias", alias)
                .field(
                    "store_password",
                    &store_password.as_ref().map(|_| "<hidden>"),
                )
                .field("key_password", &key_password.as_ref().map(|_| "<hidden>"))
                .finish(),
        }
    }
}

impl SigningKey {
    /// The debug key, made with `keytool` (the Android docs' parameters) if it does not exist.
    pub fn debug(tools: &AndroidToolchain) -> AndroidResult<Self> {
        let home = std::env::var_os("ANDROID_USER_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(|h| PathBuf::from(h).join(".android"))
            })
            .ok_or("no home folder for the debug keystore")?;
        let keystore = home.join("debug.keystore");
        if !keystore.is_file() {
            let keytool = tools
                .keytool()
                .ok_or("keytool not found: set JAVA_HOME to a JDK (Android Studio's jbr works)")?;
            fs::create_dir_all(&home).map_err(|e| e.to_string())?;
            run_output(
                Command::new(keytool).args([
                    "-genkeypair",
                    "-keystore",
                    &keystore.to_string_lossy(),
                    "-storepass",
                    "android",
                    "-alias",
                    "androiddebugkey",
                    "-keypass",
                    "android",
                    "-keyalg",
                    "RSA",
                    "-keysize",
                    "2048",
                    "-validity",
                    "10000",
                    "-dname",
                    "CN=Android Debug,O=Android,C=US",
                ]),
                "keytool (debug keystore)",
            )?;
            eprintln!(
                "[velvet android] made the debug keystore {}",
                keystore.display()
            );
        }
        Ok(Self::Debug { keystore })
    }

    /// The release key: from the flags, else the environment (`VELVET_ANDROID_KEYSTORE`,
    /// `VELVET_ANDROID_KEY_ALIAS`, `VELVET_KEYSTORE_PASS`, `VELVET_KEY_PASS`), else the first
    /// `android-signing.local.properties` found in `search` (keys `storeFile`, `keyAlias`,
    /// `storePassword`, `keyPassword`). Missing passwords are asked for by the signer.
    pub fn release(
        keystore: Option<PathBuf>,
        alias: Option<String>,
        signing_file: Option<PathBuf>,
        search: &[PathBuf],
    ) -> AndroidResult<Self> {
        let file = signing_file.or_else(|| {
            search
                .iter()
                .map(|d| d.join(SIGNING_FILE))
                .find(|p| p.is_file())
        });
        let props = match &file {
            Some(path) => read_properties(path)?,
            None => BTreeMap::new(),
        };
        let file_dir = file
            .as_ref()
            .and_then(|f| f.parent())
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let env = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        let keystore = keystore
            .or_else(|| env(KEYSTORE_ENV).map(PathBuf::from))
            .or_else(|| props.get("storeFile").map(|p| file_dir.join(p)))
            .ok_or_else(|| {
                format!(
                    "no release keystore: pass --keystore, set {KEYSTORE_ENV}, or write storeFile in \
                     {SIGNING_FILE} (`velvet android keygen-help` shows how to make a key)"
                )
            })?;
        if !keystore.is_file() {
            return Err(format!(
                "release keystore {} does not exist",
                keystore.display()
            ));
        }
        let alias = alias
            .or_else(|| env(KEY_ALIAS_ENV))
            .or_else(|| props.get("keyAlias").cloned())
            .ok_or_else(|| {
                format!("no key alias: pass --alias, set {KEY_ALIAS_ENV}, or write keyAlias in {SIGNING_FILE}")
            })?;
        Ok(Self::Release {
            keystore,
            alias,
            store_password: env(STORE_PASS_ENV).or_else(|| props.get("storePassword").cloned()),
            key_password: env(KEY_PASS_ENV).or_else(|| props.get("keyPassword").cloned()),
        })
    }

    /// apksigner's key arguments; passwords go through the child's environment, by name.
    fn apksigner_args(&self, cmd: &mut Command) {
        match self {
            Self::Debug { keystore } => {
                cmd.arg("--ks")
                    .arg(keystore)
                    .args(["--ks-key-alias", "androiddebugkey"])
                    .args(["--ks-pass", "pass:android", "--key-pass", "pass:android"]);
            }
            Self::Release {
                keystore,
                alias,
                store_password,
                key_password,
            } => {
                cmd.arg("--ks")
                    .arg(keystore)
                    .args(["--ks-key-alias", alias]);
                if let Some(pass) = store_password {
                    cmd.env(STORE_PASS_ENV, pass)
                        .args(["--ks-pass", &format!("env:{STORE_PASS_ENV}")]);
                }
                if let Some(pass) = key_password {
                    cmd.env(KEY_PASS_ENV, pass)
                        .args(["--key-pass", &format!("env:{KEY_PASS_ENV}")]);
                }
            }
        }
    }

    /// jarsigner's arguments before the file (the alias goes after it).
    fn jarsigner_args(&self, cmd: &mut Command) -> String {
        match self {
            Self::Debug { keystore } => {
                cmd.arg("-keystore").arg(keystore).args([
                    "-storepass",
                    "android",
                    "-keypass",
                    "android",
                ]);
                "androiddebugkey".into()
            }
            Self::Release {
                keystore,
                alias,
                store_password,
                key_password,
            } => {
                cmd.arg("-keystore").arg(keystore);
                if let Some(pass) = store_password {
                    cmd.env(STORE_PASS_ENV, pass)
                        .args(["-storepass:env", STORE_PASS_ENV]);
                }
                if let Some(pass) = key_password {
                    cmd.env(KEY_PASS_ENV, pass)
                        .args(["-keypass:env", KEY_PASS_ENV]);
                }
                alias.clone()
            }
        }
    }

    /// Whether this is the debug key.
    pub fn is_debug(&self) -> bool {
        matches!(self, Self::Debug { .. })
    }
}

/// `key=value` lines (Java `.properties`, without escapes); `#` and `!` start comments.
pub fn read_properties(path: &Path) -> AndroidResult<BTreeMap<String, String>> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!'))
        .filter_map(|l| {
            let (k, v) = l.split_once('=').or_else(|| l.split_once(':'))?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect())
}

fn java(tools: &AndroidToolchain) -> AndroidResult<&Path> {
    tools
        .java
        .as_deref()
        .ok_or_else(|| "java not found: set JAVA_HOME to a JDK (Android Studio's jbr works)".into())
}

/// Aligns an APK (`zipalign -P 16`: native libraries on 16 KB pages, other stored entries on 4
/// bytes).
pub fn zipalign(tools: &AndroidToolchain, input: &Path, output: &Path) -> AndroidResult<()> {
    run_output(
        Command::new(tools.zipalign())
            .args(["-f", "-P", "16", "4"])
            .arg(input)
            .arg(output),
        "zipalign",
    )
    .map(|_| ())
}

/// Checks an APK's alignment (`zipalign -c -P 16 4`).
pub fn zipalign_check(tools: &AndroidToolchain, apk: &Path) -> AndroidResult<String> {
    let out = run_output(
        Command::new(tools.zipalign())
            .args(["-c", "-P", "16", "4"])
            .arg(apk),
        "zipalign -c",
    )?;
    Ok(out
        .lines()
        .last()
        .unwrap_or("Verification successful")
        .to_string())
}

/// Signs an APK with apksigner (schemes chosen by apksigner for `min_sdk`: v2 and v3 from API 24).
pub fn sign_apk(
    tools: &AndroidToolchain,
    key: &SigningKey,
    input: &Path,
    output: &Path,
    min_sdk: u32,
) -> AndroidResult<()> {
    let mut cmd = Command::new(java(tools)?);
    cmd.arg("-jar")
        .arg(tools.apksigner_jar())
        .arg("sign")
        .args(["--min-sdk-version", &min_sdk.to_string()])
        .args(["--v4-signing-enabled", "false"]);
    key.apksigner_args(&mut cmd);
    cmd.arg("--out").arg(output).arg(input);
    // Not run_output: apksigner may ask for a password on the terminal.
    run(&mut cmd, "apksigner sign")
}

/// `apksigner verify --verbose --print-certs`.
pub fn verify_apk(tools: &AndroidToolchain, apk: &Path) -> AndroidResult<String> {
    run_output(
        Command::new(java(tools)?)
            .arg("-jar")
            .arg(tools.apksigner_jar())
            .args(["verify", "--verbose", "--print-certs"])
            .arg(apk),
        "apksigner verify",
    )
}

/// `aapt2 dump badging`: package, versions, SDKs, label, ABIs as Android reads them.
pub fn badging(tools: &AndroidToolchain, apk: &Path) -> AndroidResult<String> {
    run_output(
        Command::new(tools.aapt2())
            .args(["dump", "badging"])
            .arg(apk),
        "aapt2 dump badging",
    )
}

/// What a package (APK, app bundle or module zip) holds.
#[derive(Debug, Clone, Default)]
pub struct PackageContents {
    /// Entries.
    pub entries: usize,
    /// The file's size.
    pub file_size: u64,
    /// Native libraries: path, size, stored (not compressed), data offset.
    pub libraries: Vec<(String, u64, bool, u64)>,
    /// Asset files (the index not counted) and their uncompressed bytes.
    pub assets: usize,
    /// Uncompressed size of the assets.
    pub asset_bytes: u64,
    /// Assets stored uncompressed.
    pub stored_assets: usize,
    /// Whether the asset index is there.
    pub has_index: bool,
    /// Manifests (`AndroidManifest.xml`, `<module>/manifest/AndroidManifest.xml`).
    pub manifests: Vec<String>,
    /// Signature files under `META-INF/`.
    pub signature_files: Vec<String>,
}

impl PackageContents {
    /// Native libraries not stored on a 16 KB boundary (an APK's must be).
    pub fn misaligned_libraries(&self) -> Vec<&str> {
        self.libraries
            .iter()
            .filter(|(_, _, stored, offset)| !*stored || offset % 16384 != 0)
            .map(|(p, ..)| p.as_str())
            .collect()
    }
}

/// Lists a package's contents.
pub fn inspect_package(path: &Path) -> AndroidResult<PackageContents> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut contents = PackageContents {
        entries: archive.len(),
        file_size,
        ..Default::default()
    };
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        // An app bundle's entries sit in module folders (`base/lib/...`).
        let inner = match name.split_once('/') {
            Some((first, rest))
                if !["lib", "assets", "res", "META-INF", "manifest", "root"].contains(&first)
                    && (rest.starts_with("lib/")
                        || rest.starts_with("assets/")
                        || rest.starts_with("manifest/")) =>
            {
                rest.to_string()
            }
            _ => name.clone(),
        };
        if inner.starts_with("lib/") && inner.ends_with(".so") {
            contents.libraries.push((
                name.clone(),
                entry.size(),
                entry.compression() == CompressionMethod::Stored,
                entry.data_start(),
            ));
        } else if let Some(asset) = inner.strip_prefix("assets/") {
            if asset == AssetIndex::FILE_NAME {
                contents.has_index = true;
            } else if !asset.ends_with('/') {
                contents.assets += 1;
                contents.asset_bytes += entry.size();
                contents.stored_assets +=
                    usize::from(entry.compression() == CompressionMethod::Stored);
            }
        } else if inner == "AndroidManifest.xml" || inner == "manifest/AndroidManifest.xml" {
            contents.manifests.push(name.clone());
        } else if name.starts_with("META-INF/") {
            contents.signature_files.push(name.clone());
        }
    }
    Ok(contents)
}

/// The outputs of a packaging run.
#[derive(Debug, Clone)]
pub struct PackageReport {
    /// The final file (signed when a key was given).
    pub path: PathBuf,
    /// What went in.
    pub written: WrittenPackage,
    /// The libraries.
    pub libraries: Vec<BuiltLibrary>,
}

/// Builds a complete APK: libraries, resources, assets (asset packs included: a plain APK has
/// no packs), aligned, and signed when `key` is given.
pub fn package_apk(
    project: &AndroidProject,
    tools: &AndroidToolchain,
    libraries: Vec<BuiltLibrary>,
    profile: &str,
    key: Option<&SigningKey>,
    output: Option<&Path>,
) -> AndroidResult<PackageReport> {
    let out_dir = project.out_dir(profile);
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let debuggable = profile_dir(profile) == "debug";
    let linked = link_resources(project, tools, &out_dir.join("apk"), debuggable, false)?;
    let mut assets = collect_assets(&project.dir, &project.config.assets)?;
    for pack in &project.config.asset_packs {
        assets.extend(collect_assets(&project.dir, &pack.assets)?);
    }
    check_unique(&assets)?;
    let index = asset_index(&assets);
    let unaligned = out_dir.join(format!("{}-unaligned.apk", project.name));
    let written = write_package(
        &unaligned,
        Some((&linked, |name| Some(name.to_string()))),
        &libraries,
        &assets,
        Some(&index),
        &project.stored_extensions(),
    )?;
    let aligned = out_dir.join(format!("{}-unsigned.apk", project.name));
    zipalign(tools, &unaligned, &aligned)?;
    let _ = fs::remove_file(&unaligned);
    let path = match key {
        Some(key) => {
            let signed = output.map(Path::to_path_buf).unwrap_or_else(|| {
                out_dir.join(format!(
                    "{}-{}.apk",
                    project.name,
                    if key.is_debug() { "debug" } else { "release" }
                ))
            });
            sign_apk(tools, key, &aligned, &signed, project.config.min_sdk)?;
            signed
        }
        None => match output {
            Some(path) => {
                fs::copy(&aligned, path).map_err(|e| e.to_string())?;
                path.to_path_buf()
            }
            None => aligned,
        },
    };
    Ok(PackageReport {
        path,
        written,
        libraries,
    })
}

fn check_unique(assets: &[AssetFile]) -> AndroidResult<()> {
    let mut seen = BTreeMap::new();
    for asset in assets {
        if let Some(other) = seen.insert(asset.path.as_str(), &asset.source) {
            return Err(format!(
                "assets/{} comes from both {} and {}",
                asset.path,
                other.display(),
                asset.source.display()
            ));
        }
    }
    Ok(())
}

/// The bundle configuration: native libraries and already-compressed media left uncompressed
/// in the APKs Google Play makes, splits by ABI only (one texture set, every language).
pub fn bundle_config_json(stored: &[String]) -> String {
    let globs: Vec<String> = stored.iter().map(|e| format!("assets/**.{e}")).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "optimizations": {
            "uncompressNativeLibraries": { "enabled": true },
            "splitsConfig": {
                "splitDimension": [
                    { "value": "ABI", "negate": false },
                    { "value": "SCREEN_DENSITY", "negate": true },
                    { "value": "LANGUAGE", "negate": true },
                    { "value": "TEXTURE_COMPRESSION_FORMAT", "negate": true }
                ]
            }
        },
        "compression": { "uncompressedGlob": globs }
    }))
    .unwrap_or_default()
}

/// Builds an app bundle (`.aab`) with `bundletool`: the base module (libraries, resources,
/// assets) and one module per install-time asset pack; signed with jarsigner when `key` is
/// given.
pub fn package_aab(
    project: &AndroidProject,
    tools: &AndroidToolchain,
    libraries: Vec<BuiltLibrary>,
    profile: &str,
    key: Option<&SigningKey>,
    output: Option<&Path>,
) -> AndroidResult<PackageReport> {
    let bundletool = tools.bundletool.clone().ok_or_else(|| {
        let dir = crate::android_toolchain::user_tools_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| "a tools folder".into());
        format!(
            "bundletool not found: download bundletool-all-<version>.jar from \
             https://github.com/google/bundletool/releases into {dir}, or set BUNDLETOOL_JAR"
        )
    })?;
    let java = java(tools)?.to_path_buf();
    let out_dir = project.out_dir(profile);
    let work = out_dir.join("aab");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let debuggable = profile_dir(profile) == "debug";
    let linked = link_resources(project, tools, &work.join("base-res"), debuggable, true)?;
    let stored = project.stored_extensions();

    let base_assets = collect_assets(&project.dir, &project.config.assets)?;
    let mut packs = Vec::new();
    for pack in &project.config.asset_packs {
        packs.push((pack, collect_assets(&project.dir, &pack.assets)?));
    }
    let all: Vec<AssetFile> = base_assets
        .iter()
        .chain(packs.iter().flat_map(|(_, a)| a.iter()))
        .cloned()
        .collect();
    check_unique(&all)?;
    let index = asset_index(&all);

    // aapt2's proto APK holds AndroidManifest.xml, resources.pb and res/: a module keeps the
    // manifest under manifest/.
    fn module_name(name: &str) -> Option<String> {
        Some(match name {
            "AndroidManifest.xml" => "manifest/AndroidManifest.xml".into(),
            other => other.into(),
        })
    }
    let base_zip = work.join("base.zip");
    let mut written = write_package(
        &base_zip,
        Some((&linked, module_name)),
        &libraries,
        &base_assets,
        Some(&index),
        &stored,
    )?;
    let mut modules = vec![base_zip];
    for (pack, assets) in &packs {
        let pack_dir = work.join(format!("pack-{}", pack.name));
        fs::create_dir_all(&pack_dir).map_err(|e| e.to_string())?;
        let manifest = pack_dir.join("AndroidManifest.xml");
        fs::write(
            &manifest,
            android_manifest::asset_pack_manifest_xml(&project.config.package, &pack.name),
        )
        .map_err(|e| e.to_string())?;
        let linked_pack = pack_dir.join("linked-proto.apk");
        run_output(
            Command::new(tools.aapt2())
                .arg("link")
                .arg("--proto-format")
                .arg("-o")
                .arg(&linked_pack)
                .arg("--manifest")
                .arg(&manifest)
                .arg("-I")
                .arg(&tools.android_jar),
            "aapt2 link (asset pack)",
        )?;
        fn pack_entry(name: &str) -> Option<String> {
            (name == "AndroidManifest.xml").then(|| "manifest/AndroidManifest.xml".into())
        }
        let zip = work.join(format!("{}.zip", pack.name));
        let pack_written = write_package(
            &zip,
            Some((&linked_pack, pack_entry)),
            &[],
            assets,
            None,
            &stored,
        )?;
        written.assets += pack_written.assets;
        written.asset_bytes += pack_written.asset_bytes;
        written.stored_assets += pack_written.stored_assets;
        written.entries += pack_written.entries;
        modules.push(zip);
    }
    let config = work.join("BundleConfig.json");
    fs::write(&config, bundle_config_json(&stored)).map_err(|e| e.to_string())?;
    let path = output.map(Path::to_path_buf).unwrap_or_else(|| {
        out_dir.join(format!(
            "{}{}.aab",
            project.name,
            match key {
                Some(k) if k.is_debug() => "-debug",
                Some(_) => "-release",
                None => "-unsigned",
            }
        ))
    });
    let module_list = modules
        .iter()
        .map(|m| m.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(",");
    run_output(
        Command::new(&java)
            .arg("-jar")
            .arg(&bundletool)
            .arg("build-bundle")
            .arg(format!("--modules={module_list}"))
            .arg(format!("--output={}", path.display()))
            .arg(format!("--config={}", config.display()))
            .arg("--overwrite"),
        "bundletool build-bundle",
    )?;
    if let Some(key) = key {
        sign_aab(tools, key, &path)?;
    }
    Ok(PackageReport {
        path,
        written,
        libraries,
    })
}

/// Signs an app bundle with jarsigner (Google Play wants the upload key's signature on it).
pub fn sign_aab(tools: &AndroidToolchain, key: &SigningKey, aab: &Path) -> AndroidResult<()> {
    let jarsigner = tools
        .jarsigner()
        .ok_or("jarsigner not found: set JAVA_HOME to a JDK (Android Studio's jbr works)")?;
    let mut cmd = Command::new(jarsigner);
    let alias = key.jarsigner_args(&mut cmd);
    cmd.arg(aab).arg(alias);
    run(&mut cmd, "jarsigner")
}

/// `jarsigner -verify` and `bundletool validate` of an app bundle.
pub fn verify_aab(tools: &AndroidToolchain, aab: &Path) -> AndroidResult<String> {
    let jarsigner = tools.jarsigner().ok_or("jarsigner not found")?;
    let mut out = run_output(
        Command::new(jarsigner).args(["-verify", "-certs"]).arg(aab),
        "jarsigner -verify",
    )?;
    if let (Some(java), Some(bundletool)) = (&tools.java, &tools.bundletool) {
        out.push_str(&run_output(
            Command::new(java)
                .arg("-jar")
                .arg(bundletool)
                .arg("validate")
                .arg(format!("--bundle={}", aab.display())),
            "bundletool validate",
        )?);
    }
    Ok(out)
}

/// Turns an app bundle into the universal APK Google Play would make from it (signed with
/// `key`), to check the bundle without a store or a device. Returns the extracted APK.
pub fn universal_apk_from_aab(
    tools: &AndroidToolchain,
    aab: &Path,
    key: &SigningKey,
) -> AndroidResult<PathBuf> {
    let bundletool = tools.bundletool.as_ref().ok_or("bundletool not found")?;
    let apks = aab.with_extension("universal.apks");
    let mut cmd = Command::new(java(tools)?);
    cmd.arg("-jar")
        .arg(bundletool)
        .arg("build-apks")
        .arg(format!("--bundle={}", aab.display()))
        .arg(format!("--output={}", apks.display()))
        .args(["--mode=universal", "--overwrite"]);
    match key {
        SigningKey::Debug { keystore } => {
            cmd.arg(format!("--ks={}", keystore.display())).args([
                "--ks-key-alias=androiddebugkey",
                "--ks-pass=pass:android",
                "--key-pass=pass:android",
            ]);
        }
        SigningKey::Release {
            keystore,
            alias,
            store_password,
            key_password,
        } => {
            cmd.arg(format!("--ks={}", keystore.display()))
                .arg(format!("--ks-key-alias={alias}"));
            if let Some(pass) = store_password {
                cmd.env(STORE_PASS_ENV, pass)
                    .arg(format!("--ks-pass=env:{STORE_PASS_ENV}"));
            }
            if let Some(pass) = key_password {
                cmd.env(KEY_PASS_ENV, pass)
                    .arg(format!("--key-pass=env:{KEY_PASS_ENV}"));
            }
        }
    }
    run(&mut cmd, "bundletool build-apks")?;
    let mut archive = zip::ZipArchive::new(File::open(&apks).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut entry = archive
        .by_name("universal.apk")
        .map_err(|e| format!("{}: {e}", apks.display()))?;
    let apk = aab.with_extension("universal.apk");
    io::copy(
        &mut entry,
        &mut File::create(&apk).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    drop(entry);
    let _ = fs::remove_file(&apks);
    Ok(apk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unversioned_cxx_runtime_imports_are_reported() {
        // What llvm-nm printed for the third-person demo linked against `stdc++`.
        let nm = "                 U _ZdlPv@LIBC_O\n\
                  \x20                U _Znwm@LIBC_O\n\
                  \x20                U __cxa_atexit@LIBC\n\
                  \x20                U __cxa_begin_catch\n\
                  \x20                U __cxa_finalize@LIBC\n\
                  \x20                U __gxx_personality_v0\n\
                  \x20                U _ZNSt6__ndk112basic_stringIcEE\n\
                  \x20                w __cxa_thread_atexit_impl\n\
                  \x20                U memcpy@LIBC\n";
        assert_eq!(
            missing_cxx_runtime(nm),
            vec![
                "__cxa_begin_catch",
                "__gxx_personality_v0",
                "_ZNSt6__ndk112basic_stringIcEE"
            ]
        );
        assert!(missing_cxx_runtime("                 U memcpy@LIBC\n").is_empty());
    }

    #[test]
    fn assets_are_collected_with_their_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("scenes/sub")).unwrap();
        fs::create_dir_all(root.join("data/.git")).unwrap();
        fs::write(root.join("scenes/arena.json"), b"{}").unwrap();
        fs::write(root.join("scenes/sub/b.ogg"), b"OggS").unwrap();
        fs::write(root.join("data/.git/config"), b"x").unwrap();
        fs::write(root.join("data/big.psd"), b"x").unwrap();
        fs::write(root.join("data/ok.txt"), b"hello").unwrap();
        let specs = vec![
            AssetSpec::Dir("scenes".into()),
            AssetSpec::Mapped {
                from: "data".into(),
                to: Some("".into()),
                exclude: vec!["**/*.psd".into()],
            },
        ];
        let files = collect_assets(root, &specs).unwrap();
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["ok.txt", "scenes/arena.json", "scenes/sub/b.ogg"]
        );
        let index = asset_index(&files);
        assert!(index.is_dir("scenes/sub"));
        assert_eq!(index.file_size("ok.txt"), Some(5));
        // The same path from two folders is an error.
        let clash = vec![
            AssetSpec::Dir("scenes".into()),
            AssetSpec::Mapped {
                from: "scenes".into(),
                to: Some("scenes".into()),
                exclude: vec![],
            },
        ];
        assert!(collect_assets(root, &clash).is_err());
        assert!(collect_assets(root, &[AssetSpec::Dir("missing".into())]).is_err());
    }

    #[test]
    fn packages_store_media_and_libraries() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("song.ogg"), vec![7u8; 5000]).unwrap();
        fs::write(root.join("level.json"), vec![b'a'; 5000]).unwrap();
        fs::write(root.join("libgame.so"), b"\x7fELF fake").unwrap();
        let assets = collect_assets(
            root,
            &[
                AssetSpec::Mapped {
                    from: "song.ogg".into(),
                    to: Some("music/song.ogg".into()),
                    exclude: vec![],
                },
                AssetSpec::Mapped {
                    from: "level.json".into(),
                    to: Some("level.json".into()),
                    exclude: vec![],
                },
            ],
        )
        .unwrap();
        let lib = BuiltLibrary {
            abi: AndroidAbi::Arm64,
            packaged: root.join("libgame.so"),
            unstripped: root.join("libgame.so"),
            info: ElfInfo {
                is_64: true,
                machine: 183,
                min_load_align: 16384,
                needed: vec![],
            },
            extra: vec![],
        };
        let out = root.join("test.apk");
        let stored: Vec<String> = DEFAULT_STORED_EXTENSIONS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let written = write_package(
            &out,
            None,
            std::slice::from_ref(&lib),
            &assets,
            Some(&asset_index(&assets)),
            &stored,
        )
        .unwrap();
        assert_eq!(written.assets, 2);
        assert_eq!(written.stored_assets, 1);
        let contents = inspect_package(&out).unwrap();
        assert_eq!(contents.assets, 2);
        assert_eq!(contents.stored_assets, 1);
        assert!(contents.has_index);
        assert_eq!(contents.libraries.len(), 1);
        assert!(contents.libraries[0].2, "libraries are stored");
        assert_eq!(contents.libraries[0].0, "lib/arm64-v8a/libgame.so");
        let config = bundle_config_json(&stored);
        assert!(config.contains("assets/**.ogg"));
        assert!(config.contains("uncompressNativeLibraries"));
    }

    #[test]
    fn release_keys_come_from_a_local_file_and_hide_passwords() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("upload.keystore"), b"not a real keystore").unwrap();
        // The "password" is a marker string the Debug output must never show.
        fs::write(
            dir.path().join(SIGNING_FILE),
            "# local only\nstoreFile=upload.keystore\nkeyAlias=upload\nstorePassword=MARKER-NOT-A-PASSWORD\n",
        )
        .unwrap();
        // Environment values win over the file, so only test when they are unset.
        if [KEYSTORE_ENV, KEY_ALIAS_ENV, STORE_PASS_ENV, KEY_PASS_ENV]
            .iter()
            .all(|v| std::env::var_os(v).is_none())
        {
            let key = SigningKey::release(None, None, None, &[dir.path().to_path_buf()]).unwrap();
            let shown = format!("{key:?}");
            assert!(!shown.contains("MARKER-NOT-A-PASSWORD"), "{shown}");
            assert!(shown.contains("<hidden>"));
            match key {
                SigningKey::Release {
                    keystore,
                    alias,
                    key_password,
                    ..
                } => {
                    assert_eq!(keystore, dir.path().join("upload.keystore"));
                    assert_eq!(alias, "upload");
                    assert!(key_password.is_none());
                }
                SigningKey::Debug { .. } => panic!("not a release key"),
            }
        }
        assert!(SigningKey::release(
            Some(dir.path().join("missing.keystore")),
            Some("a".into()),
            None,
            &[]
        )
        .is_err());
    }
}
