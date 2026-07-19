//! Velvet CLI — `velvet` command entry point.

mod doctor;
mod document_cmd;
mod export_cmd;
mod launch_cmd;
mod loc_cmd;
mod menu_cmd;
mod narrative_cmd;
mod new_cmd;
mod pack_cmd;
mod play_cmd;
mod script_cmd;
mod story_cmd;
mod workspace_cmd;

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use velvet_app::prelude::*;
use velvet_core::{engine_version, RunMode};

use doctor::cmd_doctor;
use document_cmd::{cmd_document_patch, cmd_document_regions};
use export_cmd::cmd_export;
use loc_cmd::{
    cmd_loc_convert, cmd_loc_extract, cmd_loc_extract_story, cmd_loc_langs, cmd_loc_validate,
};
use narrative_cmd::{cmd_level_mutate, cmd_narrative_edit, cmd_narrative_graph};
use new_cmd::{cmd_init, cmd_new, cmd_project_info, cmd_template_install, cmd_template_list};
use pack_cmd::cmd_pack;
use play_cmd::{cmd_play_project_opts, cmd_play_story_product, cmd_recheck_replay};
use script_cmd::{cmd_script_check, cmd_script_fmt, cmd_script_lsp, cmd_script_run};
use story_cmd::{
    cmd_story_build, cmd_story_check, cmd_story_dump_ast, cmd_story_dump_lowered,
    cmd_story_extract_loc, cmd_story_format, cmd_story_run, cmd_story_studio_model,
};
use workspace_cmd::{cmd_assets, cmd_build, cmd_check, cmd_clean, cmd_fmt, cmd_inspect, cmd_test};

