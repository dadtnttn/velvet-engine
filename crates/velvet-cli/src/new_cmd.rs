//! Project init / new / project info commands.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

/// Initialize a minimal velvet.project in the current directory.
pub fn cmd_init(name: Option<String>) -> Result<()> {
    let name = name.unwrap_or_else(|| "velvet-game".into());
    let content = velvet_project::default_project_ron(&name);
    let path = PathBuf::from("velvet.project");
    if path.exists() {
        bail!("velvet.project already exists");
    }
    std::fs::write(&path, content).context("write velvet.project")?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Print project summary (and validation).
pub fn cmd_project_info(path: PathBuf, validate: bool) -> Result<()> {
    let file = path.join("velvet.project");
    let (project, root_for_validate) = if file.exists() {
        let text = std::fs::read_to_string(&file)?;
        let project = velvet_project::VelvetProject::from_ron(&text)?;
        (project, Some(path))
    } else if path.extension().and_then(|s| s.to_str()) == Some("project") {
        let text = std::fs::read_to_string(&path)?;
        let project = velvet_project::VelvetProject::from_ron(&text)?;
        (project, path.parent().map(|p| p.to_path_buf()))
    } else {
        bail!("no velvet.project under {}", path.display());
    };

    print_project(&project);

    if validate {
        let report = if let Some(root) = root_for_validate {
            project.validate_at(root)
        } else {
            project.validate()
        };
        println!(
            "validation : {} error(s), {} warning(s)",
            report.error_count(),
            report.warning_count()
        );
        for issue in &report.issues {
            let sev = match issue.severity {
                velvet_project::ValidationSeverity::Error => "error",
                velvet_project::ValidationSeverity::Warning => "warn",
                velvet_project::ValidationSeverity::Info => "info",
            };
            println!("  [{sev}] {}: {}", issue.code, issue.message);
        }
        if !report.resolved_modules.is_empty() {
            println!("resolved   : {}", report.resolved_modules.join(" → "));
        }
        if !report.is_ok() {
            bail!("project validation failed");
        }
    }
    Ok(())
}

fn print_project(p: &velvet_project::VelvetProject) {
    println!("name       : {}", p.name);
    println!("identifier : {}", p.identifier);
    println!("version    : {}", p.version);
    println!("modules    : {}", p.modules.join(", "));
    println!("entry      : {}", p.entry_scene);
    println!("assets     : {}", p.assets_dir);
}

/// Create a new project from a template.
pub fn cmd_new(name: String, template: String, out: PathBuf) -> Result<()> {
    let dir = out.join(&name);
    if dir.exists() {
        bail!("already exists: {}", dir.display());
    }
    std::fs::create_dir_all(dir.join("assets"))?;
    std::fs::create_dir_all(dir.join("scripts"))?;
    std::fs::create_dir_all(dir.join("scenes"))?;

    let project_ron = velvet_project::project_ron_for_template(&name, &template);
    std::fs::write(dir.join("velvet.project"), project_ron)?;

    let script = template_main_script(&template);
    std::fs::write(dir.join("scripts/main.vel"), script)?;
    if template == "visual-novel" || template == "narrative-adventure" {
        std::fs::write(
            dir.join("scripts/main_menu.vel"),
            template_main_menu_script(),
        )?;
    }
    if template == "top-down-rpg" {
        let level = velvet_document::LevelDocument::top_down_scaffold("town", 16, 12);
        if let Ok(json) = level.to_json() {
            std::fs::write(dir.join("scenes/town.level.json"), json)?;
        }
    }
    if template == "top-down-action" {
        let level = velvet_document::LevelDocument::action_scaffold("warehouse");
        if let Ok(json) = level.to_json() {
            std::fs::write(dir.join("scenes/warehouse.level.json"), json)?;
        }
    }
    std::fs::write(
        dir.join("README.md"),
        format!(
            "# {name}\n\nVelvet project from template `{template}`.\n\n## Run\n\n```bash\nvelvet script check scripts/main.vel\nvelvet play .\nvelvet play . --lang es\nvelvet launch .\nvelvet document regions scripts/main_menu.vel\n```\n"
        ),
    )?;
    // EN+ES localization scaffold for visual-novel (S6).
    if template == "visual-novel" {
        if let Ok(program) = velvet_story::load_program_from_source(
            &std::fs::read_to_string(dir.join("scripts/main.vel")).unwrap_or_default(),
            Some("scripts/main.vel"),
            &name,
        ) {
            let cat = velvet_story::extract_loc_keys(&program);
            let mut es = velvet_story::TranslationTable::new();
            for e in &cat.entries {
                let t = spanish_hint(&e.source);
                es.insert(e.key.clone(), t);
            }
            let _ = velvet_story::write_tl_scaffold(&dir, &program, "es", &es);
        }
    }
    println!("created {}", dir.display());
    Ok(())
}

fn spanish_hint(en: &str) -> String {
    match en {
        "I said I'd be here." => "Dije que estaria aqui.".into(),
        "You always say that." => "Siempre dices eso.".into(),
        "I keep promises." => "Cumplo mis promesas.".into(),
        "Don't make this heavy." => "No lo pongas tan pesado.".into(),
        "Ask about Kai." => "Pregunta por Kai.".into(),
        "Then maybe tonight is different." => "Entonces tal vez esta noche sea distinta.".into(),
        "You walk home alone." => "Vuelves a casa solo.".into(),
        "Still playing house?" => "Sigues jugando a la casita?".into(),
        "Stand with Mira" => "Quedate con Mira".into(),
        "Leave" => "Marchate".into(),
        "Ending: Warm Lights" => "Final: Luces calidas".into(),
        "Ending: Cool Air" => "Final: Aire fresco".into(),
        other => format!("ES: {other}"),
    }
}

/// Main menu document with visual/advanced regions (round-trip ready).
pub fn template_main_menu_script() -> &'static str {
    r#"// Main menu — visual/advanced regions for Studio round-trip
screen main_menu {
    // @visual id=button.start
    button start {
        text: "Nueva partida"
        position: (50%, 58%)
    // @advanced id=button.start
        on_pressed {
            game.new()
            scene.open("scripts/main.vel")
        }
    // @end
    }
    // @visual id=button.quit
    button quit {
        text: "Salir"
        position: (50%, 72%)
    // @advanced id=button.quit
        on_pressed {
            game.quit()
        }
    // @end
    }
}
"#
}

