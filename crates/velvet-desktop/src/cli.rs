//! Command-line arguments of the `velvet-desktop` binary.
//!
//! Usage: `velvet-desktop <game-folder> [--dev <url>] [--devtools] [--size WxH]
//! [--capture <file.png> [--after <ms>]]`. With no arguments, the program runs the game in `game/`
//! next to its own executable, when that folder has a manifest (see [`packaged_game`]). The parser
//! takes the arguments without the program name and the program's folder, so it can be tested
//! without a process.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::manifest::MANIFEST_FILE;
use crate::origin::origin_of;

/// The folder next to the program that holds a packaged game.
pub const PACKAGED_GAME_DIR: &str = "game";

/// The wait after the page loads, before a `--capture` picture, when `--after` is not given.
pub const DEFAULT_AFTER_MS: u64 = 2000;

/// The longest `--after` wait, in milliseconds (ten minutes).
pub const MAX_AFTER_MS: u64 = 600_000;

/// The smallest side that `--size` takes, in device pixels.
pub const MIN_SIZE_SIDE: u32 = 64;

/// The largest side that `--size` takes, in device pixels (8K).
pub const MAX_SIZE_SIDE: u32 = 7680;

/// The text that `--help` prints.
pub const USAGE: &str = "\
Usage: velvet-desktop <game-folder> [--dev <url>] [--devtools] [--size <W>x<H>]
[--capture <file.png> [--after <ms>]]

  <game-folder>        folder with velvet-game.json and the game files
  --dev <url>          load a dev server (for example http://localhost:5173) instead of the files
  --devtools           open the web inspector (debug builds always have it)
  --size <W>x<H>       window size in device pixels, for example 1600x900 (never full screen)
  --capture <file.png> save a picture of the page to this PNG file, then exit
  --after <ms>         wait this long after the page loads, before the picture (default 2000)
  -h, --help           show this help

With no arguments, runs the game in game/ next to this program, if it has a velvet-game.json.";

/// What the binary should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print [`USAGE`] and exit.
    Help,
    /// Open a game.
    Run(RunOptions),
}

/// Options for opening a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOptions {
    /// The game folder, with the manifest and the files.
    pub game_dir: PathBuf,
    /// The dev server address, when `--dev` was given.
    pub dev_url: Option<String>,
    /// Whether `--devtools` was given.
    pub devtools: bool,
    /// The window size, when `--size` was given.
    pub size: Option<WindowSize>,
    /// The picture to take, when `--capture` was given. The run then ends after the picture.
    pub capture: Option<CaptureOptions>,
}

/// A window size from `--size`, in device pixels. Each side is from [`MIN_SIZE_SIDE`] to
/// [`MAX_SIZE_SIDE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// The width, in device pixels.
    pub width: u32,
    /// The height, in device pixels.
    pub height: u32,
}

impl WindowSize {
    /// Reads `WIDTHxHEIGHT`, with `x` or `X` between the two sides. Each side must be in range.
    pub fn parse(text: &str) -> Result<Self, CliError> {
        let refused = || CliError::BadSize(text.to_string());
        let (width, height) = text.split_once(['x', 'X']).ok_or_else(refused)?;
        let width: u32 = width.parse().map_err(|_| refused())?;
        let height: u32 = height.parse().map_err(|_| refused())?;
        let in_range = |side: u32| (MIN_SIZE_SIDE..=MAX_SIZE_SIDE).contains(&side);
        if in_range(width) && in_range(height) {
            Ok(Self { width, height })
        } else {
            Err(refused())
        }
    }
}

/// What `--capture` asks for: the PNG file to write, and the wait before the picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureOptions {
    /// The file to write. Its name ends in `.png`.
    pub path: PathBuf,
    /// The wait after the page has loaded, in milliseconds.
    pub after_ms: u64,
}

