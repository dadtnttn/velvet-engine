//! End-to-end tests of the build and the check, on pictures that the tests generate. Each test works in
//! its own folder under the system's temporary folder, which is removed afterwards.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{DynamicImage, Rgba, RgbaImage};
use velvet_artbuild::{build, check, ArtBuildError, ArtBuildOptions, ErrorCode, OutputFormat};

/// A game folder for one test, with `art/` and `art-src/`. The folder is removed when it is dropped.
struct Game {
    root: PathBuf,
}

impl Game {
    fn new(name: &str) -> Game {
        let root = std::env::temp_dir()
            .join("velvet-artbuild-tests")
            .join(format!("{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("art")).unwrap();
        fs::create_dir_all(root.join("art-src")).unwrap();
        Game { root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn manifest(&self, json: &str) {
        self.write("art/manifest.json", json.as_bytes());
    }

    fn config(&self, json: &str) {
        self.write("art-src/art-build.json", json.as_bytes());
    }

    fn picture(&self, relative: &str, picture: &RgbaImage) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        picture.save(path).unwrap();
    }

    fn solid(&self, relative: &str, width: u32, height: u32, color: [u8; 4]) {
        self.picture(relative, &RgbaImage::from_pixel(width, height, Rgba(color)));
    }

    /// The options of a build that writes PNG only, so that the test does not need ffmpeg.
    fn options(&self) -> ArtBuildOptions {
        let mut options = ArtBuildOptions::new(&self.root);
        options.formats = Some(vec![OutputFormat::Png]);
        options
    }

    fn out(&self, relative: &str) -> RgbaImage {
        image::open(self.path(&format!("art/{relative}")))
            .unwrap()
            .to_rgba8()
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Every file under `dir`, with its contents, sorted by path: a picture of the folder's state.
fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).unwrap().display().to_string();
                out.push((relative, fs::read(&path).unwrap()));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();
    files
}

/// A picture with a red top half and a blue bottom half: the crop that keeps one half is easy to see.
fn halves(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |_, y| {
        if y < height / 2 {
            Rgba([255, 0, 0, 255])
        } else {
            Rgba([0, 0, 255, 255])
        }
    })
}

/// A picture with a pattern that does not compress well, so that its PNG is large.
fn busy(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        let mixed = x.wrapping_mul(2_654_435_761) ^ y.wrapping_mul(40_503);
        Rgba([mixed as u8, (mixed >> 8) as u8, (mixed >> 16) as u8, 255])
    })
}

/// Whether ffmpeg is on the PATH with the two encoders that the WebP and AVIF outputs need.
fn ffmpeg_has_encoders() -> bool {
    let Ok(output) = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
    else {
        return false;
    };
    let listing = String::from_utf8_lossy(&output.stdout);
    listing
        .lines()
        .any(|line| line.split_whitespace().nth(1) == Some("libwebp"))
        && listing
            .lines()
            .any(|line| line.split_whitespace().nth(1) == Some("libaom-av1"))
}

const FORMAT_HEADER: &str = r#"{"format":"velvet-art","version":1,"entries":"#;

#[test]
fn the_three_fits_write_the_box_and_the_scale_multiplies_it() {
    let game = Game::new("fits");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[180,260]}},{{\"id\":\"icon.coin\",\"kind\":\"icon\",\"size\":[64,64]}},{{\"id\":\"bg.forest\",\"kind\":\"background\",\"size\":[1920,1080]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 300, 300, [200, 0, 0, 255]);
    game.solid("art-src/icon.coin.png", 200, 100, [0, 200, 0, 255]);
    let jpeg = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        800,
        600,
        image::Rgb([0, 0, 200]),
    ));
    fs::create_dir_all(game.path("art-src/background")).unwrap();
    jpeg.save(game.path("art-src/background/bg.forest.jpg"))
        .unwrap();
    game.config(r#"{"format":"velvet-art-build","version":1,"ids":{"icon.coin":{"fit":"contain"},"bg.forest":{"fit":"stretch"}}}"#);

    let report = build(&game.options()).unwrap();
    assert!(!report.has_failures(), "{report:?}");
    assert_eq!(game.out("card/card.wood.png").dimensions(), (180, 260));
    let icon = game.out("icon/icon.coin.png");
    assert_eq!(icon.dimensions(), (64, 64));
    assert_eq!(
        icon.get_pixel(0, 0).0[3],
        0,
        "contain leaves transparent bands"
    );
    assert_eq!(icon.get_pixel(32, 32).0[3], 255);
    assert_eq!(
        game.out("background/bg.forest.png").dimensions(),
        (1920, 1080)
    );
    assert_eq!(report.built.len(), 3);

    game.config(r#"{"format":"velvet-art-build","version":1,"scale":2}"#);
    let mut options = game.options();
    options.force = true;
    build(&options).unwrap();
    assert_eq!(game.out("card/card.wood.png").dimensions(), (360, 520));
    assert_eq!(game.out("icon/icon.coin.png").dimensions(), (128, 128));
}

#[test]
fn the_focus_decides_which_part_of_a_cover_crop_stays() {
    let game = Game::new("focus");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"character.lyra.portrait\",\"kind\":\"portrait\",\"size\":[100,100]}},{{\"id\":\"prop.well\",\"kind\":\"prop\",\"size\":[100,100]}}]}}"
    ));
    game.picture("art-src/character.lyra.portrait.png", &halves(100, 200));
    game.picture("art-src/prop.well.png", &halves(100, 200));
    game.config(
        r#"{"format":"velvet-art-build","version":1,"ids":{"prop.well":{"focus":[0.5,1]}}}"#,
    );
    build(&game.options()).unwrap();

    // A portrait keeps the top by default, and a prop with focus at the bottom keeps the bottom.
    let portrait = game.out("portrait/character.lyra.portrait.png");
    assert!(
        portrait.pixels().all(|p| p.0[0] == 255 && p.0[2] == 0),
        "portrait keeps the red top"
    );
    let prop = game.out("prop/prop.well.png");
    assert!(
        prop.pixels().all(|p| p.0[2] == 255 && p.0[0] == 0),
        "prop keeps the blue bottom"
    );
}

