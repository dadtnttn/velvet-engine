//! The Android manifest and resources of a Velvet game: `AndroidManifest.xml` for
//! `android.app.NativeActivity` (no Java code), the full-screen theme, the label, and the
//! launcher icons (legacy and adaptive).

use std::fs;
use std::path::Path;

use image::{imageops::FilterType, Rgba, RgbaImage};

use crate::android_config::{parse_hex_color, AndroidAppConfig};

/// Everything the manifest needs besides the configuration.
#[derive(Debug, Clone)]
pub struct ManifestValues<'a> {
    /// The configuration.
    pub config: &'a AndroidAppConfig,
    /// `versionCode`.
    pub version_code: u32,
    /// `versionName`.
    pub version_name: &'a str,
    /// The native library's name (`android.app.lib_name`).
    pub lib_name: &'a str,
    /// A debuggable build (`android:debuggable`, for `adb` debugging and profiling).
    pub debuggable: bool,
}

/// Configuration changes the activity handles itself: without them Android would destroy and
/// re-create the activity (on rotation, a keyboard, a font size change...), which a winit app
/// cannot survive (winit builds one event loop per process).
pub const CONFIG_CHANGES: &str = "orientation|screenSize|screenLayout|smallestScreenSize|keyboard|\
keyboardHidden|navigation|uiMode|density|fontScale|locale|layoutDirection|colorMode|touchscreen|\
mcc|mnc|fontWeightAdjustment|grammaticalGender";

/// The application manifest.
pub fn manifest_xml(v: &ManifestValues<'_>) -> String {
    let c = v.config;
    let mut extra = String::new();
    for permission in &c.permissions {
        extra.push_str(&format!(
            "    <uses-permission android:name=\"{}\"/>\n",
            xml_escape(permission)
        ));
    }
    for feature in &c.uses_features {
        extra.push_str(&format!(
            "    <uses-feature android:name=\"{}\" android:required=\"false\"/>\n",
            xml_escape(feature)
        ));
    }
    let debuggable = if v.debuggable {
        "\n        android:debuggable=\"true\""
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Written by `velvet android` from [package.metadata.velvet.android]; edits are overwritten. -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package}"
    android:versionCode="{code}"
    android:versionName="{name}"
    android:installLocation="auto">
    <uses-sdk android:minSdkVersion="{min}" android:targetSdkVersion="{target}"/>
    <!-- wgpu draws with Vulkan, or OpenGL ES 3 where Vulkan is missing or broken. -->
    <uses-feature android:glEsVersion="0x00030000" android:required="true"/>
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x400003" android:required="false"/>
    <uses-feature android:name="android.hardware.gamepad" android:required="false"/>
{extra}    <application
        android:label="@string/app_name"
        android:icon="@mipmap/ic_launcher"
        android:hasCode="false"
        android:appCategory="game"
        android:extractNativeLibs="false"
        android:enableOnBackInvokedCallback="false"
        android:theme="@style/VelvetTheme"{debuggable}>
        <!-- NativeActivity runs the game's `android_main` (velvet_platform::android_main!). -->
        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:launchMode="singleTask"
            android:screenOrientation="{orientation}"
            android:configChanges="{changes}"
            android:alwaysRetainTaskState="true"
            android:resizeableActivity="true">
            <meta-data android:name="android.app.lib_name" android:value="{lib}"/>
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
        package = xml_escape(&c.package),
        code = v.version_code,
        name = xml_escape(v.version_name),
        min = c.min_sdk,
        target = c.target_sdk,
        orientation = xml_escape(&c.orientation),
        changes = CONFIG_CHANGES,
        lib = xml_escape(v.lib_name),
    )
}

/// The manifest of an install-time asset pack (the module `name` of the app bundle).
pub fn asset_pack_manifest_xml(package: &str, name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:dist="http://schemas.android.com/apk/distribution"
    package="{package}"
    split="{name}">
    <dist:module dist:type="asset-pack">
        <dist:fusing dist:include="true"/>
        <dist:delivery>
            <dist:install-time/>
        </dist:delivery>
    </dist:module>
</manifest>
"#,
        package = xml_escape(package),
        name = xml_escape(name),
    )
}