/// A problem with the command line.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CliError {
    /// No game folder was given.
    #[error("no game folder given")]
    MissingFolder,
    /// A second folder was given.
    #[error("unexpected extra argument `{0}`")]
    ExtraArgument(String),
    /// The option is not known.
    #[error("unknown option `{0}`")]
    UnknownOption(String),
    /// `--dev` has no address after it.
    #[error("`--dev` needs an address, for example --dev http://localhost:5173")]
    MissingDevUrl,
    /// `--dev` was given twice.
    #[error("`--dev` was given more than once")]
    RepeatedDev,
    /// The dev address is not an http or https address.
    #[error("`{0}` is not an http:// or https:// address")]
    BadDevUrl(String),
    /// An option that needs a value was given without one. The name is the option.
    #[error("`{0}` needs a value, see --help")]
    MissingValue(&'static str),
    /// An option that may be given only once was given again. The name is the option.
    #[error("`{0}` was given more than once")]
    RepeatedOption(&'static str),
    /// The picture file name does not end in `.png`.
    #[error("the picture file `{0}` must end in .png")]
    NotPng(String),
    /// The `--after` value is not a whole number of milliseconds in range.
    #[error("`{0}` is not a wait: use whole milliseconds, from 0 to 600000 (ten minutes)")]
    BadAfter(String),
    /// `--after` was given without `--capture`, so there is nothing to wait for.
    #[error("`--after` only works together with `--capture`")]
    AfterWithoutCapture,
    /// The `--size` value is not WIDTHxHEIGHT with both sides in range.
    #[error(
        "`{0}` is not a size: use WIDTHxHEIGHT, for example 1600x900, with each side from 64 to \
         7680 pixels"
    )]
    BadSize(String),
}

/// Reads the command-line arguments, without the program name.
///
/// With no arguments at all, the result is the packaged game in `exe_dir` (see [`packaged_game`]),
/// or [`Command::Help`] when there is none. `exe_dir` is the folder of the running program. It is
/// passed in rather than read here, so the tests can use a temporary folder.
pub fn parse<I, S>(args: I, exe_dir: Option<&Path>) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        return Ok(match exe_dir.and_then(packaged_game) {
            Some(game_dir) => Command::Run(RunOptions {
                game_dir,
                dev_url: None,
                devtools: false,
                size: None,
                capture: None,
            }),
            None => Command::Help,
        });
    }

    let mut args = args.into_iter();
    let mut game_dir: Option<PathBuf> = None;
    let mut dev_url: Option<String> = None;
    let mut devtools = false;
    let mut size: Option<WindowSize> = None;
    let mut capture_path: Option<String> = None;
    let mut after_ms: Option<u64> = None;

    while let Some(arg) = args.next() {
        if arg == "-h" || arg == "--help" {
            return Ok(Command::Help);
        }
        if arg == "--devtools" {
            devtools = true;
        } else if arg == "--dev" {
            let url = args.next().ok_or(CliError::MissingDevUrl)?;
            set_dev_url(&mut dev_url, url)?;
        } else if let Some(url) = arg.strip_prefix("--dev=") {
            let url = url.to_string();
            set_dev_url(&mut dev_url, url)?;
        } else if let Some(value) = option_value(&arg, "--size", &mut args) {
            set_once(&mut size, WindowSize::parse(&value?)?, "--size")?;
        } else if let Some(value) = option_value(&arg, "--capture", &mut args) {
            set_once(&mut capture_path, value?, "--capture")?;
        } else if let Some(value) = option_value(&arg, "--after", &mut args) {
            set_once(&mut after_ms, parse_after(&value?)?, "--after")?;
        } else if arg.starts_with('-') {
            return Err(CliError::UnknownOption(arg));
        } else if game_dir.is_some() {
            return Err(CliError::ExtraArgument(arg));
        } else {
            game_dir = Some(PathBuf::from(arg));
        }
    }

    let game_dir = game_dir.ok_or(CliError::MissingFolder)?;
    let capture = match (capture_path, after_ms) {
        (Some(path), after) => {
            if !has_png_extension(&path) {
                return Err(CliError::NotPng(path));
            }
            Some(CaptureOptions {
                path: PathBuf::from(path),
                after_ms: after.unwrap_or(DEFAULT_AFTER_MS),
            })
        }
        (None, Some(_)) => return Err(CliError::AfterWithoutCapture),
        (None, None) => None,
    };
    Ok(Command::Run(RunOptions {
        game_dir,
        dev_url,
        devtools,
        size,
        capture,
    }))
}

