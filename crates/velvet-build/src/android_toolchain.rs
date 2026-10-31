//! Finding the Android tools: SDK, NDK, build-tools, platform, JDK, bundletool and adb; and the
//! environment that lets cargo build a crate with the NDK's compilers.
//!
//! Looked up, in order:
//!
//! - SDK: `ANDROID_HOME`, `ANDROID_SDK_ROOT`, then the usual install folders.
//! - NDK: `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, `ANDROID_NDK`, then the newest `<sdk>/ndk/*`.
//! - build-tools: the newest `<sdk>/build-tools/*` with `aapt2`, `zipalign` and `apksigner`
//!   (35 or newer for 16 KB page alignment).
//! - JDK: `JAVA_HOME`, then `java` on the `PATH`.
//! - bundletool (app bundles only): `BUNDLETOOL_JAR`, then `bundletool*.jar` in
//!   `$VELVET_TOOLS_DIR`, the user's Velvet tools folder (`%LOCALAPPDATA%\velvet\tools` or
//!   `~/.local/share/velvet/tools`) and `<sdk>/bundletool`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::android_config::AndroidAbi;

/// The Android tools found on this machine.
#[derive(Debug, Clone)]
pub struct AndroidToolchain {
    /// The SDK.
    pub sdk: PathBuf,
    /// The NDK.
    pub ndk: PathBuf,
    /// The NDK's version (`Pkg.Revision`, e.g. `28.2.13676358`).
    pub ndk_version: String,
    /// The build-tools folder used (aapt2, zipalign, apksigner).
    pub build_tools: PathBuf,
    /// The `android.jar` resources are linked against.
    pub android_jar: PathBuf,
    /// Its API level.
    pub compile_sdk: u32,
    /// `java`, when a JDK was found (apksigner, bundletool).
    pub java: Option<PathBuf>,
    /// `bundletool` (a jar), when found.
    pub bundletool: Option<PathBuf>,
}

/// Where a tool was not found.
pub type ToolError = String;

impl AndroidToolchain {
    /// Finds the tools. `compile_sdk` asks for that platform; `None` takes the lowest installed
    /// platform at or above `at_least` (the target SDK: compiling against it is the norm).
    pub fn discover(compile_sdk: Option<u32>, at_least: u32) -> Result<Self, ToolError> {
        let sdk = crate::android::detect_android_sdk().ok_or(
            "Android SDK not found: set ANDROID_HOME (Android Studio installs it under \
             %LOCALAPPDATA%\\Android\\Sdk on Windows, ~/Android/Sdk on Linux)",
        )?;
        let ndk = find_ndk(&sdk)?;
        let ndk_version = read_property(&ndk.join("source.properties"), "Pkg.Revision")
            .unwrap_or_else(|| "unknown".into());
        let build_tools = find_build_tools(&sdk)?;
        let (android_jar, compile_sdk) = find_platform(&sdk, compile_sdk, at_least)?;
        Ok(Self {
            java: find_java(),
            bundletool: find_bundletool(&sdk),
            sdk,
            ndk,
            ndk_version,
            build_tools,
            android_jar,
            compile_sdk,
        })
    }

    /// The NDK's major version (`28` for r28).
    pub fn ndk_major(&self) -> Option<u32> {
        self.ndk_version.split('.').next()?.parse().ok()
    }

    /// The NDK's LLVM tools folder for this host.
    pub fn llvm_bin(&self) -> PathBuf {
        self.ndk
            .join("toolchains/llvm/prebuilt")
            .join(host_tag())
            .join("bin")
    }

    /// The NDK's clang wrapper for an ABI and API level (`aarch64-linux-android26-clang`).
    pub fn clang(&self, abi: AndroidAbi, api: u32, cpp: bool) -> PathBuf {
        let api = self.clamp_api(abi, api);
        let name = format!(
            "{}{api}-clang{}{}",
            abi.clang_prefix(),
            if cpp { "++" } else { "" },
            if cfg!(windows) { ".cmd" } else { "" }
        );
        self.llvm_bin().join(name)
    }

    /// The API level the NDK has wrappers for that is closest to `api` (NDKs carry 21 up to
    /// their newest release, e.g. 35 in r28).
    fn clamp_api(&self, abi: AndroidAbi, api: u32) -> u32 {
        let exists = |level: u32| {
            let name = format!(
                "{}{level}-clang{}",
                abi.clang_prefix(),
                if cfg!(windows) { ".cmd" } else { "" }
            );
            self.llvm_bin().join(name).is_file()
        };
        if exists(api) {
            return api;
        }
        (21..api).rev().find(|&l| exists(l)).unwrap_or(api)
    }

    /// An LLVM tool of the NDK (`llvm-ar`, `llvm-strip`, `llvm-readelf`).
    pub fn llvm_tool(&self, name: &str) -> PathBuf {
        self.llvm_bin().join(exe(name))
    }

    /// `aapt2`.
    pub fn aapt2(&self) -> PathBuf {
        self.build_tools.join(exe("aapt2"))
    }

    /// `zipalign`.
    pub fn zipalign(&self) -> PathBuf {
        self.build_tools.join(exe("zipalign"))
    }

