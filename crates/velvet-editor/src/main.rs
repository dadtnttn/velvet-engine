//! Velvet Studio — docking GUI + CLI project tooling.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use velvet_editor::asset_panel;
use velvet_editor::commands;
use velvet_editor::document_edit;
use velvet_editor::gui::{run_studio_gui, StudioGuiConfig};
use velvet_editor::inspector;
use velvet_editor::studio::StudioApp;

#[derive(Parser, Debug)]
#[command(name = "velvet-studio", about = "Velvet Studio")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open interactive studio shell on a project path.
    Open {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Launch docking GUI (headless-ready by default for CI).
    Gui {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Document to open on the visual canvas.
        #[arg(long)]
        document: Option<PathBuf>,
        /// Do not attempt an OS window.
        #[arg(long, default_value_t = true)]
        headless: bool,
        /// Attempt a brief OS window (overrides headless).
        #[arg(long, default_value_t = false)]
        window: bool,
        /// Exit after init / demo drag.
        #[arg(long, default_value_t = true)]
        once: bool,
        /// Demo drag region id.
        #[arg(long)]
        drag_region: Option<String>,
        /// Demo drag dx (percent points).
        #[arg(long, default_value_t = 0.0)]
        dx: f32,
        /// Demo drag dy.
        #[arg(long, default_value_t = 0.0)]
        dy: f32,
        /// Save document after demo drag.
        #[arg(long, default_value_t = false)]
        save: bool,
        /// Write ready log path.
        #[arg(long)]
        ready_log: Option<PathBuf>,
    },
    /// Drag a visual region on a .vel file (same API as GUI canvas).
    Drag {
        /// Path to a .vel file.
        file: PathBuf,
        /// Region id.
        region: String,
        /// Delta X (percent or pixels matching stored unit).
        dx: f32,
        /// Delta Y.
        dy: f32,
    },
    /// Create project from template name.
    New {
        name: String,
        #[arg(long, default_value = "visual-novel")]
        template: String,
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
    /// Print project hierarchy (script symbols + assets).
    Hierarchy {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Run diagnostics on all .vel files.
    Check {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List assets with optional kind filter.
    Assets {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        filter: Option<String>,
    },
    /// Inspect project or a file.
    Inspect {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Create a scene stub under scripts/ or scenes/.
    NewScene {
        #[arg(default_value = ".")]
        path: PathBuf,
        name: String,
    },
    /// List @visual/@advanced/@protected regions in a document.
    Regions {
        file: PathBuf,
    },
    /// Set a visual property without destroying advanced/protected regions.
    PatchVisual {
        file: PathBuf,
        region: String,
        key: String,
        value: String,
    },
    /// Studio home menu.
    Home,
    /// Cellular brush lab (author tools for velvet-cellular).
    Cellular {
        /// Steps to simulate after painting.
        #[arg(long, default_value_t = 60)]
        steps: u32,
        /// Brush preset name (Sand, Water, Blood, …).
        #[arg(long, default_value = "Sand")]
        preset: String,
        /// Stamp X.
        #[arg(long, default_value_t = 0)]
        x: i32,
        /// Stamp Y.
        #[arg(long, default_value_t = 20)]
        y: i32,
        /// Spawn a demo slime enemy.
        #[arg(long, default_value_t = true)]
        enemy: bool,
        /// Generate a small cave field.
        #[arg(long, default_value_t = false)]
        caves: bool,
        /// Optional save path for world JSON.
        #[arg(long)]
        save: Option<PathBuf>,
    },
    /// Unified flow: check → play product path → optional export.
    Launch {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value_t = 0)]
        choice: usize,
        #[arg(long, default_value = "en")]
        lang: String,
        #[arg(long, default_value = "dist")]
        export_out: PathBuf,
        #[arg(long, default_value_t = false)]
        no_export: bool,
    },
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("velvet_studio=info,info");
    let args = Args::parse();
    match args.command {
        Commands::Open { path } => StudioApp::open(path)?.run_shell(),
        Commands::Gui {
            path,
            document,
            headless,
            window,
            once,
            drag_region,
            dx,
            dy,
            save,
            ready_log,
        } => {
            let status = run_studio_gui(StudioGuiConfig {
                root: path,
                document,
                headless: if window { false } else { headless },
                once,
                demo_drag_region: drag_region,
                demo_dx: dx,
                demo_dy: dy,
                ready_log,
                save_after_drag: save,
            })?;
            if let Some(err) = status.display_error {
                eprintln!("display note: {err}");
            }
            Ok(())
        }
        Commands::Drag {
            file,
            region,
            dx,
            dy,
        } => {
            let pos = document_edit::drag_region_on_disk(&file, &region, dx, dy)?;
            println!("dragged {region} -> position: {pos} in {}", file.display());
            Ok(())
        }
        Commands::New {
            name,
            template,
            out,
        } => StudioApp::create_project(&name, &template, out),
        Commands::Hierarchy { path } => {
            let mut app = StudioApp::open(path)?;
            app.print_hierarchy();
            Ok(())
        }
        Commands::Check { path } => {
            let mut app = StudioApp::open(path)?;
            let n = app.check_all()?;
            println!("checked {n} script(s)");
            Ok(())
        }
        Commands::Assets { path, filter } => {
            let app = StudioApp::open(path)?;
            let args: Vec<&str> = filter
                .as_deref()
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            let f = asset_panel::parse_filter_args(&args);
            let n = asset_panel::print_assets(&app.root, &f)?;
            println!("{n} asset(s) listed");
            Ok(())
        }
        Commands::Inspect { path, file } => {
            let app = StudioApp::open(path)?;
            let selection = match file {
                Some(f) => inspector::Selection::File(app.root.join(f)),
                None => inspector::Selection::Project,
            };
            let report = inspector::inspect(&app.root, &selection)?;
            inspector::print_report(&report);
            Ok(())
        }
        Commands::NewScene { path, name } => {
            let app = StudioApp::open(path)?;
            let p = commands::create_scene_stub(&app.root, &name)?;
            println!("created {}", p.display());
            Ok(())
        }
        Commands::Regions { file } => {
            document_edit::require_file(&file)?;
            for (kind, id) in document_edit::list_regions(&file)? {
                println!("{kind}\t{id}");
            }
            Ok(())
        }
        Commands::PatchVisual {
            file,
            region,
            key,
            value,
        } => {
            document_edit::require_file(&file)?;
            let _ = document_edit::set_visual_property(&file, &region, &key, &value)?;
            println!("patched {region}.{key} in {}", file.display());
            Ok(())
        }
        Commands::Home => {
            println!("Velvet Studio");
            println!("-------------");
            println!("  gui <path> [--document f] [--drag-region id --dx N --dy N]");
            println!("  drag <file.vel> <region> <dx> <dy>   Canvas drag (preserves advanced)");
            println!("  cellular [--preset Sand] [--caves] [--enemy] [--steps N]");
            println!("  new / open / hierarchy / check / assets / inspect");
            println!("  regions / patch-visual / launch");
            println!("Docking panels: hierarchy, assets, canvas, inspector, scripts, console");
            Ok(())
        }
        Commands::Cellular {
            steps,
            preset,
            x,
            y,
            enemy,
            caves,
            save,
        } => studio_cellular(steps, preset, x, y, enemy, caves, save),
        Commands::Launch {
            path,
            choice,
            lang,
            export_out,
            no_export,
        } => studio_launch(path, choice, lang, export_out, no_export),
    }
}

fn studio_cellular(
    steps: u32,
    preset: String,
    x: i32,
    y: i32,
    enemy: bool,
    caves: bool,
    save: Option<PathBuf>,
) -> Result<()> {
    use velvet_cellular::prelude::*;

    println!("=== velvet-studio cellular (brush author lab) ===");
    let mut session = CellularSession::with_builtins(WorldConfig::default());
    session.seed_demo_platform();
    if caves {
        session.gen_caves(CaveOptions {
            x0: -48,
            y0: 0,
            x1: 48,
            y1: 40,
            solid: session.mat("stone"),
            border: session.mat("bedrock"),
            open_threshold: 0.5,
            scale: 0.1,
            seed: 9,
            border_thickness: 2,
        });
        println!("caves generated");
    }
    if !session.select_preset(&preset) {
        session.brush_material("sand");
        println!("preset '{preset}' not found — using sand");
    } else {
        println!("brush preset: {preset}");
    }
    let painted = session.brush_down(x, y);
    session.brush_drag(x + 8, y);
    session.brush_up();
    println!("brush stroke cells≈{painted}+");
    if enemy {
        if let Some(id) = session.spawn_enemy("slime", x as f32, (y + 5) as f32) {
            println!("spawned slime id={id}");
        }
    }
    session.splatter(x + 4, y + 2, 3);
    let pb = session.particle_burst(x as f32, (y + 8) as f32, "sand", 24);
    let _ = session.cast_spell("spark_bolt", x as f32 + 2.0, y as f32 + 6.0);
    session.step_n(steps);
    let buf = session.render(-40, -8, 80, 56);
    println!(
        "tick={} occupied={} enemies={} particles={} particle_spawned={} opaque_px={}",
        session.world.tick,
        session.world.occupied_cells(),
        session.enemies.alive_count(),
        session.particle_count(),
        pb,
        opaque_pixel_count(&buf)
    );
    println!(
        "materials={}",
        session
            .world
            .materials
            .keys()
            .take(12)
            .collect::<Vec<_>>()
            .join(",")
    );
    if let Some(path) = save {
        session.save(&path)?;
        println!("saved {}", path.display());
    }
    println!("ASSERT_OK cellular_brush");
    Ok(())
}

fn studio_launch(
    path: PathBuf,
    choice: usize,
    lang: String,
    export_out: PathBuf,
    no_export: bool,
) -> Result<()> {
    use velvet_story::prelude::*;

    let app = StudioApp::open(&path)?;
    println!("=== velvet-studio launch ===");
    println!("project: {}", app.root.display());
    let n = {
        let mut a = StudioApp::open(&path)?;
        a.check_all()?
    };
    println!("ASSERT_OK check scripts={n}");

    let proj_text = std::fs::read_to_string(app.root.join("velvet.project"))?;
    let project = velvet_project::VelvetProject::from_ron(&proj_text)?;
    let entry = app.root.join(&project.entry_scene);
    let entry = if entry.exists() {
        entry
    } else {
        app.root.join("scripts/main.vel")
    };
    let mut session = open_session_from_file(&entry, "studio-launch", Some(app.root.join("saves")))
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .with_project_root(app.root.clone());
    if lang != "en" {
        session
            .set_language(&lang)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    let ending = session.run_to_ending(256, choice);
    println!(
        "play ending={:?} text={}",
        ending,
        session.player().current_text()
    );
    println!("ASSERT_OK play");

    if !no_export {
        let report = velvet_build::export_desktop(&velvet_build::ExportOptions {
            out_dir: export_out.clone(),
            binary_name: "hello-velvet".into(),
            assets_dir: app.root.join("assets"),
            release: true,
            target: None,
            project_name: project.name.clone(),
            dry_run: false,
            platform: "host".into(),
            exclude: vec![],
            include: vec![],
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("export {}", report.out_dir.display());
        if let Some(a) = report.archive_path {
            println!("ASSERT_OK export zip {}", a.display());
        }
    }
    println!("=== studio launch complete ===");
    Ok(())
}