#[derive(Parser, Debug)]
#[command(
    name = "velvet",
    version,
    about = "Velvet Engine command-line interface",
    long_about = "Create, run, build, and inspect Velvet Engine projects."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show engine and environment information.
    Doctor,
    /// Print version details.
    Version,
    /// Run the engine (windowed by default; use --headless for CI).
    Run {
        /// Maximum frames (0 = until window closed; headless default 60).
        #[arg(long, default_value_t = 0)]
        frames: u64,
        /// Force headless runner (no OS window).
        #[arg(long)]
        headless: bool,
        /// Project directory (reserved for later phases).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Initialize a minimal velvet.project in the current directory.
    Init {
        /// Project name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Show project info when a velvet.project exists.
    #[command(name = "project")]
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// Velvet Script tools.
    Script {
        #[command(subcommand)]
        command: ScriptCommands,
    },
    /// Velvet Story tools (writer-friendly narrative language → VS2).
    Story {
        /// Diagnostic UI language: `es` | `en` | `ja` | `de` | `zh`.
        /// Overrides env `VELVET_STORY_LANG`. Default: Spanish (`es`).
        #[arg(long, global = true)]
        lang: Option<String>,
        #[command(subcommand)]
        command: StoryCommands,
    },
    /// Create a new project from a template.
    New {
        /// Project name.
        name: String,
        /// Template name.
        #[arg(long, default_value = "visual-novel")]
        template: String,
        /// Parent directory.
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
    /// Localization tools.
    Localization {
        #[command(subcommand)]
        command: LocCommands,
    },
    /// Pack assets and write manifest.
    Pack {
        /// Assets directory.
        #[arg(long, default_value = "assets")]
        assets: PathBuf,
        /// Output manifest path.
        #[arg(long, default_value = "asset-pack.json")]
        out: PathBuf,
        /// Exclude glob (repeatable).
        #[arg(long)]
        exclude: Vec<String>,
        /// Include-only glob (repeatable).
        #[arg(long)]
        include: Vec<String>,
    },
    /// Export desktop package (dry-run by default for safety).
    Export {
        /// Output directory.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Binary package name.
        #[arg(long, default_value = "hello-velvet")]
        binary: String,
        /// Assets directory.
        #[arg(long, default_value = "assets")]
        assets: PathBuf,
        /// Actually run cargo build.
        #[arg(long)]
        release: bool,
        /// Perform cargo build (off = dry-run).
        #[arg(long)]
        build: bool,
        /// Platform: host|windows-x64|linux-x64|macos-arm64|… or raw triple.
        #[arg(long)]
        platform: Option<String>,
        /// Dry-run manifests for multiple platforms.
        #[arg(long)]
        multi: bool,
        /// Exclude glob for assets (repeatable).
        #[arg(long)]
        exclude: Vec<String>,
    },
    /// Type-check the workspace (`cargo check --workspace`).
    Check {
        /// Workspace / project root.
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Release profile.
        #[arg(long)]
        release: bool,
    },
    /// Run workspace tests (`cargo test --workspace`).
    Test {
        /// Workspace root.
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Release profile.
        #[arg(long)]
        release: bool,
        /// Optional test name filter.
        filter: Option<String>,
    },
    /// Build workspace or a package.
    Build {
        /// Workspace root.
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Release profile.
        #[arg(long)]
        release: bool,
        /// Optional package name (`-p`).
        #[arg(long)]
        package: Option<String>,
    },
    /// Clean build artifacts (`cargo clean`).
    Clean {
        /// Workspace root.
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Format `.vel` scripts under a path (optional rustfmt).
    Fmt {
        /// File or directory.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Also run `cargo fmt --all`.
        #[arg(long)]
        rust: bool,
    },
    /// List / summarize assets (optionally write pack manifest).
    Assets {
        /// Assets directory.
        #[arg(default_value = "assets")]
        path: PathBuf,
        /// Optional pack manifest output.
        #[arg(long)]
        pack: Option<PathBuf>,
    },
    /// Inspect a project, script, or file.
    Inspect {
        /// Path to project dir, velvet.project, or .vel file.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Document region tools (visual/advanced round-trip).
    Document {
        #[command(subcommand)]
        command: DocumentCommands,
    },
    /// Project templates.
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
    /// Play a story or project entry via product VnSession (Say/Choice).
    Play {
        /// Path to `.vel` file OR project directory with velvet.project.
        path: PathBuf,
        /// Max steps (safety).
        #[arg(long, default_value_t = 256)]
        max_steps: u32,
        /// Choice index when multiple options (default 0).
        #[arg(long)]
        choice: Option<usize>,
        /// Attempt windowed host tick (falls back honestly if no display).
        #[arg(long, default_value_t = false)]
        windowed: bool,
        /// Language code (`en`, `es`, …) — loads `tl/<lang>/strings.json`.
        #[arg(long, default_value = "en")]
        lang: String,
    },
    /// Re-check script then re-play product path (author DX).
    RecheckReplay {
        /// Path to `.vel` file OR project directory.
        path: PathBuf,
        #[arg(long, default_value_t = 256)]
        max_steps: u32,
        #[arg(long)]
        choice: Option<usize>,
        #[arg(long, default_value = "en")]
        lang: String,
    },
    /// Minimal interactive launcher menu (1–7).
    Menu {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Auto-select launch (option 5) without stdin.
        #[arg(long, default_value_t = false)]
        auto: bool,
    },
    /// Unified author flow: project info → check → play → export zip.
    Launch {
        /// Project directory (with velvet.project).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Choice index for headless play.
        #[arg(long, default_value_t = 0)]
        choice: u32,
        /// Language for play.
        #[arg(long, default_value = "en")]
        lang: String,
        /// Export output directory.
        #[arg(long, default_value = "dist")]
        export_out: PathBuf,
        /// Host binary package name for export.
        #[arg(long, default_value = "hello-velvet")]
        binary: String,
        /// Actually cargo build for export.
        #[arg(long, default_value_t = true)]
        build: bool,
        /// Skip export step.
        #[arg(long, default_value_t = false)]
        no_export: bool,
    },
    /// Narrative block authoring tools.
    Narrative {
        #[command(subcommand)]
        command: NarrativeCommands,
    },
    /// Level document tools (RPG/Action).
    Level {
        #[command(subcommand)]
        command: LevelCommands,
    },
}

#[derive(Subcommand, Debug)]
enum NarrativeCommands {
    /// Append dialogue (and optional binary decision) to a scene in a `.vel` file.
    Edit {
        /// Path to story `.vel`.
        path: PathBuf,
        /// Scene name.
        #[arg(long)]
        scene: String,
        /// Optional speaker id.
        #[arg(long)]
        speaker: Option<String>,
        /// Dialogue text.
        #[arg(long)]
        text: String,
        /// Choice A label.
        #[arg(long)]
        choice_a: Option<String>,
        /// Jump target for A.
        #[arg(long)]
        jump_a: Option<String>,
        /// Choice B label.
        #[arg(long)]
        choice_b: Option<String>,
        /// Jump target for B.
        #[arg(long)]
        jump_b: Option<String>,
    },
    /// Print narrative graph + validation.
    Graph {
        /// Path to story `.vel`.
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum LevelCommands {
    /// Move an entity and paint a tile; save + reload assert.
    Mutate {
        /// Path to `.level.json`.
        path: PathBuf,
        /// Entity id.
        #[arg(long)]
        entity: String,
        /// X.
        #[arg(long)]
        x: f32,
        /// Y.
        #[arg(long)]
        y: f32,
    },
}

#[derive(Subcommand, Debug)]
enum TemplateCommands {
    /// List available templates.
    List,
    /// Install a template as a new project (same as `velvet new`).
    Install {
        /// Project name.
        name: String,
        /// Template id.
        #[arg(long, default_value = "visual-novel")]
        template: String,
        /// Parent directory.
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum DocumentCommands {
    /// List @visual / @advanced / @protected regions.
    Regions {
        /// Path to a .vel file.
        path: PathBuf,
    },
    /// Set a visual property without destroying advanced code.
    Patch {
        /// Path to a .vel file.
        path: PathBuf,
        /// Region id.
        region: String,
        /// Property key.
        key: String,
        /// Property value.
        value: String,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCommands {
    /// Print project summary.
    Info {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Run validation (modules, paths).
        #[arg(long)]
        validate: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ScriptCommands {
    /// Parse and compile a `.vel` file (diagnostics include file:line:column).
    Check {
        /// Path to script.
        path: PathBuf,
    },
    /// Compile and execute a script's main chunk.
    Run {
        /// Path to script.
        path: PathBuf,
        /// Optional exported function to call after main.
        #[arg(long)]
        call: Option<String>,
    },
    /// Format a script file in-place.
    Fmt {
        /// Path.
        path: PathBuf,
    },
    /// Print LSP-style analysis JSON.
    Lsp {
        /// Path.
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum StoryCommands {
    /// Validate a `.vstory` file (parse + semantics, no execute).
    Check {
        /// Path to `.vstory`.
        path: PathBuf,
    },
    /// Lower to Velvet Script 2 IR / OpVs2 unit.
    Build {
        /// Path to `.vstory`.
        path: PathBuf,
    },
    /// Execute via existing VS2 host (OpVs2), not a second VM.
    Run {
        /// Path to `.vstory`.
        path: PathBuf,
        /// Choice index when menus appear.
        #[arg(long, default_value_t = 0)]
        choice: usize,
    },
    /// Format a `.vstory` file in-place.
    Format {
        /// Path.
        path: PathBuf,
        /// Only check formatting.
        #[arg(long, default_value_t = false)]
        check: bool,
    },
    /// Dump narrative AST as JSON (developer tool).
    #[command(name = "dump-ast")]
    DumpAst { path: PathBuf },
    /// Dump lowered OpVs2 disassembly (developer tool).
    #[command(name = "dump-lowered")]
    DumpLowered { path: PathBuf },
    /// Emit Studio structured model JSON.
    #[command(name = "studio-model")]
    StudioModel { path: PathBuf },
    /// Extract localizable strings (stable msg ids).
    #[command(name = "extract-loc")]
    ExtractLoc {
        path: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum LocCommands {
    /// Extract strings from a .vel file.
    Extract {
        path: PathBuf,
        #[arg(long, default_value = "locale-source.json")]
        out: PathBuf,
        /// Output format: json | po | properties
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Extract story keys into `tl/en` (+ optional target lang scaffold).
    ExtractStory {
        /// Path to `.vel` story.
        path: PathBuf,
        /// Project root (default: parent of scripts/).
        #[arg(long)]
        project: Option<PathBuf>,
        /// Also scaffold this language (e.g. es).
        #[arg(long)]
        lang: Option<String>,
    },
    /// List available languages under project `tl/`.
    Langs {
        #[arg(default_value = ".")]
        project: PathBuf,
    },
    /// Validate target locale against source catalog.
    Validate { source: PathBuf, target: PathBuf },
    /// Convert catalog between json/po/properties by extension.
    Convert {
        input: PathBuf,
        out: PathBuf,
        #[arg(long, default_value = "en")]
        locale: String,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Doctor => cmd_doctor(),
        Commands::Version => {
            println!("Velvet Engine {}", engine_version());
            println!("CLI {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Run {
            frames,
            headless,
            path,
        } => cmd_run(frames, headless, path),
        Commands::Init { name } => cmd_init(name),
        Commands::Project {
            command: ProjectCommands::Info { path, validate },
        } => cmd_project_info(path, validate),
        Commands::Script {
            command: ScriptCommands::Check { path },
        } => cmd_script_check(path),
        Commands::Script {
            command: ScriptCommands::Run { path, call },
        } => cmd_script_run(path, call),
        Commands::Script {
            command: ScriptCommands::Fmt { path },
        } => cmd_script_fmt(path),
        Commands::Script {
            command: ScriptCommands::Lsp { path },
        } => cmd_script_lsp(path),
        Commands::Story { lang, command } => {
            velvet_story_lang::apply_locale_from_env();
            let loc = if let Some(l) = lang {
                let loc =
                    velvet_story_lang::DiagLocale::parse(&l).map_err(|e| anyhow::anyhow!(e))?;
                // Process default for tools that only call set_diag_locale,
                // plus thread-scoped effective locale for isolation-safe paths.
                velvet_story_lang::set_diag_locale(loc);
                Some(loc)
            } else {
                None
            };
            let run = move || match command {
                StoryCommands::Check { path } => cmd_story_check(path),
                StoryCommands::Build { path } => cmd_story_build(path),
                StoryCommands::Run { path, choice } => cmd_story_run(path, choice),
                StoryCommands::Format { path, check } => cmd_story_format(path, check),
                StoryCommands::DumpAst { path } => cmd_story_dump_ast(path),
                StoryCommands::DumpLowered { path } => cmd_story_dump_lowered(path),
                StoryCommands::StudioModel { path } => cmd_story_studio_model(path),
                StoryCommands::ExtractLoc { path, out } => cmd_story_extract_loc(path, out),
            };
            match loc {
                Some(l) => velvet_story_lang::with_diag_locale(l, run),
                None => run(),
            }
        }
        Commands::New {
            name,
            template,
            out,
        } => cmd_new(name, template, out),
        Commands::Localization {
            command: LocCommands::Extract { path, out, format },
        } => cmd_loc_extract(path, out, &format),
        Commands::Localization {
            command:
                LocCommands::ExtractStory {
                    path,
                    project,
                    lang,
                },
        } => cmd_loc_extract_story(path, project, lang),
        Commands::Localization {
            command: LocCommands::Langs { project },
        } => cmd_loc_langs(project),
        Commands::Localization {
            command: LocCommands::Validate { source, target },
        } => cmd_loc_validate(source, target),
        Commands::Localization {
            command: LocCommands::Convert { input, out, locale },
        } => cmd_loc_convert(input, out, &locale),
        Commands::Pack {
            assets,
            out,
            exclude,
            include,
        } => cmd_pack(assets, out, exclude, include),
        Commands::Export {
            out,
            binary,
            assets,
            release,
            build,
            platform,
            multi,
            exclude,
        } => cmd_export(
            out, binary, assets, release, build, platform, multi, exclude,
        ),
        Commands::Check { path, release } => cmd_check(path, release),
        Commands::Test {
            path,
            release,
            filter,
        } => cmd_test(path, release, filter),
        Commands::Build {
            path,
            release,
            package,
        } => cmd_build(path, release, package),
        Commands::Clean { path } => cmd_clean(path),
        Commands::Fmt { path, rust } => cmd_fmt(path, rust),
        Commands::Assets { path, pack } => cmd_assets(path, pack),
        Commands::Inspect { path } => cmd_inspect(path),
        Commands::Document {
            command: DocumentCommands::Regions { path },
        } => cmd_document_regions(path),
        Commands::Document {
            command:
                DocumentCommands::Patch {
                    path,
                    region,
                    key,
                    value,
                },
        } => cmd_document_patch(path, region, key, value),
        Commands::Template {
            command: TemplateCommands::List,
        } => cmd_template_list(),
        Commands::Template {
            command:
                TemplateCommands::Install {
                    name,
                    template,
                    out,
                },
        } => cmd_template_install(name, template, out),
        Commands::Play {
            path,
            max_steps,
            choice,
            windowed,
            lang,
        } => {
            if path.is_dir() || path.join("velvet.project").exists() {
                cmd_play_project_opts(path, max_steps, choice, windowed, lang)
            } else {
                cmd_play_story_product(path, max_steps, choice, windowed, lang)
            }
        }
        Commands::RecheckReplay {
            path,
            max_steps,
            choice,
            lang,
        } => cmd_recheck_replay(path, max_steps, choice, lang),
        Commands::Menu { path, auto } => crate::menu_cmd::cmd_menu(path, auto),
        Commands::Launch {
            path,
            choice,
            lang,
            export_out,
            binary,
            build,
            no_export,
        } => crate::launch_cmd::cmd_launch(
            path,
            choice as usize,
            lang,
            export_out,
            binary,
            build,
            no_export,
        ),
        Commands::Narrative {
            command:
                NarrativeCommands::Edit {
                    path,
                    scene,
                    speaker,
                    text,
                    choice_a,
                    jump_a,
                    choice_b,
                    jump_b,
                },
        } => cmd_narrative_edit(
            path, scene, speaker, text, choice_a, jump_a, choice_b, jump_b,
        ),
        Commands::Narrative {
            command: NarrativeCommands::Graph { path },
        } => cmd_narrative_graph(path),
        Commands::Level {
            command: LevelCommands::Mutate { path, entity, x, y },
        } => cmd_level_mutate(path, entity, x, y),
    }
}

fn cmd_run(frames: u64, headless: bool, path: Option<PathBuf>) -> Result<()> {
    velvet_core::init_tracing_default("velvet=info,info");
    if let Some(p) = &path {
        info!(path = %p.display(), "project path noted (loading not yet implemented)");
    }
    let mut config = EngineConfig {
        mode: RunMode::Development,
        name: "velvet run".into(),
        ..Default::default()
    };
    config.window.title = "Velvet Engine".into();

    let mut app = App::with_config(config);
    if headless {
        let frames = if frames == 0 { 60 } else { frames };
        app.set_runner(HeadlessRunner {
            max_frames: Some(frames),
            delta_secs: 1.0 / 60.0,
        });
        let code = app.run();
        if code.0 != 0 {
            bail!("run failed with exit code {}", code.0);
        }
        info!(frames, "headless run completed");
        println!("ran {frames} frame(s) successfully (headless)");
    } else {
        #[cfg(feature = "window")]
        {
            let max_frames = if frames == 0 { None } else { Some(frames) };
            app.set_runner(WindowRunner { max_frames });
            let code = app.run();
            if code.0 != 0 {
                bail!("run failed with exit code {}", code.0);
            }
            println!("window session ended");
        }
        #[cfg(not(feature = "window"))]
        {
            bail!("window feature disabled; rebuild with --features window or pass --headless");
        }
    }
    Ok(())
}
