//! Command-line arguments of the `velvet-desktop` binary.
//!
//! Usage: `velvet-desktop <game-folder> [--dev <url>] [--devtools]`. With no arguments, the program
//! runs the game in `game/` next to its own executable, when that folder has a manifest (see
//! [`packaged_game`]). The parser takes the arguments without the program name and the program's
//! folder, so it can be tested without a process.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::manifest::MANIFEST_FILE;
use crate::origin::origin_of;

/// The folder next to the program that holds a packaged game.
pub const PACKAGED_GAME_DIR: &str = "game";

/// The text that `--help` prints.
pub const USAGE: &str = "\
Usage: velvet-desktop <game-folder> [--dev <url>] [--devtools]

  <game-folder>   folder with velvet-game.json and the game files
  --dev <url>     load a dev server (for example http://localhost:5173) instead of the files
  --devtools      open the web inspector (debug builds always have it)
  -h, --help      show this help

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
            }),
            None => Command::Help,
        });
    }

    let mut args = args.into_iter();
    let mut game_dir: Option<PathBuf> = None;
    let mut dev_url: Option<String> = None;
    let mut devtools = false;

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
        } else if arg.starts_with('-') {
            return Err(CliError::UnknownOption(arg));
        } else if game_dir.is_some() {
            return Err(CliError::ExtraArgument(arg));
        } else {
            game_dir = Some(PathBuf::from(arg));
        }
    }

    let game_dir = game_dir.ok_or(CliError::MissingFolder)?;
    Ok(Command::Run(RunOptions {
        game_dir,
        dev_url,
        devtools,
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
        for option in ["<game-folder>", "--dev <url>", "--devtools", "--help"] {
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
}