#[test]
fn variants_are_found_by_name_and_missing_names_are_listed_with_their_consequence() {
    let game = Game::new("variants");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[180,260],\"variants\":[\"red\",\"blue\"]}},{{\"id\":\"icon.coin\",\"kind\":\"icon\",\"size\":[64,64]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 180, 260, [9, 9, 9, 255]);
    game.solid("art-src/card.wood.red.png", 180, 260, [200, 10, 10, 255]);

    let report = build(&game.options()).unwrap();
    assert!(game.path("art/card/card.wood.red.png").is_file());
    assert!(!game.path("art/card/card.wood.blue.png").exists());
    assert!(report
        .missing
        .iter()
        .any(|m| m.id == "card.wood" && m.variant.as_deref() == Some("blue") && !m.has_file));
    assert!(report
        .missing
        .iter()
        .any(|m| m.name == "icon.coin" && m.variant.is_none() && !m.has_file));
    assert!(report.has_problems());
    assert!(!report.has_failures());
}

#[test]
fn an_output_is_built_once_until_its_source_or_its_settings_change_or_force_is_given() {
    let game = Game::new("stamps");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 40, 40, [200, 0, 0, 255]);

    assert_eq!(build(&game.options()).unwrap().built.len(), 1);
    let again = build(&game.options()).unwrap();
    assert_eq!(again.built.len(), 0);
    assert_eq!(again.up_to_date.len(), 1);
    assert!(game.path("art/.build-stamps.json").is_file());

    game.solid("art-src/card.wood.png", 40, 40, [0, 200, 0, 255]);
    assert_eq!(
        build(&game.options()).unwrap().built.len(),
        1,
        "a changed source is built again"
    );

    game.config(r#"{"format":"velvet-art-build","version":1,"scale":2}"#);
    assert_eq!(
        build(&game.options()).unwrap().built.len(),
        1,
        "a changed setting is built again"
    );

    let mut forced = game.options();
    forced.force = true;
    assert_eq!(
        build(&forced).unwrap().built.len(),
        1,
        "force rebuilds an up-to-date output"
    );
}

#[test]
fn files_in_art_that_no_entry_resolves_to_are_listed_and_never_deleted() {
    let game = Game::new("orphans");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 40, 40, [200, 0, 0, 255]);
    game.write("art/card/old.png", b"old");
    game.write("art/icon/card.wood.png", b"the wrong kind folder");
    game.write("art/card/card.wood.tiff", b"not a format");
    game.write("art/card/.leftover.png.tmp", b"a crash left this");

    let report = build(&game.options()).unwrap();
    for orphan in [
        "card/old.png",
        "icon/card.wood.png",
        "card/card.wood.tiff",
        "card/.leftover.png.tmp",
    ] {
        assert!(
            report.orphans.iter().any(|path| path == orphan),
            "{orphan} is an orphan: {report:?}"
        );
        assert!(
            game.path(&format!("art/{orphan}")).is_file(),
            "{orphan} must not be deleted"
        );
    }
    assert!(!report
        .orphans
        .iter()
        .any(|path| path == "card/card.wood.png"));
    assert!(!report
        .orphans
        .iter()
        .any(|path| path == "manifest.json" || path == ".build-stamps.json"));
}

