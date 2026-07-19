//! Live **dev mode** for author assets (HTML-like hot reload).
//!
//! Watches filesystem paths under the demo `data/` tree and reloads without
//! process restart:
//! - `.vcss` → reparse stylesheet (previous good sheet kept on parse error)
//! - images (`.jpg`/`.png`) → re-decode into RGB buffers
//! - `.vstory` → soft-reload flag for host re-boot
//!
//! Uses shipped [`velvet_assets::HotReloader`] mtime polling.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use velvet_assets::HotReloader;
use velvet_style::{parse_stylesheet, Stylesheet};
use velvet_story::pack_rgb;

/// What kind of author asset a watch key maps to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchKind {
    /// Named stylesheet (e.g. `casino`).
    Stylesheet {
        /// Registry / world name.
        name: String,
    },
    /// Image slot consumed by the title / cards paint path.
    Image {
        /// Logical slot.
        slot: ImageSlot,
    },
    /// Velvet Story source.
    Story,
}

/// Paint slots that can hot-reload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageSlot {
    /// Lobby background.
    MenuBg,
    /// Logo emblem.
    Logo,
    /// Profile portrait.
    Portrait,
    /// Card art by id (`strike`, …).
    Card(String),
}

/// One successful or attempted reload batch from a poll.
#[derive(Debug, Default)]
pub struct LiveDevApply {
    /// New stylesheet if a `.vcss` reloaded successfully.
    pub stylesheet: Option<(String, Stylesheet)>,
    /// Reloaded images.
    pub images: Vec<(ImageSlot, RgbBuf)>,
    /// Story file changed — host may soft re-boot.
    pub story_reload: bool,
    /// Keys that changed this tick.
    pub reloaded: Vec<String>,
    /// Non-fatal errors (parse/load failed; previous state kept).
    pub errors: Vec<String>,
    /// Human log lines for status bar / stdout.
    pub log: Vec<String>,
}

/// Packed RGB image (w, h, pixels) — same layout as demo `RgbImage`.
pub type RgbBuf = (u32, u32, Vec<u32>);

/// Dev session: watch + apply author assets.
#[derive(Debug)]
pub struct LiveDevSession {
    reloader: HotReloader,
    kinds: HashMap<String, WatchKind>,
    /// Last good stylesheet by name (for fallback after bad parse).
    good_sheets: HashMap<String, Stylesheet>,
    /// Accumulated log (capped).
    pub log: Vec<String>,
    /// data root being watched.
    pub data_root: PathBuf,
    /// Total successful reloads (any kind).
    pub reload_count: u64,
}

impl LiveDevSession {
    /// Empty enabled session.
    pub fn new(data_root: impl Into<PathBuf>) -> Self {
        Self {
            reloader: HotReloader::new(),
            kinds: HashMap::new(),
            good_sheets: HashMap::new(),
            log: Vec::new(),
            data_root: data_root.into(),
            reload_count: 0,
        }
    }

    /// Disabled (no-op tick) — for non-dev launches.
    pub fn disabled(data_root: impl Into<PathBuf>) -> Self {
        let mut s = Self::new(data_root);
        s.reloader = HotReloader::disabled();
        s
    }

    /// Whether watching is enabled.
    pub fn enabled(&self) -> bool {
        self.reloader.enabled
    }

    /// Number of watched files.
    pub fn watch_count(&self) -> usize {
        self.reloader.len()
    }

    /// Register standard velvet-stakes author paths under `data_root`.
    pub fn watch_stakes_tree(data_root: &Path) -> Self {
        let mut s = Self::new(data_root);
        s.watch_stylesheet(
            "style:casino",
            "casino",
            data_root.join("styles/casino.vcss"),
        );
        s.watch_image(
            "img:menu_bg",
            ImageSlot::MenuBg,
            data_root.join("ui/menu_bg.jpg"),
        );
        s.watch_image(
            "img:logo",
            ImageSlot::Logo,
            data_root.join("ui/logo_emblem.jpg"),
        );
        s.watch_image(
            "img:portrait",
            ImageSlot::Portrait,
            data_root.join("ui/portrait_collector.jpg"),
        );
        for id in ["strike", "guard", "fireball", "focus", "bash"] {
            s.watch_image(
                format!("img:card:{id}"),
                ImageSlot::Card(id.into()),
                data_root.join(format!("art/{id}.jpg")),
            );
        }
        s.watch_story("story:main", data_root.join("story/main.vstory"));
        s.push_log(format!(
            "dev: watching {} files under {}",
            s.watch_count(),
            data_root.display()
        ));
        s
    }

