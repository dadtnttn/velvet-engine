//! Command-line arguments of the `velvet-desktop` binary.
//!
//! Usage: `velvet-desktop <game-folder> [--dev <url>] [--devtools]`. The parser takes the
//! arguments without the program name, so it can be tested without a process.

use std::path::PathBuf;

use thiserror::Error;

use crate::app::origin_of;

/// The text that `--help` prints.
pub const USAGE: &str = "\
Usage: velvet-desktop <game-folder> [--dev <url>] [--devtools]

  <game-folder>   folder with velvet-game.json and the game files
  --dev <url>     load a dev server (for example http://localhost:5173) instead of the files
  --devtools      open the web inspector (debug builds always have it)
  -h, --help      show this help";

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
pub fn parse<I, S>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
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
        RunOptions {
            game_dir: PathBuf::from(game),
            dev_url: None,
            devtools: false,
        }
    }

    #[test]
    fn a_folder_alone_runs_the_game() {
        assert_eq!(parse(["game"]), Ok(Command::Run(run("game"))));
    }

    #[test]
    fn devtools_can_be_added_after_the_folder() {
        let expected = RunOptions {
            devtools: true,
            ..run("game")
        };
        assert_eq!(parse(["game", "--devtools"]), Ok(Command::Run(expected)));
    }

    #[test]
    fn a_dev_server_can_be_given_in_both_forms() {
        let expected = RunOptions {
            dev_url: Some("http://localhost:5173".into()),
            ..run("game")
        };
        assert_eq!(
            parse(["--dev", "http://localhost:5173", "game"]),
            Ok(Command::Run(expected.clone()))
        );
        assert_eq!(
            parse(["game", "--dev=http://localhost:5173"]),
            Ok(Command::Run(expected))
        );
    }

    #[test]
    fn https_dev_servers_are_accepted() {
        assert!(matches!(
            parse(["game", "--dev", "https://dev.example:8443/app/"]),
            Ok(Command::Run(RunOptions {
                dev_url: Some(_),
                ..
            }))
        ));
    }

    #[test]
    fn help_is_recognised_anywhere_before_an_error() {
        assert_eq!(parse(["--help"]), Ok(Command::Help));
        assert_eq!(parse(["game", "-h"]), Ok(Command::Help));
    }

    #[test]
    fn a_missing_folder_is_reported() {
        assert_eq!(parse(Vec::<String>::new()), Err(CliError::MissingFolder));
        assert_eq!(parse(["--devtools"]), Err(CliError::MissingFolder));
    }

    #[test]
    fn extra_folders_and_unknown_options_are_reported() {
        assert_eq!(parse(["a", "b"]), Err(CliError::ExtraArgument("b".into())));
        assert_eq!(
            parse(["--nope", "game"]),
            Err(CliError::UnknownOption("--nope".into()))
        );
    }

    #[test]
    fn dev_option_problems_are_reported() {
        assert_eq!(parse(["game", "--dev"]), Err(CliError::MissingDevUrl));
        assert_eq!(
            parse(["--dev", "ftp://x", "game"]),
            Err(CliError::BadDevUrl("ftp://x".into()))
        );
        assert_eq!(
            parse(["--dev", "http://", "game"]),
            Err(CliError::BadDevUrl("http://".into()))
        );
        assert_eq!(
            parse(["--dev", "localhost:5173", "game"]),
            Err(CliError::BadDevUrl("localhost:5173".into()))
        );
        assert_eq!(
            parse(["--dev", "http://a:1", "--dev", "http://b:2", "game"]),
            Err(CliError::RepeatedDev)
        );
    }

    #[test]
    fn the_usage_text_lists_every_option() {
        for option in ["<game-folder>", "--dev <url>", "--devtools", "--help"] {
            assert!(USAGE.contains(option), "usage lists {option}");
        }
    }
}