#[test]
fn a_check_reports_what_a_build_would_do_and_writes_nothing() {
    let game = Game::new("check");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}},{{\"id\":\"icon.coin\",\"kind\":\"icon\",\"size\":[64,64]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 40, 40, [200, 0, 0, 255]);
    game.write("art/card/old.png", b"old");
    let before = snapshot(&game.root);

    let report = check(&game.options()).unwrap();
    assert!(report.dry_run);
    assert!(report
        .built
        .iter()
        .any(|record| record.path == "card/card.wood.png" && record.bytes.is_none()));
    assert!(report.missing.iter().any(|m| m.name == "icon.coin"));
    assert!(report.orphans.iter().any(|path| path == "card/old.png"));
    assert!(report.has_problems());
    assert_eq!(
        snapshot(&game.root),
        before,
        "a check writes nothing, not even stamps"
    );
}

#[test]
fn a_cover_that_cuts_a_lot_is_warned_and_a_contain_is_not() {
    let game = Game::new("aspect");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[180,260]}},{{\"id\":\"icon.coin\",\"kind\":\"icon\",\"size\":[64,64]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 300, 100, [200, 0, 0, 255]);
    game.solid("art-src/icon.coin.png", 300, 100, [0, 200, 0, 255]);
    game.config(
        r#"{"format":"velvet-art-build","version":1,"ids":{"icon.coin":{"fit":"contain"}}}"#,
    );

    let report = build(&game.options()).unwrap();
    assert_eq!(report.aspect.len(), 1, "{report:?}");
    let warning = &report.aspect[0];
    assert_eq!(warning.name, "card.wood");
    assert_eq!(warning.fit, "cover");
    assert_eq!(warning.source_size, [300, 100]);
    assert!(warning.difference_percent > 300.0);
}

#[test]
fn outputs_over_the_size_limit_are_listed() {
    let game = Game::new("heavy");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[180,260]}}]}}"
    ));
    game.picture("art-src/card.wood.png", &busy(180, 260));
    game.config(r#"{"format":"velvet-art-build","version":1,"max_bytes":100}"#);

    let report = build(&game.options()).unwrap();
    assert_eq!(report.heavy.len(), 1, "{report:?}");
    assert_eq!(report.heavy[0].path, "card/card.wood.png");
    assert_eq!(report.heavy[0].limit, 100);
    assert!(report.heavy[0].bytes > 100);
}

#[test]
fn without_ffmpeg_webp_and_avif_are_skipped_with_a_reason_and_png_is_still_written() {
    let game = Game::new("noffmpeg");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 40, 40, [200, 0, 0, 255]);
    let mut options = ArtBuildOptions::new(&game.root);
    options.formats = Some(OutputFormat::ALL.to_vec());
    options.ffmpeg = Some(PathBuf::from("velvet-artbuild-no-such-ffmpeg"));

    let report = build(&options).unwrap();
    let skipped: Vec<&str> = report
        .skipped
        .iter()
        .map(|item| item.format.as_str())
        .collect();
    assert_eq!(skipped, ["avif", "webp"]);
    assert!(report
        .skipped
        .iter()
        .all(|item| item.reason.contains("ffmpeg")));
    assert_eq!(report.built.len(), 1);
    assert_eq!(report.built[0].format, "png");
    assert!(!report.has_failures());
    assert!(report.has_problems());
}

#[test]
fn a_refused_manifest_builds_nothing_and_names_the_code() {
    let game = Game::new("refused");
    game.manifest(r#"{"format":"velvet-art","version":1,"entries":[{"id":"Bad","kind":"card","size":[1,1]}]}"#);
    match build(&game.options()) {
        Err(ArtBuildError::Refused { refusal, .. }) => assert_eq!(refusal.code, ErrorCode::Value),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(!game.path("art/card").exists());
}

#[test]
fn a_build_config_with_a_misspelt_name_is_refused_as_an_unknown_field() {
    let game = Game::new("config");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}}]}}"
    ));
    game.config(r#"{"format":"velvet-art-build","version":1,"fitt":"contain"}"#);
    match build(&game.options()) {
        Err(ArtBuildError::Refused { refusal, .. }) => {
            assert_eq!(refusal.code, ErrorCode::UnknownField)
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_missing_manifest_is_an_io_error() {
    let game = Game::new("nomanifest");
    assert!(matches!(
        build(&game.options()),
        Err(ArtBuildError::Io { .. })
    ));
}

#[test]
fn an_unreadable_source_fails_alone_and_the_rest_of_the_build_goes_on() {
    let game = Game::new("unreadable");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}},{{\"id\":\"icon.coin\",\"kind\":\"icon\",\"size\":[64,64]}}]}}"
    ));
    game.write("art-src/card.wood.png", b"this is not a picture");
    game.solid("art-src/icon.coin.png", 64, 64, [0, 0, 200, 255]);

    let report = build(&game.options()).unwrap();
    assert!(
        report
            .failed
            .iter()
            .any(|failure| failure.path == "card.wood.png"),
        "{report:?}"
    );
    assert!(report
        .built
        .iter()
        .any(|record| record.path == "icon/icon.coin.png"));
    assert!(report.has_failures());
}

#[test]
fn an_output_larger_than_the_webp_side_is_skipped_before_any_work() {
    let game = Game::new("toolarge");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"bg.forest\",\"kind\":\"background\",\"size\":[7680,4320]}}]}}"
    ));
    game.solid("art-src/bg.forest.png", 16, 9, [0, 200, 0, 255]);
    game.config(r#"{"format":"velvet-art-build","version":1,"scale":3}"#);

    let report = build(&game.options()).unwrap();
    assert_eq!(report.skipped.len(), 1);
    assert!(report.skipped[0].reason.contains("largest side"));
    assert!(!game.path("art/background/bg.forest.png").exists());
}

