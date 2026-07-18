//! Minimal interactive launcher menu (text UI, Ren'Py-launcher-like surface).

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;

use crate::export_cmd::cmd_export;
use crate::launch_cmd::cmd_launch;
use crate::new_cmd::cmd_project_info;
use crate::play_cmd::cmd_play_project_opts;
use crate::script_cmd::cmd_script_check;

/// Simple numbered menu for a project directory.
pub fn cmd_menu(path: PathBuf, non_interactive: bool) -> Result<()> {
    let root = path.canonicalize().unwrap_or(path);
    println!("Velvet Launcher — {}", root.display());
    println!("  [1] Project info");
    println!("  [2] Script check");
    println!("  [3] Play (EN)");
    println!("  [4] Play (ES if tl/es)");
    println!("  [5] Launch (check+play+export)");
    println!("  [6] Export host zip");
    println!("  [7] Export web (Node player)");
    println!("  [0] Quit");

    let choice = if non_interactive {
        println!("(non-interactive: auto-select 5 launch)");
        5
    } else {
        print!("> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        line.trim().parse().unwrap_or(0)
    };

    match choice {
        1 => cmd_project_info(root, true),
        2 => {
            let entry = resolve_entry(&root)?;
            cmd_script_check(entry)
        }
        3 => cmd_play_project_opts(root, 128, Some(0), false, "en".into()),
        4 => cmd_play_project_opts(root, 128, Some(0), false, "es".into()),
        5 => cmd_launch(
            root,
            0,
            "en".into(),
            PathBuf::from("dist"),
            "hello-velvet".into(),
            true,
            false,
        ),
        6 => cmd_export(
            PathBuf::from("dist"),
            "hello-velvet".into(),
            root.join("assets"),
            true,
            true,
            None,
            false,
            vec![],
        ),
        7 => cmd_export(
            root.join("web_dist"),
            root.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("velvet")
                .into(),
            root.join("assets"),
            false,
            false,
            Some("web".into()),
            false,
            vec![],
        ),
        _ => {
            println!("bye");
            Ok(())
        }
    }
}

fn resolve_entry(root: &std::path::Path) -> Result<PathBuf> {
    let text = std::fs::read_to_string(root.join("velvet.project"))?;
    let project = velvet_project::VelvetProject::from_ron(&text)?;
    let entry = root.join(&project.entry_scene);
    if entry.exists() {
        Ok(entry)
    } else {
        Ok(root.join("scripts/main.vel"))
    }
}