/// The packaged game in `exe_dir/game`, when that folder has a `velvet-game.json`.
///
/// A release folder has this layout (see the crate README). Only the file's presence is checked
/// here. The manifest is read and checked when the game starts, so a broken one is reported then.
pub fn packaged_game(exe_dir: &Path) -> Option<PathBuf> {
    let game_dir = exe_dir.join(PACKAGED_GAME_DIR);
    game_dir.join(MANIFEST_FILE).is_file().then_some(game_dir)
}

/// Reads the value of an option that is written `--name value` or `--name=value`. Returns `None`
/// when `arg` is another option, and an error when the value is missing.
fn option_value(
    arg: &str,
    name: &'static str,
    rest: &mut impl Iterator<Item = String>,
) -> Option<Result<String, CliError>> {
    if arg == name {
        return Some(rest.next().ok_or(CliError::MissingValue(name)));
    }
    let inline = arg.strip_prefix(name)?.strip_prefix('=')?;
    Some(Ok(inline.to_string()))
}

/// Stores the value of an option that may be given once. A second value is an error.
fn set_once<T>(slot: &mut Option<T>, value: T, name: &'static str) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::RepeatedOption(name));
    }
    *slot = Some(value);
    Ok(())
}

/// Reads an `--after` wait: whole milliseconds, from 0 to [`MAX_AFTER_MS`].
fn parse_after(text: &str) -> Result<u64, CliError> {
    text.parse::<u64>()
        .ok()
        .filter(|ms| *ms <= MAX_AFTER_MS)
        .ok_or_else(|| CliError::BadAfter(text.to_string()))
}