/// Built-in template script bodies (kept in sync with studio/templates intent).
pub fn template_main_script(template: &str) -> &'static str {
    match template {
        "visual-novel" => {
            r##"character hero { name: "Hero" color: "#ff4f8b" }
character friend { name: "Mira" color: "#4fc3ff" }
character rival { name: "Kai" color: "#ffaa44" }
state {
    trust: int = 0
    chapter: int = 1
}
scene main {
    background "assets/bg/room.png"
    music "assets/music/soft.ogg" fade_in 1.0
    hero "I said I'd be here."
    show friend at right
    friend "You always say that."
    choice {
        "I keep promises." {
            trust += 1
            jump warm
        }
        "Don't make this heavy." {
            trust -= 1
            jump cool
        }
        "Ask about Kai." {
            jump rival_path
        }
    }
}
scene warm {
    friend "Then maybe tonight is different."
    jump ending_good
}
scene cool {
    "You walk home alone."
    jump ending_lonely
}
scene rival_path {
    show rival at left
    rival "Still playing house?"
    choice {
        "Stand with Mira" {
            trust += 2
            jump ending_good
        }
        "Leave" {
            jump ending_lonely
        }
    }
}
scene ending_good {
    "Ending: Warm Lights"
}
scene ending_lonely {
    "Ending: Cool Air"
}
"##
        }
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
            r##"// RPG narrative entry — dialogue + quest hooks (map in scenes/map_town.ron or runtime)
character villager { name: "Mira" color: "#88cc88" }
character elder { name: "Elder" color: "#ccaa66" }
state {
    has_key: bool = false
    quest_started: bool = false
    gold: int = 20
}
scene talk_mira {
    villager "The dungeon door is sealed. Find the key in the woods."
    quest_started = true
    choice {
        "I'll find it." {
            jump woods_hint
        }
        "Not my problem." {
            jump talk_end
        }
    }
}
scene woods_hint {
    villager "Check the hollow stump east of town."
}
scene talk_elder {
    elder "Trade: 10 gold for a map scrap."
    choice {
        "Buy scrap" {
            gold -= 10
            has_key = true
            elder "May it open what was closed."
        }
        "Leave" {
            elder "Coin talks louder than courage."
        }
    }
}
scene talk_end {
    "You walk away. The door stays shut."
}
"##
        }
        "top-down-action" => {
            r##"// Action level boot + briefings
character radio { name: "Dispatch" }
state { score: int = 0 kills: int = 0 }
scene briefing {
    radio "Clear the warehouse. Hostiles on patrol."
    radio "Extract east when the door unlocks."
}
scene debrief {
    radio "Score tallied. Live to extract."
}
function on_level_start() {
    return 1
}
function on_enemy_down() {
    return 1
}
"##
        }
        _ => {
            r#"character hero { name: "Hero" }
scene main {
    hero "Hello from Velvet."
}
"#
        }
    }
}