/// Writes the resources (`values/`, `values-v28/`, the icons) into `res`.
pub fn write_resources(
    res: &Path,
    config: &AndroidAppConfig,
    label: &str,
    icon: Option<&Path>,
) -> Result<(), String> {
    let io = |e: std::io::Error| e.to_string();
    let parent = if config.fullscreen {
        "@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen"
    } else {
        "@android:style/Theme.DeviceDefault.NoActionBar"
    };
    fs::create_dir_all(res.join("values")).map_err(io)?;
    fs::create_dir_all(res.join("values-v28")).map_err(io)?;
    let image = match icon {
        Some(path) => image::open(path)
            .map_err(|e| format!("icon {}: {e}", path.display()))?
            .to_rgba8(),
        None => default_icon(512),
    };
    let background = config
        .icon_background
        .as_deref()
        .and_then(parse_hex_color)
        .unwrap_or_else(|| {
            let p = image.get_pixel(0, 0);
            if p[3] > 128 {
                [p[0], p[1], p[2]]
            } else {
                [0x24, 0x16, 0x33]
            }
        });
    fs::write(
        res.join("values/strings.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <string name=\"app_name\">{}</string>\n</resources>\n",
            android_string_escape(label)
        ),
    )
    .map_err(io)?;
    fs::write(
        res.join("values/colors.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <color name=\"ic_launcher_background\">#{:02X}{:02X}{:02X}</color>\n</resources>\n",
            background[0], background[1], background[2]
        ),
    )
    .map_err(io)?;
    let style = |extra: &str| {
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <style name=\"VelvetTheme\" parent=\"{parent}\">\n        <item name=\"android:windowBackground\">@android:color/black</item>\n{extra}    </style>\n</resources>\n"
        )
    };
    fs::write(res.join("values/styles.xml"), style("")).map_err(io)?;
    // Display cutouts: API 28 and up.
    fs::write(
        res.join("values-v28/styles.xml"),
        style(&format!(
            "        <item name=\"android:windowLayoutInDisplayCutoutMode\">{}</item>\n",
            config.cutout
        )),
    )
    .map_err(io)?;
    write_icons(res, &image)
}

/// Launcher icons: legacy `ic_launcher.png` per density, and an adaptive icon (API 26+) whose
/// foreground keeps the picture inside the 66 dp safe zone of the 108 dp layer.
fn write_icons(res: &Path, image: &RgbaImage) -> Result<(), String> {
    let densities = [
        ("mdpi", 1.0f32),
        ("hdpi", 1.5),
        ("xhdpi", 2.0),
        ("xxhdpi", 3.0),
        ("xxxhdpi", 4.0),
    ];
    for (name, scale) in densities {
        let dir = res.join(format!("mipmap-{name}"));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let legacy = (48.0 * scale).round() as u32;
        image::imageops::resize(image, legacy, legacy, FilterType::Lanczos3)
            .save(dir.join("ic_launcher.png"))
            .map_err(|e| e.to_string())?;
        let layer = (108.0 * scale).round() as u32;
        let inner = (66.0 * scale).round() as u32;
        let mut foreground = RgbaImage::new(layer, layer);
        let picture = image::imageops::resize(image, inner, inner, FilterType::Lanczos3);
        let at = ((layer - inner) / 2) as i64;
        image::imageops::overlay(&mut foreground, &picture, at, at);
        foreground
            .save(dir.join("ic_launcher_foreground.png"))
            .map_err(|e| e.to_string())?;
    }
    let anydpi = res.join("mipmap-anydpi-v26");
    fs::create_dir_all(&anydpi).map_err(|e| e.to_string())?;
    fs::write(
        anydpi.join("ic_launcher.xml"),
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">\n    <background android:drawable=\"@color/ic_launcher_background\"/>\n    <foreground android:drawable=\"@mipmap/ic_launcher_foreground\"/>\n</adaptive-icon>\n",
    )
    .map_err(|e| e.to_string())
}