/// Whether a file name ends in `.png`, in any letter case.
fn has_png_extension(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

fn set_dev_url(slot: &mut Option<String>, url: String) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::RepeatedDev);
    }
    let is_web = url.starts_with("http://") || url.starts_with("https://");
    if !is_web || origin_of(&url).is_none() {
        return Err(CliError::BadDevUrl(url));
    }
    *slot = Some(url);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(game: &str) -> RunOptions {
        run_at(PathBuf::from(game))
    }

    fn run_at(game_dir: PathBuf) -> RunOptions {
        RunOptions {
            game_dir,
            dev_url: None,
            devtools: false,
            size: None,
            capture: None,
        }
    }

    /// A program folder with a packaged game (a manifest in `game/`).
    fn program_with_game() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        let game_dir = dir.path().join(PACKAGED_GAME_DIR);
        std::fs::create_dir_all(&game_dir).expect("game folder");
        std::fs::write(game_dir.join(MANIFEST_FILE), "{}").expect("manifest");
        dir
    }

    #[test]
    fn a_folder_alone_runs_the_game() {
        assert_eq!(parse(["game"], None), Ok(Command::Run(run("game"))));
    }

    #[test]
    fn devtools_can_be_added_after_the_folder() {
        let expected = RunOptions {
            devtools: true,
            ..run("game")
        };
        assert_eq!(
            parse(["game", "--devtools"], None),
            Ok(Command::Run(expected))
        );
    }

    #[test]
    fn a_dev_server_can_be_given_in_both_forms() {
        let expected = RunOptions {
            dev_url: Some("http://localhost:5173".into()),
            ..run("game")
        };
        assert_eq!(
            parse(["--dev", "http://localhost:5173", "game"], None),
            Ok(Command::Run(expected.clone()))
        );
        assert_eq!(
            parse(["game", "--dev=http://localhost:5173"], None),
            Ok(Command::Run(expected))
        );
    }

    #[test]
    fn https_dev_servers_are_accepted() {
        assert!(matches!(
            parse(["game", "--dev", "https://dev.example:8443/app/"], None),
            Ok(Command::Run(RunOptions {
                dev_url: Some(_),
                ..
            }))
        ));
    }

    #[test]
    fn help_is_recognised_anywhere_before_an_error() {
        assert_eq!(parse(["--help"], None), Ok(Command::Help));
        assert_eq!(parse(["game", "-h"], None), Ok(Command::Help));
    }

    #[test]
    fn options_without_a_folder_are_reported() {
        assert_eq!(parse(["--devtools"], None), Err(CliError::MissingFolder));
    }

    #[test]
    fn extra_folders_and_unknown_options_are_reported() {
        assert_eq!(
            parse(["a", "b"], None),
            Err(CliError::ExtraArgument("b".into()))
        );
        assert_eq!(
            parse(["--nope", "game"], None),
            Err(CliError::UnknownOption("--nope".into()))
        );
    }

    #[test]
    fn dev_option_problems_are_reported() {
        assert_eq!(parse(["game", "--dev"], None), Err(CliError::MissingDevUrl));
        assert_eq!(
            parse(["--dev", "ftp://x", "game"], None),
            Err(CliError::BadDevUrl("ftp://x".into()))
        );
        assert_eq!(
            parse(["--dev", "http://", "game"], None),
            Err(CliError::BadDevUrl("http://".into()))
        );
        assert_eq!(
            parse(["--dev", "localhost:5173", "game"], None),
            Err(CliError::BadDevUrl("localhost:5173".into()))
        );
        assert_eq!(
            parse(["--dev", "http://a:1", "--dev", "http://b:2", "game"], None),
            Err(CliError::RepeatedDev)
        );
    }

    #[test]
    fn the_usage_text_lists_every_option() {
        for option in [
            "<game-folder>",
            "--dev <url>",
            "--devtools",
            "--size <W>x<H>",
            "--capture <file.png>",
            "--after <ms>",
            "--help",
        ] {
            assert!(USAGE.contains(option), "usage lists {option}");
        }
    }

    #[test]
    fn no_arguments_run_the_packaged_game_next_to_the_program() {
        let program = program_with_game();
        assert_eq!(
            parse(Vec::<String>::new(), Some(program.path())),
            Ok(Command::Run(run_at(program.path().join(PACKAGED_GAME_DIR))))
        );
    }

    #[test]
    fn no_arguments_show_the_usage_when_no_game_is_packaged() {
        let dir = tempfile::tempdir().expect("temp dir");
        // A `game` folder without a manifest is not a game.
        std::fs::create_dir_all(dir.path().join(PACKAGED_GAME_DIR)).expect("game folder");
        assert_eq!(
            parse(Vec::<String>::new(), Some(dir.path())),
            Ok(Command::Help)
        );
        assert_eq!(parse(Vec::<String>::new(), None), Ok(Command::Help));
    }

    #[test]
    fn the_packaged_game_is_used_only_when_there_are_no_arguments() {
        let program = program_with_game();
        assert_eq!(
            parse(["--devtools"], Some(program.path())),
            Err(CliError::MissingFolder)
        );
    }

    #[test]
    fn packaged_game_needs_the_manifest_file_not_just_the_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert_eq!(packaged_game(dir.path()), None);
        std::fs::create_dir_all(dir.path().join(PACKAGED_GAME_DIR)).expect("game folder");
        assert_eq!(packaged_game(dir.path()), None);
        let program = program_with_game();
        assert_eq!(
            packaged_game(program.path()),
            Some(program.path().join(PACKAGED_GAME_DIR))
        );
    }

    #[test]
    fn a_capture_takes_a_png_file_and_the_default_wait() {
        let expected = RunOptions {
            capture: Some(CaptureOptions {
                path: PathBuf::from("shot.png"),
                after_ms: DEFAULT_AFTER_MS,
            }),
            ..run("game")
        };
        assert_eq!(
            parse(["game", "--capture", "shot.png"], None),
            Ok(Command::Run(expected))
        );
        assert_eq!(DEFAULT_AFTER_MS, 2000);
    }

    #[test]
    fn capture_and_size_take_both_forms_in_any_order() {
        let expected = RunOptions {
            capture: Some(CaptureOptions {
                path: PathBuf::from("out/shot.PNG"),
                after_ms: 3000,
            }),
            size: Some(WindowSize {
                width: 1600,
                height: 900,
            }),
            ..run("game")
        };
        assert_eq!(
            parse(
                [
                    "--capture=out/shot.PNG",
                    "--size=1600x900",
                    "game",
                    "--after",
                    "3000",
                ],
                None,
            ),
            Ok(Command::Run(expected))
        );
    }

    #[test]
    fn the_size_option_also_applies_to_a_normal_run() {
        let expected = RunOptions {
            size: Some(WindowSize {
                width: 1280,
                height: 720,
            }),
            ..run("game")
        };
        assert_eq!(
            parse(["game", "--size", "1280x720"], None),
            Ok(Command::Run(expected))
        );
    }

    #[test]
    fn a_capture_file_must_end_in_png() {
        for path in ["shot.jpg", "shot", "shot.png.txt", ".png"] {
            assert_eq!(
                parse(["game", "--capture", path], None),
                Err(CliError::NotPng(path.into())),
                "`{path}` should be refused"
            );
        }
    }

    #[test]
    fn after_needs_a_capture_and_a_whole_number_of_milliseconds() {
        assert_eq!(
            parse(["game", "--after", "500"], None),
            Err(CliError::AfterWithoutCapture)
        );
        for value in ["-1", "1.5", "abc", "600001", ""] {
            assert_eq!(
                parse(["game", "--capture", "a.png", "--after", value], None),
                Err(CliError::BadAfter(value.into())),
                "`{value}` should be refused"
            );
        }
        assert!(matches!(
            parse(["game", "--capture", "a.png", "--after", "600000"], None),
            Ok(Command::Run(_))
        ));
    }

    #[test]
    fn capture_options_need_values_and_may_appear_once() {
        assert_eq!(
            parse(["game", "--size"], None),
            Err(CliError::MissingValue("--size"))
        );
        assert_eq!(
            parse(["game", "--capture"], None),
            Err(CliError::MissingValue("--capture"))
        );
        assert_eq!(
            parse(["game", "--after"], None),
            Err(CliError::MissingValue("--after"))
        );
        assert_eq!(
            parse(["game", "--capture", "a.png", "--capture", "b.png"], None),
            Err(CliError::RepeatedOption("--capture"))
        );
        assert_eq!(
            parse(["game", "--size", "800x600", "--size=900x700"], None),
            Err(CliError::RepeatedOption("--size"))
        );
        assert_eq!(
            parse(
                ["game", "--capture", "a.png", "--after", "1", "--after=2"],
                None
            ),
            Err(CliError::RepeatedOption("--after"))
        );
    }

    #[test]
    fn a_capture_needs_a_game_folder_too() {
        assert_eq!(
            parse(["--capture", "shot.png"], None),
            Err(CliError::MissingFolder)
        );
    }

    #[test]
    fn window_sizes_are_read_with_either_case_of_x() {
        let size = |width, height| WindowSize { width, height };
        assert_eq!(WindowSize::parse("1600x900"), Ok(size(1600, 900)));
        assert_eq!(WindowSize::parse("1600X900"), Ok(size(1600, 900)));
        assert_eq!(WindowSize::parse("64x64"), Ok(size(64, 64)));
        assert_eq!(WindowSize::parse("7680x4320"), Ok(size(7680, 4320)));
    }

    #[test]
    fn window_sizes_outside_the_limits_are_refused() {
        for text in [
            "63x900",
            "1600x7681",
            "0x0",
            "100000x100",
            "1600",
            "1600x",
            "x900",
            "axb",
            "1600x900x1",
            "",
            " 1600x900",
            "1600 x 900",
            "-1600x900",
        ] {
            assert_eq!(
                WindowSize::parse(text),
                Err(CliError::BadSize(text.into())),
                "`{text}` should be refused"
            );
        }
    }
}
