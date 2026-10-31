//! `velvet android`: build, package, sign, verify, install and debug Android apps of Velvet
//! games (NativeActivity, no Gradle). See `docs/platforms/ANDROID.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use velvet_build::android_config::AndroidAbi;
use velvet_build::android_package::{
    self as pkg, AndroidProject, BuildOptions, SigningKey, SIGNING_FILE,
};
use velvet_build::android_toolchain::{adb_in, AndroidToolchain};

#[derive(Subcommand, Debug)]
pub enum AndroidCommand {
    /// Check the toolchain (SDK, NDK, build-tools, JDK, bundletool, Rust targets) and list
    /// connected devices. Installs nothing.
    Doctor,
    /// Print a [package.metadata.velvet.android] template to paste into a game's Cargo.toml.
    Init {
        #[command(flatten)]
        target: PackageArgs,
    },
    /// Build the game's native library for each ABI (cargo with the NDK).
    Build {
        #[command(flatten)]
        target: PackageArgs,
        #[command(flatten)]
        build: BuildArgs,
    },
    /// Build, package, align, sign and verify an APK.
    Apk {
        #[command(flatten)]
        target: PackageArgs,
        #[command(flatten)]
        build: BuildArgs,
        #[command(flatten)]
        sign: SignArgs,
        /// Where to write the APK (default: target/velvet-android/<package>/<profile>/).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build an Android App Bundle (.aab) for Google Play (needs bundletool).
    Aab {
        #[command(flatten)]
        target: PackageArgs,
        #[command(flatten)]
        build: BuildArgs,
        #[command(flatten)]
        sign: SignArgs,
        /// Where to write the bundle.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also make the universal APK Google Play would serve from it and verify that APK.
        #[arg(long)]
        check: bool,
    },
    /// Sign an existing APK or app bundle again.
    Sign {
        /// The .apk or .aab.
        file: PathBuf,
        #[command(flatten)]
        sign: SignArgs,
        /// Lowest API level the APK's signature must work on (default 26).
        #[arg(long, default_value_t = 26)]
        min_sdk: u32,
        /// Output file (APK only; default: next to the input, `-signed`).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Verify an APK or app bundle: signature, 16 KB alignment, ABIs, assets, manifest.
    Verify {
        /// The .apk or .aab.
        file: PathBuf,
    },
    /// Install an APK on a connected device with adb (no other command ever installs).
    Install {
        /// The APK.
        apk: PathBuf,
        /// Device serial (`adb devices`), when several are connected.
        #[arg(long)]
        device: Option<String>,
    },
    /// Show the game's log from a connected device (stdout/stderr, panics, crashes).
    Logcat {
        #[command(flatten)]
        target: PackageArgs,
        /// Device serial, when several are connected.
        #[arg(long)]
        device: Option<String>,
        /// The whole device log, not only the game's.
        #[arg(long)]
        all: bool,
        /// Print what is in the log and stop (instead of following it).
        #[arg(long)]
        dump: bool,
    },
    /// Show how to make a release (upload) key. Nothing is created.
    KeygenHelp,
}

#[derive(Args, Debug, Clone)]
pub struct PackageArgs {
    /// The game's package (Cargo name).
    #[arg(short = 'p', long)]
    package: Option<String>,
    /// Its Cargo.toml (default: the current folder's).
    #[arg(long)]
    manifest_path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct BuildArgs {
    /// Cargo profile.
    #[arg(long, default_value = "release")]
    profile: String,
    /// Shorthand for `--profile dev` (a debuggable app).
    #[arg(long)]
    debug: bool,
    /// Parallel cargo jobs.
    #[arg(short = 'j', long)]
    jobs: Option<u32>,
    /// ABIs to build instead of the configuration's (repeatable): arm64-v8a, x86_64, ...
    #[arg(long = "abi")]
    abis: Vec<String>,
}

impl BuildArgs {
    fn options(&self) -> Result<BuildOptions> {
        let abis = if self.abis.is_empty() {
            None
        } else {
            Some(
                self.abis
                    .iter()
                    .map(|a| AndroidAbi::parse(a).ok_or_else(|| anyhow!("unknown ABI `{a}`")))
                    .collect::<Result<Vec<_>>>()?,
            )
        };
        Ok(BuildOptions {
            profile: if self.debug {
                "dev".into()
            } else {
                self.profile.clone()
            },
            jobs: self.jobs,
            abis,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum SignMode {
    /// Unsigned (cannot be installed).
    None,
    /// The Android debug key: installable for testing, refused by stores.
    Debug,
    /// The release (upload) key from --keystore/--alias, the environment or the local signing file.
    Release,
}

#[derive(Args, Debug, Clone)]
pub struct SignArgs {
    /// Which key signs the package.
    #[arg(long, value_enum, default_value_t = SignMode::Debug)]
    sign: SignMode,
    /// Release keystore (else VELVET_ANDROID_KEYSTORE, else storeFile in the signing file).
    #[arg(long)]
    keystore: Option<PathBuf>,
    /// Release key alias (else VELVET_ANDROID_KEY_ALIAS, else keyAlias in the signing file).
    #[arg(long)]
    alias: Option<String>,
    /// A signing file (default: android-signing.local.properties beside the game or the workspace).
    #[arg(long)]
    signing_file: Option<PathBuf>,
}

impl SignArgs {
    fn key(&self, tools: &AndroidToolchain, search: &[PathBuf]) -> Result<Option<SigningKey>> {
        Ok(match self.sign {
            SignMode::None => None,
            SignMode::Debug => Some(SigningKey::debug(tools).map_err(|e| anyhow!(e))?),
            SignMode::Release => Some(
                SigningKey::release(
                    self.keystore.clone(),
                    self.alias.clone(),
                    self.signing_file.clone(),
                    search,
                )
                .map_err(|e| anyhow!(e))?,
            ),
        })
    }
}

pub fn cmd_android(command: AndroidCommand) -> Result<()> {
    match command {
        AndroidCommand::Doctor => doctor(),
        AndroidCommand::Init { target } => init(&target),
        AndroidCommand::Build { target, build } => {
            let project = load(&target)?;
            let tools = toolchain(&project)?;
            let libs = pkg::build_libraries(&project, &tools, &build.options()?)
                .map_err(|e| anyhow!(e))?;
            for lib in libs {
                println!("{} {}", lib.abi.name(), lib.packaged.display());
            }
            Ok(())
        }
        AndroidCommand::Apk {
            target,
            build,
            sign,
            out,
        } => {
            let project = load(&target)?;
            let tools = toolchain(&project)?;
            let key = sign.key(&tools, &search_dirs(&project))?;
            let options = build.options()?;
            let libs = pkg::build_libraries(&project, &tools, &options).map_err(|e| anyhow!(e))?;
            let report = pkg::package_apk(
                &project,
                &tools,
                libs,
                &options.profile,
                key.as_ref(),
                out.as_deref(),
            )
            .map_err(|e| anyhow!(e))?;
            println!(
                "packed {} assets ({} bytes, {} stored uncompressed)",
                report.written.assets, report.written.asset_bytes, report.written.stored_assets
            );
            report_apk(&tools, &report.path, key.is_some())?;
            if key.as_ref().is_some_and(SigningKey::is_debug) {
                println!("signed with the debug key: fine to install for testing, not for stores");
            }
            Ok(())
        }
        AndroidCommand::Aab {
            target,
            build,
            sign,
            out,
            check,
        } => {
            let project = load(&target)?;
            let tools = toolchain(&project)?;
            let key = sign.key(&tools, &search_dirs(&project))?;
            let options = build.options()?;
            let libs = pkg::build_libraries(&project, &tools, &options).map_err(|e| anyhow!(e))?;
            let report = pkg::package_aab(
                &project,
                &tools,
                libs,
                &options.profile,
                key.as_ref(),
                out.as_deref(),
            )
            .map_err(|e| anyhow!(e))?;
            report_aab(&tools, &report.path, key.is_some())?;
            if check {
                let debug = SigningKey::debug(&tools).map_err(|e| anyhow!(e))?;
                let apk = pkg::universal_apk_from_aab(&tools, &report.path, &debug)
                    .map_err(|e| anyhow!(e))?;
                println!("\nuniversal APK made from the bundle (debug-signed, for checking):");
                report_apk(&tools, &apk, true)?;
            }
            Ok(())
        }
        AndroidCommand::Sign {
            file,
            sign,
            min_sdk,
            out,
        } => {
            let tools = AndroidToolchain::discover(None, 21).map_err(|e| anyhow!(e))?;
            let search = std::env::current_dir().into_iter().collect::<Vec<_>>();
            let key = sign
                .key(&tools, &search)?
                .ok_or_else(|| anyhow!("--sign none: nothing to do"))?;
            if is_bundle(&file) {
                pkg::sign_aab(&tools, &key, &file).map_err(|e| anyhow!(e))?;
                report_aab(&tools, &file, true)
            } else {
                let out = out.unwrap_or_else(|| sibling(&file, "-signed.apk"));
                pkg::sign_apk(&tools, &key, &file, &out, min_sdk).map_err(|e| anyhow!(e))?;
                report_apk(&tools, &out, true)
            }
        }
        AndroidCommand::Verify { file } => {
            let tools = AndroidToolchain::discover(None, 21).map_err(|e| anyhow!(e))?;
            if is_bundle(&file) {
                report_aab(&tools, &file, true)
            } else {
                report_apk(&tools, &file, true)
            }
        }
        AndroidCommand::Install { apk, device } => install(&apk, device.as_deref()),
        AndroidCommand::Logcat {
            target,
            device,
            all,
            dump,
        } => logcat(&target, device.as_deref(), all, dump),
        AndroidCommand::KeygenHelp => {
            keygen_help();
            Ok(())
        }
    }
}

fn load(target: &PackageArgs) -> Result<AndroidProject> {
    AndroidProject::load(target.manifest_path.as_deref(), target.package.as_deref())
        .map_err(|e| anyhow!(e))
}

fn toolchain(project: &AndroidProject) -> Result<AndroidToolchain> {
    AndroidToolchain::discover(project.config.compile_sdk, project.config.target_sdk)
        .map_err(|e| anyhow!(e))
}

fn search_dirs(project: &AndroidProject) -> Vec<PathBuf> {
    vec![project.dir.clone(), project.workspace_root.clone()]
}

fn is_bundle(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("aab"))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "app".into());
    path.with_file_name(format!("{stem}{suffix}"))
}

fn size_text(bytes: u64) -> String {
    if bytes >= 1 << 30 {
        format!("{:.2} GiB", bytes as f64 / (1u64 << 30) as f64)
    } else {
        format!("{:.2} MiB", bytes as f64 / (1u64 << 20) as f64)
    }
}

/// Signature, alignment and contents of an APK.
fn report_apk(tools: &AndroidToolchain, apk: &Path, signed: bool) -> Result<()> {
    let contents = pkg::inspect_package(apk).map_err(|e| anyhow!(e))?;
    println!(
        "\nAPK {} ({}, {} bytes, {} entries)",
        apk.display(),
        size_text(contents.file_size),
        contents.file_size,
        contents.entries
    );
    for (path, size, stored, offset) in &contents.libraries {
        println!(
            "  {path}: {} bytes, {}, offset {offset} ({})",
            size,
            if *stored { "stored" } else { "COMPRESSED" },
            if offset % 16384 == 0 {
                "16 KB aligned"
            } else {
                "NOT 16 KB aligned"
            }
        );
    }
    println!(
        "  assets: {} files, {} ({} stored uncompressed), index {}",
        contents.assets,
        size_text(contents.asset_bytes),
        contents.stored_assets,
        if contents.has_index {
            "present"
        } else {
            "missing"
        }
    );
    println!("  manifest: {}", contents.manifests.join(", "));
    let misaligned = contents.misaligned_libraries();
    if !misaligned.is_empty() {
        bail!("native libraries not stored on 16 KB pages: {misaligned:?}");
    }
    println!(
        "  zipalign -c -P 16 4: {}",
        pkg::zipalign_check(tools, apk).map_err(|e| anyhow!(e))?
    );
    match pkg::badging(tools, apk) {
        Ok(badging) => {
            for line in badging.lines().filter(|l| {
                [
                    "package:",
                    "minSdkVersion:",
                    "targetSdkVersion:",
                    "application-label:",
                    "application-debuggable",
                    "launchable-activity:",
                    "native-code:",
                    "uses-gl-es:",
                    "uses-feature",
                ]
                .iter()
                .any(|p| l.trim_start().starts_with(p))
            }) {
                println!("  {}", line.trim_start());
            }
        }
        Err(e) => println!("  aapt2 dump badging failed: {e}"),
    }
    if signed {
        let verify = pkg::verify_apk(tools, apk).map_err(|e| anyhow!(e))?;
        println!("apksigner verify --verbose --print-certs:");
        for line in verify.lines().filter(|l| !l.trim().is_empty()) {
            println!("  {line}");
        }
    }
    Ok(())
}

/// Signature and contents of an app bundle.
fn report_aab(tools: &AndroidToolchain, aab: &Path, signed: bool) -> Result<()> {
    let contents = pkg::inspect_package(aab).map_err(|e| anyhow!(e))?;
    println!(
        "\nAAB {} ({}, {} entries)",
        aab.display(),
        size_text(contents.file_size),
        contents.entries
    );
    for (path, size, ..) in &contents.libraries {
        println!("  {path}: {size} bytes");
    }
    println!(
        "  assets: {} files, {}, index {}",
        contents.assets,
        size_text(contents.asset_bytes),
        if contents.has_index {
            "present"
        } else {
            "missing"
        }
    );
    println!("  modules: {}", contents.manifests.join(", "));
    if signed {
        let verify = pkg::verify_aab(tools, aab).map_err(|e| anyhow!(e))?;
        println!("jarsigner -verify / bundletool validate:");
        for line in verify.lines().filter(|l| !l.trim().is_empty()).take(40) {
            println!("  {line}");
        }
    }
    Ok(())
}

fn doctor() -> Result<()> {
    println!("Android toolchain for `velvet android`:");
    match AndroidToolchain::discover(None, 36) {
        Ok(tools) => {
            println!("  SDK          {}", tools.sdk.display());
            println!(
                "  NDK          {} ({})",
                tools.ndk.display(),
                tools.ndk_version
            );
            if tools.ndk_major().is_some_and(|m| m < 28) {
                println!(
                    "               NDK r28 or newer links 16 KB pages by default (Google Play)"
                );
            }
            println!("  build-tools  {}", tools.build_tools.display());
            println!(
                "  platform     android-{} ({})",
                tools.compile_sdk,
                tools.android_jar.display()
            );
            match &tools.java {
                Some(java) => println!("  java         {}", java.display()),
                None => {
                    println!("  java         MISSING (set JAVA_HOME; Android Studio's jbr works)")
                }
            }
            match tools.jarsigner() {
                Some(j) => println!("  jarsigner    {}", j.display()),
                None => println!("  jarsigner    missing (needed for app bundles)"),
            }
            match &tools.bundletool {
                Some(b) => println!("  bundletool   {}", b.display()),
                None => println!(
                    "  bundletool   not found (only for .aab: BUNDLETOOL_JAR, or a bundletool-all-*.jar in {})",
                    velvet_build::android_toolchain::user_tools_dir()
                        .map(|d| d.display().to_string())
                        .unwrap_or_default()
                ),
            }
            let installed = pkg::installed_rust_targets(Path::new("."));
            for abi in [AndroidAbi::Arm64, AndroidAbi::X86_64, AndroidAbi::Armv7] {
                let target = abi.rust_target();
                let have = installed.iter().any(|t| t == target);
                println!(
                    "  rust         {target}: {}",
                    if have {
                        "installed".to_string()
                    } else {
                        format!("missing (rustup target add {target})")
                    }
                );
            }
            let adb = tools.adb();
            if adb.is_file() {
                match devices(&adb) {
                    Ok(list) if list.is_empty() => println!("  devices      none connected"),
                    Ok(list) => {
                        for (serial, state) in list {
                            println!("  device       {serial} ({state})");
                        }
                    }
                    Err(e) => println!("  devices      adb failed: {e}"),
                }
            } else {
                println!("  adb          missing (platform-tools)");
            }
        }
        Err(e) => println!("  {e}"),
    }
    Ok(())
}

fn init(target: &PackageArgs) -> Result<()> {
    let name = target.package.clone().unwrap_or_else(|| "my-game".into());
    let id: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    println!(
        r#"# Paste into the game's Cargo.toml, then: velvet android apk -p {name}
# The game's program must live in src/lib.rs (with velvet_platform::android_main!), see
# docs/platforms/ANDROID.md.
[package.metadata.velvet.android]
package = "com.example.{id}"        # your application id: it can never change once published
label = "{name}"
version_code = 1                    # raise it for every upload
min_sdk = 26                        # AAudio (rodio/cpal) needs 26
target_sdk = 36                     # Google Play requirement since 2026-08-31
abis = ["arm64-v8a", "x86_64"]
orientation = "sensorLandscape"
assets = ["assets"]                 # folders packed into the APK (read with velvet_platform::files::assets)
# icon = "android/icon.png"         # square PNG, 512 px
# asset_packs = [{{ name = "media", assets = [{{ from = "media", to = "media" }}] }}]  # AAB only
"#
    );
    Ok(())
}

fn keygen_help() {
    println!(
        r#"Make your release (upload) key once, keep it and its passwords safe, and never commit it:

  keytool -genkeypair -v -keystore my-upload-key.keystore -alias upload -keyalg RSA -keysize 4096 -validity 10000

keytool (from a JDK, e.g. Android Studio's jbr/bin) asks for the passwords and your name. Then sign
with it in one of these ways (passwords are given to the signers by variable name, never on a command
line; when they are not set, apksigner and jarsigner ask for them):

  1. Environment:  VELVET_ANDROID_KEYSTORE=<path>  VELVET_ANDROID_KEY_ALIAS=upload
                   VELVET_KEYSTORE_PASS=<password>  [VELVET_KEY_PASS=<password>]
  2. A local file {SIGNING_FILE} beside the game's Cargo.toml or the workspace root
     (git-ignored), with Gradle's keys:
                   storeFile=<path, relative to the file>
                   keyAlias=upload
                   storePassword=<optional>
                   keyPassword=<optional>

  velvet android aab -p <game> --sign release

Google Play: enrol in Play App Signing; the key above is then your upload key, and Google keeps the
app signing key. Lose the upload key and Google can reset it; lose an app signing key you manage
yourself and the app can never be updated."#
    );
}

/// `adb devices`: serial and state of each device.
fn devices(adb: &Path) -> Result<Vec<(String, String)>> {
    let out = Command::new(adb)
        .arg("devices")
        .output()
        .context("adb devices")?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect())
}

fn adb_for_device(device: Option<&str>) -> Result<(PathBuf, String)> {
    let sdk = velvet_build::detect_android_sdk().ok_or_else(|| anyhow!("Android SDK not found"))?;
    let adb = adb_in(&sdk);
    if !adb.is_file() {
        bail!("adb not found in {}", adb.display());
    }
    let ready: Vec<String> = devices(&adb)?
        .into_iter()
        .filter(|(_, state)| state == "device")
        .map(|(serial, _)| serial)
        .collect();
    let serial = match device {
        Some(d) if ready.iter().any(|s| s == d) => d.to_string(),
        Some(d) => bail!("device {d} is not connected and ready (adb devices)"),
        None => match ready.as_slice() {
            [] => bail!("no device connected and ready (adb devices)"),
            [one] => one.clone(),
            many => bail!(
                "several devices connected ({}): pass --device",
                many.join(", ")
            ),
        },
    };
    Ok((adb, serial))
}

fn install(apk: &Path, device: Option<&str>) -> Result<()> {
    if !apk.is_file() {
        bail!("{} does not exist", apk.display());
    }
    let (adb, serial) = adb_for_device(device).context("nothing was installed")?;
    println!("installing {} on {serial}", apk.display());
    let status = Command::new(adb)
        .args(["-s", &serial, "install", "-r"])
        .arg(apk)
        .status()
        .context("adb install")?;
    if !status.success() {
        bail!("adb install failed ({status})");
    }
    Ok(())
}

fn logcat(target: &PackageArgs, device: Option<&str>, all: bool, dump: bool) -> Result<()> {
    let (adb, serial) = adb_for_device(device)?;
    let mut cmd = Command::new(&adb);
    cmd.args(["-s", &serial, "logcat", "-v", "time"]);
    if dump {
        cmd.arg("-d");
    }
    if !all {
        let project = load(target)?;
        let id = project.config.package.clone();
        let pid = Command::new(&adb)
            .args(["-s", &serial, "shell", "pidof", "-s", &id])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
        match pid {
            Some(pid) => {
                println!("following {id} (pid {pid}); --all for the whole log");
                cmd.arg(format!("--pid={pid}"));
            }
            None => {
                // Not running yet: the game's own output, Rust panics, and native crashes.
                println!("{id} is not running: showing game output and crashes only");
                cmd.args([
                    "RustStdoutStderr:V",
                    "velvet:V",
                    "wgpu_hal:V",
                    "AndroidRuntime:E",
                    "DEBUG:V",
                    "libc:F",
                    "*:S",
                ]);
            }
        }
    }
    let status = cmd.status().context("adb logcat")?;
    if !status.success() {
        bail!("adb logcat failed ({status})");
    }
    Ok(())
}