    /// Watch a `.vcss` path.
    pub fn watch_stylesheet(
        &mut self,
        key: impl Into<String>,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) {
        let key = key.into();
        let name = name.into();
        let path = path.into();
        // Seed last-good if readable
        if let Ok(src) = std::fs::read_to_string(&path) {
            if let Ok(sheet) = parse_stylesheet(&src) {
                self.good_sheets.insert(name.clone(), sheet);
            }
        }
        self.reloader.watch(key.clone(), path);
        self.kinds
            .insert(key, WatchKind::Stylesheet { name });
    }

    /// Watch an image path.
    pub fn watch_image(
        &mut self,
        key: impl Into<String>,
        slot: ImageSlot,
        path: impl Into<PathBuf>,
    ) {
        let key = key.into();
        self.reloader.watch(key.clone(), path);
        self.kinds.insert(key, WatchKind::Image { slot });
    }

    /// Watch a `.vstory` path.
    pub fn watch_story(&mut self, key: impl Into<String>, path: impl Into<PathBuf>) {
        let key = key.into();
        self.reloader.watch(key.clone(), path);
        self.kinds.insert(key, WatchKind::Story);
    }

    /// Path for a key (tests / diagnostics).
    pub fn path_of(&self, key: &str) -> Option<&Path> {
        self.reloader.path_of(key)
    }

    /// Force mark a key dirty (tests) then [`tick`].
    pub fn force_tick_key(&mut self, key: &str) -> LiveDevApply {
        self.reloader.mark_changed(key);
        self.apply_keys(vec![key.to_string()])
    }

    /// Poll disk mtimes and apply any changes.
    pub fn tick(&mut self) -> LiveDevApply {
        let changed = self.reloader.poll();
        if changed.is_empty() {
            return LiveDevApply::default();
        }
        self.apply_keys(changed)
    }

    fn apply_keys(&mut self, keys: Vec<String>) -> LiveDevApply {
        let mut out = LiveDevApply {
            reloaded: keys.clone(),
            ..Default::default()
        };
        for key in keys {
            let Some(kind) = self.kinds.get(&key).cloned() else {
                continue;
            };
            let Some(path) = self.reloader.path_of(&key).map(|p| p.to_path_buf()) else {
                continue;
            };
            match kind {
                WatchKind::Stylesheet { name } => match reload_stylesheet(&path) {
                    Ok(sheet) => {
                        let rules = sheet.rules.len();
                        let fns = sheet.script.functions.len();
                        self.good_sheets.insert(name.clone(), sheet.clone());
                        out.stylesheet = Some((name.clone(), sheet));
                        self.reload_count += 1;
                        let msg = format!(
                            "dev: reloaded .vcss `{name}` ({rules} rules, {fns} fns) from {}",
                            path.display()
                        );
                        out.log.push(msg.clone());
                        self.push_log(msg);
                    }
                    Err(e) => {
                        let msg = format!(
                            "dev: .vcss parse failed (kept previous): {} — {e}",
                            path.display()
                        );
                        out.errors.push(msg.clone());
                        out.log.push(msg.clone());
                        self.push_log(msg);
                        // restore last good if any
                        if let Some(good) = self.good_sheets.get(&name) {
                            out.stylesheet = Some((name.clone(), good.clone()));
                        }
                    }
                },
                WatchKind::Image { slot } => match load_rgb_buf(&path) {
                    Some(buf) => {
                        let (w, h) = (buf.0, buf.1);
                        out.images.push((slot.clone(), buf));
                        self.reload_count += 1;
                        let msg = format!(
                            "dev: reloaded image {:?} {}x{} from {}",
                            slot,
                            w,
                            h,
                            path.display()
                        );
                        out.log.push(msg.clone());
                        self.push_log(msg);
                    }
                    None => {
                        let msg = format!("dev: image load failed: {}", path.display());
                        out.errors.push(msg.clone());
                        out.log.push(msg.clone());
                        self.push_log(msg);
                    }
                },
                WatchKind::Story => {
                    out.story_reload = true;
                    self.reload_count += 1;
                    let msg = format!("dev: story changed {}", path.display());
                    out.log.push(msg.clone());
                    self.push_log(msg);
                }
            }
        }
        out
    }