#[test]
fn an_explicit_source_name_wins_and_the_command_line_formats_override_the_config() {
    let game = Game::new("explicit");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[40,40]}}]}}"
    ));
    game.solid("art-src/card.wood.png", 40, 40, [200, 0, 0, 255]);
    let jpeg =
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(40, 40, image::Rgb([0, 200, 0])));
    jpeg.save(game.path("art-src/wood-final.jpeg")).unwrap();
    game.config(r#"{"format":"velvet-art-build","version":1,"formats":["avif"],"ids":{"card.wood":{"source":"wood-final.jpeg"}}}"#);

    let report = build(&game.options()).unwrap();
    assert_eq!(report.built.len(), 1);
    assert_eq!(report.built[0].format, "png");
    assert_eq!(report.built[0].source, "wood-final.jpeg");
    assert_eq!(game.out("card/card.wood.png").get_pixel(5, 5).0[1], 200);
}

#[test]
fn webp_and_avif_are_written_with_their_alpha_and_avif_sources_are_read_back() {
    if !ffmpeg_has_encoders() {
        eprintln!("skipped: ffmpeg with libwebp and libaom-av1 is not on the PATH");
        return;
    }
    let game = Game::new("ffmpeg");
    game.manifest(&format!(
        "{FORMAT_HEADER}[{{\"id\":\"card.wood\",\"kind\":\"card\",\"size\":[180,260]}},{{\"id\":\"icon.coin\",\"kind\":\"icon\",\"size\":[63,61]}}]}}"
    ));
    // A picture whose alpha ramps from transparent on the left to opaque on the right.
    let ramp = RgbaImage::from_fn(90, 130, |x, _| {
        Rgba([(x * 2) as u8, 120, 60, (x * 255 / 89) as u8])
    });
    game.picture("art-src/card.wood.png", &ramp);
    game.solid("art-src/icon.coin.png", 200, 200, [30, 40, 50, 255]);
    let mut options = game.options();
    options.formats = Some(vec![OutputFormat::Avif, OutputFormat::Webp]);

    let report = build(&options).unwrap();
    assert!(!report.has_failures(), "{report:?}");
    assert_eq!(report.built.len(), 4);

    let avif = fs::read(game.path("art/card/card.wood.avif")).unwrap();
    let decoded = velvet_avif::decode_avif(&avif).expect("the AVIF output decodes");
    assert_eq!(decoded.dimensions(), (180, 260));
    assert!(
        decoded.get_pixel(0, 130).0[3] < 32,
        "transparent on the left"
    );
    assert!(
        decoded.get_pixel(179, 130).0[3] > 224,
        "opaque on the right"
    );
    let icon =
        velvet_avif::decode_avif(&fs::read(game.path("art/icon/icon.coin.avif")).unwrap()).unwrap();
    assert_eq!(icon.dimensions(), (63, 61));
    assert_eq!(icon.get_pixel(30, 30).0[3], 255);

    let webp = fs::read(game.path("art/card/card.wood.webp")).unwrap();
    assert_eq!(&webp[0..4], b"RIFF");
    assert_eq!(&webp[8..12], b"WEBP");

    let again = build(&options).unwrap();
    assert_eq!(
        again.up_to_date.len(),
        4,
        "a second build finds every output up to date"
    );

    // The AVIF output is also a valid source: it is read back and fitted to a PNG.
    game.write("art-src/card.wood.avif", &avif);
    fs::remove_file(game.path("art-src/card.wood.png")).unwrap();
    let mut from_avif = game.options();
    from_avif.force = true;
    let report = build(&from_avif).unwrap();
    let record = report
        .built
        .iter()
        .find(|record| record.path == "card/card.wood.png")
        .expect("png built");
    assert_eq!(record.source, "card.wood.avif");
    assert_eq!(game.out("card/card.wood.png").dimensions(), (180, 260));
}
