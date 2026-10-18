//! The command surface: `ArtArgs` parses the subcommands and flags the way `velvet art` will, and `run`
//! returns the exit codes that the README promises.

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use velvet_artbuild::{run, ArtArgs, ArtCommand, OutputFormat};

/// A stand-in for the `velvet` CLI, which flattens `ArtArgs` into its `art` subcommand.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    art: ArtArgs,
}

#[test]
fn the_subcommands_and_the_flags_parse_before_or_after_the_subcommand() {
    let after = Cli::try_parse_from([
        "velvet",
        "build",
        "--game",
        "some/game",
        "--formats",
        "png,webp",
        "--force",
        "--json",
    ])
    .unwrap()
    .art;
    assert_eq!(after.command, ArtCommand::Build);
    assert_eq!(after.game, PathBuf::from("some/game"));
    assert_eq!(
        after.formats,
        Some(vec![OutputFormat::Png, OutputFormat::Webp])
    );
    assert!(after.force && after.json);

    let before = Cli::try_parse_from(["velvet", "--game", "g", "check"])
        .unwrap()
        .art;
    assert_eq!(before.command, ArtCommand::Check);
    assert_eq!(before.game, PathBuf::from("g"));
    assert_eq!(before.src, None);
    assert_eq!(before.formats, None);
    assert!(!before.force && !before.json);
    assert_eq!(
        Cli::try_parse_from(["velvet", "check"]).unwrap().art.game,
        PathBuf::from(".")
    );
}

#[test]
fn an_unknown_format_is_a_usage_error() {
    assert!(Cli::try_parse_from(["velvet", "build", "--formats", "svg"]).is_err());
}

#[test]
fn run_exits_zero_when_clean_and_one_when_a_check_finds_a_problem() {
    let root = std::env::temp_dir()
        .join("velvet-artbuild-tests")
        .join(format!("{}-cli", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("art")).unwrap();
    fs::create_dir_all(root.join("art-src")).unwrap();
    fs::write(
        root.join("art").join("manifest.json"),
        r#"{"format":"velvet-art","version":1,"entries":[{"id":"card.wood","kind":"card","size":[40,40]}]}"#,
    )
    .unwrap();
    let game = root.to_string_lossy().into_owned();
    let args = |command: &str, extra: &[&str]| {
        let mut argv = vec![
            "velvet",
            command,
            "--game",
            game.as_str(),
            "--formats",
            "png",
        ];
        argv.extend_from_slice(extra);
        Cli::try_parse_from(argv).unwrap().art
    };

    // No source picture yet: the check reports it, and exits with 1.
    assert_eq!(run(&args("check", &[])).unwrap(), 1);
    assert!(
        !root.join("art").join("card").exists(),
        "a check writes nothing"
    );

    // With the picture, the check is clean and the build exits with 0.
    image::RgbaImage::from_pixel(40, 40, image::Rgba([1, 2, 3, 255]))
        .save(root.join("art-src").join("card.wood.png"))
        .unwrap();
    assert_eq!(run(&args("check", &["--json"])).unwrap(), 0);
    assert_eq!(run(&args("build", &[])).unwrap(), 0);
    assert!(root
        .join("art")
        .join("card")
        .join("card.wood.png")
        .is_file());
    assert_eq!(run(&args("check", &[])).unwrap(), 0);
    let _ = fs::remove_dir_all(&root);
}