    /// `apksigner.jar` (run with `java -jar`, which avoids the `.bat` launcher on Windows).
    pub fn apksigner_jar(&self) -> PathBuf {
        self.build_tools.join("lib").join("apksigner.jar")
    }

    /// `adb`.
    pub fn adb(&self) -> PathBuf {
        adb_in(&self.sdk)
    }

    /// `jarsigner` beside `java` (app bundles are signed with it).
    pub fn jarsigner(&self) -> Option<PathBuf> {
        let java = self.java.as_ref()?;
        let path = java.with_file_name(exe("jarsigner"));
        path.is_file().then_some(path)
    }

    /// `keytool` beside `java`.
    pub fn keytool(&self) -> Option<PathBuf> {
        let java = self.java.as_ref()?;
        let path = java.with_file_name(exe("keytool"));
        path.is_file().then_some(path)
    }

    /// `libc++_shared.so` of the NDK for an ABI (packaged when a library needs it).
    pub fn libcxx_shared(&self, abi: AndroidAbi) -> PathBuf {
        let triple = match abi {
            AndroidAbi::Arm64 => "aarch64-linux-android",
            AndroidAbi::Armv7 => "arm-linux-androideabi",
            AndroidAbi::X86_64 => "x86_64-linux-android",
            AndroidAbi::X86 => "i686-linux-android",
        };
        self.llvm_bin()
            .parent()
            .unwrap_or(Path::new("."))
            .join("sysroot/usr/lib")
            .join(triple)
            .join("libc++_shared.so")
    }

    /// The variables that make cargo (and the `cc` crate, for C and C++ dependencies) build
    /// for an ABI with the NDK: linker, C/C++ compilers, archiver, the NDK path for build
    /// scripts that look for it, and a static C++ library (one shared library in the app).
    pub fn cargo_env(&self, abi: AndroidAbi, api: u32) -> Vec<(String, OsString)> {
        let target = abi.rust_target();
        let under = target.replace('-', "_");
        let upper = under.to_uppercase();
        let cc = self.clang(abi, api, false).into_os_string();
        let cxx = self.clang(abi, api, true).into_os_string();
        let ar = self.llvm_tool("llvm-ar").into_os_string();
        vec![
            (format!("CARGO_TARGET_{upper}_LINKER"), cc.clone()),
            (format!("CC_{under}"), cc),
            (format!("CXX_{under}"), cxx),
            (format!("AR_{under}"), ar.clone()),
            (
                format!("RANLIB_{under}"),
                self.llvm_tool("llvm-ranlib").into(),
            ),
            (format!("CXXSTDLIB_{under}"), "c++_static".into()),
            ("ANDROID_NDK_HOME".into(), self.ndk.clone().into_os_string()),
            ("ANDROID_NDK_ROOT".into(), self.ndk.clone().into_os_string()),
            ("ANDROID_NDK".into(), self.ndk.clone().into_os_string()),
            ("ANDROID_HOME".into(), self.sdk.clone().into_os_string()),
        ]
    }
}

/// `adb` of an SDK.
pub fn adb_in(sdk: &Path) -> PathBuf {
    sdk.join("platform-tools").join(exe("adb"))
}

/// The NDK's name for this host's prebuilt folder.
pub fn host_tag() -> &'static str {
    if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "linux-x86_64"
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// A `key=value` line of a `.properties` file.
pub fn read_property(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim().to_string())
    })
}

/// Sorts version-like folder names (`28.2.13676358`, `36.1.0`, `android-36.1`) numerically.
fn version_key(name: &str) -> Vec<u64> {
    name.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

fn newest_dir(parent: &Path, accept: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && accept(p))
        .collect();
    dirs.sort_by_key(|p| version_key(&p.file_name().unwrap_or_default().to_string_lossy()));
    dirs.pop()
}

fn find_ndk(sdk: &Path) -> Result<PathBuf, ToolError> {
    for var in ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "ANDROID_NDK"] {
        if let Some(dir) = std::env::var_os(var).map(PathBuf::from) {
            if dir.join("toolchains/llvm").is_dir() {
                return Ok(dir);
            }
        }
    }
    let has_llvm = |d: &Path| d.join("toolchains/llvm").is_dir();
    if let Some(dir) = newest_dir(&sdk.join("ndk"), has_llvm) {
        return Ok(dir);
    }
    let bundle = sdk.join("ndk-bundle");
    if has_llvm(&bundle) {
        return Ok(bundle);
    }
    Err(format!(
        "Android NDK not found under {} (install it with: sdkmanager \"ndk;28.2.13676358\", or set \
         ANDROID_NDK_HOME)",
        sdk.join("ndk").display()
    ))
}

fn find_build_tools(sdk: &Path) -> Result<PathBuf, ToolError> {
    let complete = |d: &Path| {
        d.join(exe("aapt2")).is_file()
            && d.join(exe("zipalign")).is_file()
            && d.join("lib/apksigner.jar").is_file()
    };
    newest_dir(&sdk.join("build-tools"), complete).ok_or_else(|| {
        format!(
            "no build-tools with aapt2, zipalign and apksigner under {} (install them with: \
             sdkmanager \"build-tools;36.0.0\")",
            sdk.join("build-tools").display()
        )
    })
}

