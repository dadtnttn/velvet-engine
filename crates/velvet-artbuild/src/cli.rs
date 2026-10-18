//! The command surface of `velvet art`: `build` writes the outputs, `check` only reports. The
//! `velvet` CLI wires [`ArtArgs`] into its subcommands and calls [`run`].

use std::path::PathBuf;

use crate::output_format::OutputFormat;
use crate::pipeline::{self, ArtBuildOptions};

/// The arguments of `velvet art`.
#[derive(Debug, Clone, clap::Args)]
pub struct ArtArgs {
    /// What to do: `build` writes the outputs, `check` reports the problems and writes nothing.
    #[command(subcommand)]
    pub command: ArtCommand,
    /// The game folder, which holds `art/` (the manifest and the outputs). Defaults to the current folder.
    #[arg(long, global = true, default_value = ".")]
    pub game: PathBuf,
    /// The folder of source pictures. Defaults to `art-src/` in the game folder.
    #[arg(long, global = true)]
    pub src: Option<PathBuf>,
    /// The formats to write, separated by commas: avif, webp or png. Overrides the build config.
    #[arg(long, global = true, value_delimiter = ',', value_enum)]
    pub formats: Option<Vec<OutputFormat>>,
    /// Rebuild every output, even the ones that are up to date. Ignored by `check`.
    #[arg(long, global = true)]
    pub force: bool,
    /// Print the report as JSON instead of text.
    #[arg(long, global = true)]
    pub json: bool,
}

/// The subcommands of `velvet art`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::Subcommand)]
pub enum ArtCommand {
    /// Write the outputs of every source that is new or changed, then report what happened.
    Build,
    /// Report the missing sources, orphans, aspect warnings, heavy files and skipped outputs. Writes nothing.
    Check,
}

impl ArtArgs {
    /// The options of the build or the check that these arguments ask for.
    pub fn options(&self) -> ArtBuildOptions {
        ArtBuildOptions {
            game: self.game.clone(),
            src: self.src.clone(),
            formats: self.formats.clone(),
            force: self.force,
            ffmpeg: None,
        }
    }
}

/// Runs `velvet art`: performs the build or the check, prints its report to standard output (as text,
/// or as JSON with `--json`), and returns the exit code.
///
/// The exit code is 0 when all is well. A build exits with 1 when a source or an output failed. A check
/// exits with 1 when it found any problem: a missing source, an orphan, an aspect warning, a heavy
/// file, a skipped output or a failure.
///
/// # Errors
/// Returns an error when the manifest or the build config is refused, or when a folder cannot be read.
pub fn run(args: &ArtArgs) -> anyhow::Result<i32> {
    let options = args.options();
    let report = match args.command {
        ArtCommand::Build => pipeline::build(&options)?,
        ArtCommand::Check => pipeline::check(&options)?,
    };
    if args.json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.to_text());
    }
    let failed = match args.command {
        ArtCommand::Build => report.has_failures(),
        ArtCommand::Check => report.has_problems(),
    };
    Ok(if failed { 1 } else { 0 })
}
