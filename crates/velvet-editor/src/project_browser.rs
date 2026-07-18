//! Project file browsing and template scaffolding.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

/// List files under project with extension filter.
pub fn list_files(root: &Path, ext: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some(ext)
        {
            out.push(entry.path().to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}

/// Create a project directory from a built-in template.
pub fn scaffold_project(name: &str, template: &str, parent: &Path) -> Result<PathBuf> {
    let dir = parent.join(name);
    if dir.exists() {
        bail!("path already exists: {}", dir.display());
    }
    fs::create_dir_all(dir.join("assets"))?;
    fs::create_dir_all(dir.join("scenes"))?;
    fs::create_dir_all(dir.join("scripts"))?;

    let known = [
        "visual-novel",
        "narrative-adventure",
        "top-down-rpg",
        "top-down-action",
    ];
    if !known.contains(&template) {
        bail!(
            "unknown template: {template} (expected one of {})",
            known.join(", ")
        );
    }
    let project = velvet_project::project_ron_for_template(name, template);
    fs::write(dir.join("velvet.project"), project)?;

    // Prefer copying from workspace templates/ when available.
    if let Some(tpl_root) = find_workspace_template(template) {
        let script_src = tpl_root.join("scripts/main.vel");
        let story_src = tpl_root.join("story/main.vel");
        if script_src.exists() {
            fs::copy(&script_src, dir.join("scripts/main.vel"))?;
        } else if story_src.exists() {
            fs::copy(&story_src, dir.join("scripts/main.vel"))?;
        } else {
            fs::write(dir.join("scripts/main.vel"), fallback_script(template))?;
        }
        let readme_src = tpl_root.join("README.md");
        if readme_src.exists() {
            let mut body = fs::read_to_string(readme_src)?;
            body = body.replace("{{name}}", name);
            fs::write(dir.join("README.md"), body)?;
        } else {
            fs::write(
                dir.join("README.md"),
                format!("# {name}\n\nCreated with Velvet Studio template `{template}`.\n"),
            )?;
        }
    } else {
        fs::write(dir.join("scripts/main.vel"), fallback_script(template))?;
        fs::write(
            dir.join("README.md"),
            format!("# {name}\n\nCreated with Velvet Studio template `{template}`.\n"),
        )?;
    }
    Ok(dir)
}

fn fallback_script(template: &str) -> &'static str {
    match template {
        "narrative-adventure" => {
            r##"character guide { name: "Guide" color: "#4fc3ff" }
state { flags: int = 0 }
scene main {
    guide "You stand at a fork in the road."
    choice {
        "Take the forest path" { jump forest }
        "Head to the village" { jump village }
    }
}
scene forest {
    guide "Trees close in. Something watches."
}
scene village {
    guide "Smoke rises from quiet chimneys."
}
"##
        }
        "top-down-rpg" => {
            r#"character npc { name: "Villager" }
scene talk {
    npc "The dungeon door is sealed."
}
"#
        }
        "top-down-action" => {
            r#"function on_level_start() {
    return 1
}
"#
        }
        _ => {
            r##"character hero { name: "Hero" color: "#ff4f8b" }
state { trust: int = 0 }
scene main {
    background "assets/bg.png"
    hero "Welcome to your new Velvet project."
    choice {
        "Continue" { jump end }
    }
}
scene end {
    "The beginning."
}
"##
        }
    }
}

/// Walk up from CWD for templates/<name>.
fn find_workspace_template(template: &str) -> Option<PathBuf> {
    let mut cur = std::env::current_dir().ok()?;
    for _ in 0..8 {
        let candidate = cur.join("templates").join(template);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

/// Read velvet.project if present.
pub fn load_project_info(root: &Path) -> Result<Option<velvet_project::VelvetProject>> {
    let p = root.join("velvet.project");
    if !p.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    Ok(Some(velvet_project::VelvetProject::from_ron(&text)?))
}