/// A plain icon for games that have none yet: a soft violet disc on a dark square.
pub fn default_icon(size: u32) -> RgbaImage {
    let mut img = RgbaImage::new(size, size);
    let c = size as f32 / 2.0;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let dx = (x as f32 + 0.5 - c) / c;
        let dy = (y as f32 + 0.5 - c) / c;
        let d = (dx * dx + dy * dy).sqrt();
        let bg = [0x24u8, 0x16, 0x33];
        let disc = [0xB0u8, 0x5C, 0xD6];
        // Edge of the disc smoothed over a pixel.
        let edge = ((0.62 - d) * c).clamp(0.0, 1.0);
        let shade = 1.0 - 0.35 * (dx + dy + 1.4).clamp(0.0, 2.8) / 2.8;
        let mix = |a: u8, b: u8| (a as f32 * (1.0 - edge) + b as f32 * shade * edge).round() as u8;
        *pixel = Rgba([
            mix(bg[0], disc[0]),
            mix(bg[1], disc[1]),
            mix(bg[2], disc[2]),
            255,
        ]);
    }
    img
}

/// Escapes text for an XML attribute or element.
pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escapes text for an Android string resource (XML, then Android's own `'`, `"`, `\` and a
/// leading `@` or `?`).
pub fn android_string_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    for (i, ch) in text.chars().enumerate() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '@' | '?' if i == 0 => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_names_the_activity_and_library() {
        let config = AndroidAppConfig {
            package: "com.example.game".into(),
            permissions: vec!["android.permission.VIBRATE".into()],
            ..Default::default()
        };
        let xml = manifest_xml(&ManifestValues {
            config: &config,
            version_code: 7,
            version_name: "1.0 \"beta\"",
            lib_name: "my_game",
            debuggable: false,
        });
        assert!(xml.contains("android:name=\"android.app.NativeActivity\""));
        assert!(xml.contains("android:value=\"my_game\""));
        assert!(xml.contains("android:versionCode=\"7\""));
        assert!(xml.contains("1.0 &quot;beta&quot;"));
        assert!(xml.contains("android:minSdkVersion=\"26\""));
        assert!(xml.contains("android:targetSdkVersion=\"36\""));
        assert!(xml.contains("android:hasCode=\"false\""));
        assert!(xml.contains("android:appCategory=\"game\""));
        assert!(xml.contains("android.permission.VIBRATE"));
        assert!(xml.contains("screenOrientation=\"sensorLandscape\""));
        assert!(!xml.contains("debuggable"));
        assert!(asset_pack_manifest_xml("com.example.game", "media").contains("install-time"));
    }

    #[test]
    fn strings_are_escaped_for_android() {
        assert_eq!(
            android_string_escape("Queen's \"Night\""),
            "Queen\\'s \\\"Night\\\""
        );
        assert_eq!(android_string_escape("@home"), "\\@home");
        assert_eq!(android_string_escape("A & B <c>"), "A &amp; B &lt;c&gt;");
        assert_eq!(xml_escape("a<b>&'\""), "a&lt;b&gt;&amp;&apos;&quot;");
    }

    #[test]
    fn resources_and_icons_are_written() {
        let dir = tempfile::tempdir().unwrap();
        let res = dir.path().join("res");
        let config = AndroidAppConfig {
            package: "com.example.game".into(),
            ..Default::default()
        };
        write_resources(&res, &config, "My Game", None).unwrap();
        for f in [
            "values/strings.xml",
            "values/colors.xml",
            "values/styles.xml",
            "values-v28/styles.xml",
            "mipmap-xxxhdpi/ic_launcher.png",
            "mipmap-mdpi/ic_launcher_foreground.png",
            "mipmap-anydpi-v26/ic_launcher.xml",
        ] {
            assert!(res.join(f).is_file(), "{f}");
        }
        let icon = image::open(res.join("mipmap-xxxhdpi/ic_launcher.png")).unwrap();
        assert_eq!((icon.width(), icon.height()), (192, 192));
        let cutout = std::fs::read_to_string(res.join("values-v28/styles.xml")).unwrap();
        assert!(cutout.contains("shortEdges"));
    }
}