    fn push_log(&mut self, line: String) {
        eprintln!("{line}");
        self.log.push(line);
        if self.log.len() > 40 {
            let n = self.log.len() - 40;
            self.log.drain(0..n);
        }
    }

    /// Last-good stylesheet by name.
    pub fn good_sheet(&self, name: &str) -> Option<&Stylesheet> {
        self.good_sheets.get(name)
    }
}

/// Load + parse a stylesheet from disk (shipped path used by dev + boot).
pub fn reload_stylesheet(path: &Path) -> Result<Stylesheet, String> {
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_stylesheet(&src).map_err(|e| e.to_string())
}

/// Decode image file to packed RGB (same packing as demo renderer).
pub fn load_rgb_buf(path: &Path) -> Option<RgbBuf> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut px = Vec::with_capacity((w * h) as usize);
    for p in rgba.pixels() {
        let [r, g, b, _a] = p.0;
        px.push(pack_rgb(r, g, b));
    }
    Some((w, h, px))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "velvet_live_dev_{}_{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::create_dir_all(dir.join("styles"));
        let _ = std::fs::create_dir_all(dir.join("ui"));
        dir
    }

    fn write_file(path: &Path, body: &str) {
        if let Some(p) = path.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    /// Ensure mtime advances on coarse filesystems.
    fn bump_mtime(path: &Path, body: &str) {
        std::thread::sleep(Duration::from_millis(30));
        write_file(path, body);
        // 1s resolution fallback
        let mut session = LiveDevSession::new(path.parent().unwrap());
        session.watch_stylesheet("t", "t", path);
        if session.tick().stylesheet.is_none() {
            std::thread::sleep(Duration::from_millis(1100));
            write_file(path, body);
        }
    }

    #[test]
    fn vcss_reload_updates_rule_count_via_shipped_tick() {
        let dir = temp_dir("vcss");
        let path = dir.join("styles/casino.vcss");
        write_file(
            &path,
            r#"
            .button { color: #aaaaaa; height: 40; }
            "#,
        );
        let mut dev = LiveDevSession::new(&dir);
        dev.watch_stylesheet("style:casino", "casino", &path);
        let baseline = dev.good_sheet("casino").expect("seeded");
        assert_eq!(baseline.rules.len(), 1);

        // mutate: add rule + keyframes + script
        let v2 = r#"
            .button { color: #ffffff; height: 52; }
            .button:selected { color: #ffe496; }
            @keyframes deal { from { opacity: 0; } to { opacity: 1; } }
            @script {
              fn dealHand(n) {
                for (let i = 0; i < n; i = i + 1) {
                  play("deal", { target: "card" + i, delay: i * 0.08 });
                }
              }
            }
        "#;
        std::thread::sleep(Duration::from_millis(30));
        write_file(&path, v2);
        let mut apply = dev.tick();
        if apply.stylesheet.is_none() {
            std::thread::sleep(Duration::from_millis(1100));
            write_file(&path, v2);
            apply = dev.tick();
        }
        let (name, sheet) = apply.stylesheet.expect("stylesheet reloaded");
        assert_eq!(name, "casino");
        assert_eq!(sheet.rules.len(), 2, "rules should grow after edit");
        assert!(sheet.keyframes.contains_key("deal"));
        assert!(sheet.script.functions.contains_key("dealHand"));
        assert!(dev.reload_count >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_vcss_keeps_previous_good_sheet() {
        let dir = temp_dir("bad_vcss");
        let path = dir.join("styles/x.vcss");
        write_file(&path, ".ok { color: #00ff00; }\n");
        let mut dev = LiveDevSession::new(&dir);
        dev.watch_stylesheet("style:x", "x", &path);
        assert_eq!(dev.good_sheet("x").unwrap().rules.len(), 1);

        std::thread::sleep(Duration::from_millis(30));
        write_file(&path, "this is {{{ not valid vcss");
        let mut apply = dev.tick();
        if apply.reloaded.is_empty() {
            // force path for stubborn mtime
            apply = dev.force_tick_key("style:x");
        }
        assert!(
            !apply.errors.is_empty() || apply.stylesheet.is_some(),
            "should report error or restore good"
        );
        // last-good still 1 rule
        assert_eq!(dev.good_sheet("x").unwrap().rules.len(), 1);
        if let Some((_, sheet)) = apply.stylesheet {
            assert_eq!(sheet.rules.len(), 1, "must not apply broken sheet");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn image_reload_changes_pixel_buffer() {
        let dir = temp_dir("img");
        let path = dir.join("ui/menu_bg.jpg");
        // write tiny PNGs via image crate (jpeg optional)
        let path_png = dir.join("ui/menu_bg.png");
        {
            let mut img = image::RgbImage::new(4, 4);
            for p in img.pixels_mut() {
                *p = image::Rgb([10, 20, 30]);
            }
            img.save(&path_png).unwrap();
        }
        let mut dev = LiveDevSession::new(&dir);
        dev.watch_image("img:menu_bg", ImageSlot::MenuBg, &path_png);
        // first force load via mark
        let a1 = dev.force_tick_key("img:menu_bg");
        assert_eq!(a1.images.len(), 1);
        let (_, (_, _, px1)) = &a1.images[0];
        assert_eq!(px1.len(), 16);
        let first = px1[0];

        std::thread::sleep(Duration::from_millis(30));
        {
            let mut img = image::RgbImage::new(4, 4);
            for p in img.pixels_mut() {
                *p = image::Rgb([200, 10, 10]);
            }
            img.save(&path_png).unwrap();
        }
        let mut a2 = dev.tick();
        if a2.images.is_empty() {
            std::thread::sleep(Duration::from_millis(1100));
            {
                let mut img = image::RgbImage::new(4, 4);
                for p in img.pixels_mut() {
                    *p = image::Rgb([200, 10, 10]);
                }
                img.save(&path_png).unwrap();
            }
            a2 = dev.tick();
        }
        if a2.images.is_empty() {
            a2 = dev.force_tick_key("img:menu_bg");
        }
        assert_eq!(a2.images.len(), 1);
        let (_, (_, _, px2)) = &a2.images[0];
        assert_ne!(px2[0], first, "pixel bytes must change after rewrite");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = path; // silence
    }

    #[test]
    fn story_change_sets_reload_flag() {
        let dir = temp_dir("story");
        let path = dir.join("story/main.vstory");
        write_file(&path, "scene start\nend\n");
        let mut dev = LiveDevSession::new(&dir);
        dev.watch_story("story:main", &path);
        std::thread::sleep(Duration::from_millis(30));
        write_file(&path, "scene start\nnarrator:\n    hi\nend\n");
        let mut apply = dev.tick();
        if !apply.story_reload {
            std::thread::sleep(Duration::from_millis(1100));
            write_file(&path, "scene start\nnarrator:\n    hi2\nend\n");
            apply = dev.tick();
        }
        if !apply.story_reload {
            apply = dev.force_tick_key("story:main");
        }
        assert!(apply.story_reload);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn watch_stakes_tree_registers_disk_paths() {
        // Prefer real demo data if present
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        if !root.join("styles/casino.vcss").exists() {
            return;
        }
        let dev = LiveDevSession::watch_stakes_tree(&root);
        assert!(dev.enabled());
        assert!(dev.watch_count() >= 5);
        assert!(dev.path_of("style:casino").unwrap().exists());
        assert!(dev.good_sheet("casino").is_some());
        // filesystem load, not include_str-only
        let p = dev.path_of("style:casino").unwrap();
        assert!(p.ends_with("casino.vcss"));
    }

    #[test]
    fn bump_helper_compiles() {
        let dir = temp_dir("bump");
        let path = dir.join("styles/a.vcss");
        write_file(&path, ".a { color: #111111; }\n");
        bump_mtime(&path, ".a { color: #222222; }\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
