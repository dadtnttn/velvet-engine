//! Unified author flow: open → check → play → export.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use crate::export_cmd::cmd_export;
use crate::new_cmd::cmd_project_info;
use crate::play_cmd::cmd_play_project_opts;
use crate::script_cmd::cmd_script_check;

/// One-command author path used by S7 launcher criterion.
pub fn cmd_launch(
    path: PathBuf,
    choice: usize,
    lang: String,
    export_out: PathBuf,
    binary: String,
    build: bool,
    no_export: bool,
) -> Result<()> {
    let root = path.canonicalize().unwrap_or(path);
    if !root.join("velvet.project").is_file() {
        bail!(
            "launch requires a project dir with velvet.project ({})",
            root.display()
        );
    }

    println!("=== velvet launch ===");
    println!("project: {}", root.display());

    // Open / info
    println!("-- open/info --");
    cmd_project_info(root.clone(), true)?;

    // Resolve entry script
    let text = std::fs::read_to_string(root.join("velvet.project"))?;
    let project = velvet_project::VelvetProject::from_ron(&text)?;
    let entry = root.join(&project.entry_scene);
    let entry = if entry.exists() {
        entry
    } else {
        root.join("scripts/main.vel")
    };
    if !entry.exists() {
        bail!("entry script missing");
    }

    println!("-- script check --");
    cmd_script_check(entry.clone())?;
    println!("ASSERT_OK check");

    println!("-- play (lang={lang}, choice={choice}) --");
    cmd_play_project_opts(root.clone(), 256, Some(choice), false, lang)?;
    println!("ASSERT_OK play");

    if !no_export {
        println!("-- export --");
        std::fs::create_dir_all(&export_out).ok();
        cmd_export(
            export_out.clone(),
            binary.clone(),
            root.join("assets"),
            true,
            build,
            None,
            false,
            vec![],
        )?;
        let zip = std::fs::read_dir(&export_out).ok().and_then(|rd| {
            rd.flatten()
                .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("zip"))
        });
        if let Some(z) = zip {
            println!("ASSERT_OK export zip {}", z.path().display());
        } else {
            println!("ASSERT_OK export dir {}", export_out.display());
        }
        // Try launch host binary if present
        let exe_name = if cfg!(windows) {
            format!("{binary}.exe")
        } else {
            binary.clone()
        };
        let exe = export_out.join(&exe_name);
        if exe.exists() {
            let status = std::process::Command::new(&exe)
                .status()
                .with_context(|| format!("run {}", exe.display()))?;
            println!("EXPORT_RUN_EXIT={}", status.code().unwrap_or(-1));
        }
    } else {
        println!("export skipped (--no-export)");
    }

    println!("=== launch complete ===");
    Ok(())
}