fn find_platform(
    sdk: &Path,
    wanted: Option<u32>,
    at_least: u32,
) -> Result<(PathBuf, u32), ToolError> {
    let platforms = sdk.join("platforms");
    if let Some(level) = wanted {
        let jar = platforms.join(format!("android-{level}/android.jar"));
        return if jar.is_file() {
            Ok((jar, level))
        } else {
            Err(format!(
                "platform android-{level} is not installed (sdkmanager \"platforms;android-{level}\")"
            ))
        };
    }
    // Plain `android-NN` folders only: `android-36.1` style minor releases are skipped unless
    // asked for, so the build links against the API level it names.
    let mut found: Vec<(u32, PathBuf)> = std::fs::read_dir(&platforms)
        .map_err(|e| format!("{}: {e}", platforms.display()))?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let level: u32 = name.strip_prefix("android-")?.parse().ok()?;
            let jar = e.path().join("android.jar");
            jar.is_file().then_some((level, jar))
        })
        .collect();
    found.sort();
    match found.iter().find(|(level, _)| *level >= at_least) {
        Some((level, jar)) => Ok((jar.clone(), *level)),
        None => Err(format!(
            "no platform android-{at_least} or newer under {} (sdkmanager \"platforms;android-{at_least}\")",
            platforms.display()
        )),
    }
}

/// `java` from `JAVA_HOME`, else the `PATH`.
pub fn find_java() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("JAVA_HOME").map(PathBuf::from) {
        let java = home.join("bin").join(exe("java"));
        if java.is_file() {
            return Some(java);
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(exe("java")))
        .find(|p| p.is_file())
}

/// The user's Velvet tools folder (where `bundletool` may be kept).
pub fn user_tools_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("VELVET_TOOLS_DIR") {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("velvet").join("tools"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .map(|d| d.join("velvet").join("tools"))
    }
}

fn find_bundletool(sdk: &Path) -> Option<PathBuf> {
    if let Some(jar) = std::env::var_os("BUNDLETOOL_JAR").map(PathBuf::from) {
        if jar.is_file() {
            return Some(jar);
        }
    }
    let mut dirs: Vec<PathBuf> = user_tools_dir().into_iter().collect();
    dirs.push(sdk.join("bundletool"));
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut jars: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with("bundletool") && name.ends_with(".jar")
            })
            .collect();
        jars.sort_by_key(|p| version_key(&p.file_name().unwrap_or_default().to_string_lossy()));
        if let Some(jar) = jars.pop() {
            return Some(jar);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_sort_numerically() {
        let mut names = vec!["9.0.0", "36.1.0", "36.0.0", "28.2.13676358"];
        names.sort_by_key(|n| version_key(n));
        assert_eq!(names, vec!["9.0.0", "28.2.13676358", "36.0.0", "36.1.0"]);
        let mut jars = ["bundletool-all-1.18.3.jar", "bundletool-all-1.9.0.jar"];
        jars.sort_by_key(|n| version_key(n));
        assert_eq!(jars[1], "bundletool-all-1.18.3.jar");
    }

    #[test]
    fn a_fake_sdk_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let sdk = dir.path();
        let bt = sdk.join("build-tools/36.0.0");
        std::fs::create_dir_all(bt.join("lib")).unwrap();
        for f in [exe("aapt2"), exe("zipalign"), "lib/apksigner.jar".into()] {
            std::fs::write(bt.join(f), b"").unwrap();
        }
        std::fs::create_dir_all(sdk.join("build-tools/30.0.0")).unwrap();
        assert_eq!(find_build_tools(sdk).unwrap(), bt);
        for level in ["35", "36"] {
            let p = sdk.join(format!("platforms/android-{level}"));
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("android.jar"), b"").unwrap();
        }
        assert_eq!(find_platform(sdk, None, 34).unwrap().1, 35);
        assert_eq!(find_platform(sdk, None, 36).unwrap().1, 36);
        assert!(find_platform(sdk, Some(33), 33).is_err());
        assert!(find_platform(sdk, None, 37).is_err());
        let ndk = sdk.join("ndk/28.2.13676358");
        std::fs::create_dir_all(ndk.join("toolchains/llvm")).unwrap();
        std::fs::create_dir_all(sdk.join("ndk/26.1.10909125/toolchains/llvm")).unwrap();
        if std::env::var_os("ANDROID_NDK_HOME").is_none()
            && std::env::var_os("ANDROID_NDK_ROOT").is_none()
            && std::env::var_os("ANDROID_NDK").is_none()
        {
            assert_eq!(find_ndk(sdk).unwrap(), ndk);
        }
        std::fs::write(
            ndk.join("source.properties"),
            "Pkg.Revision = 28.2.13676358\n",
        )
        .unwrap();
        assert_eq!(
            read_property(&ndk.join("source.properties"), "Pkg.Revision").as_deref(),
            Some("28.2.13676358")
        );
    }
}