/// Known template names.
pub fn known_templates() -> &'static [&'static str] {
    &[
        "visual-novel",
        "narrative-adventure",
        "top-down-rpg",
        "top-down-action",
    ]
}

/// List built-in templates (and on-disk `templates/` if present).
pub fn cmd_template_list() -> Result<()> {
    println!("Built-in templates:");
    for t in known_templates() {
        let desc = match *t {
            "visual-novel" => "Classic VN menu + branching story",
            "narrative-adventure" => "Narrative adventure with exploration hooks",
            "top-down-rpg" => "Top-down RPG map + dialogue + quests",
            "top-down-action" => "Top-down action combat arena",
            _ => "",
        };
        println!("  {t:20} {desc}");
    }
    // Optional workspace templates dir
    let tdir = PathBuf::from("templates");
    if tdir.is_dir() {
        println!("On-disk templates/ :");
        if let Ok(rd) = std::fs::read_dir(&tdir) {
            for e in rd.flatten() {
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    println!("  {}", e.file_name().to_string_lossy());
                }
            }
        }
    }
    Ok(())
}

/// Install/copy a template into a new project directory (alias of `new`).
pub fn cmd_template_install(name: String, template: String, out: PathBuf) -> Result<()> {
    if !known_templates().contains(&template.as_str()) {
        bail!(
            "unknown template `{template}`; known: {}",
            known_templates().join(", ")
        );
    }
    cmd_new(name, template, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn new_project_writes_files() {
        let dir = tempdir().unwrap();
        cmd_new(
            "demo".into(),
            "visual-novel".into(),
            dir.path().to_path_buf(),
        )
        .unwrap();
        let root = dir.path().join("demo");
        assert!(root.join("velvet.project").exists());
        assert!(root.join("scripts/main.vel").exists());
        assert!(root.join("scripts/main_menu.vel").exists());
        let menu = std::fs::read_to_string(root.join("scripts/main_menu.vel")).unwrap();
        assert!(menu.contains("@visual"));
        assert!(menu.contains("@advanced"));
        let text = std::fs::read_to_string(root.join("velvet.project")).unwrap();
        let p = velvet_project::VelvetProject::from_ron(&text).unwrap();
        assert_eq!(p.name, "demo");
        assert!(p.has_module("story"));
    }

    #[test]
    fn built_in_template_scripts_parse_cleanly_and_contain_their_entry_scene() {
        let cases = [
            ("visual-novel", "scene main", 8usize),
            ("narrative-adventure", "scene main", 4usize),
            ("top-down-rpg", "scene talk_mira", 5usize),
            ("top-down-action", "scene briefing", 5usize),
        ];
        assert_eq!(known_templates().len(), cases.len());
        for (template, required_source, min_items) in cases {
            let source = template_main_script(template);
            assert!(
                source.contains(required_source),
                "{template} lacks {required_source}"
            );
            assert!(
                !source.contains("TODO"),
                "{template} ships an unfinished placeholder"
            );
            let parsed = velvet_script_parser::parse_file(source, Some(template)).unwrap();
            assert!(
                !parsed.module.has_errors(),
                "{template}: {:?}",
                parsed.module.diagnostics
            );
            assert!(
                parsed.module.items.len() >= min_items,
                "{template} has only {} items",
                parsed.module.items.len()
            );
        }
    }
}
