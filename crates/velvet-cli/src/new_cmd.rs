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

/// Normalize a `tl/<lang>/` code (`fr`, `pt-br`, `ja`).
/// Fails fast before anything is created.
pub fn normalize_lang_code(raw: &str) -> Result<String> {
    let lang = raw.trim().to_ascii_lowercase();
    if lang.len() < 2
        || lang.len() > 8
        || !lang
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("bad language code `{raw}` (use 2-8 letters/digits, e.g. fr, pt-br, ja)");
    }
    Ok(lang)
}

/// Create a new project from a template.
///
/// `langs` adds extra story languages (`tl/<lang>/strings.json` prefilled
/// with English source so the game runs day one; translate at leisure).
/// Visual-novel always ships `en` + `es`.
pub fn cmd_new(name: String, template: String, out: PathBuf, langs: &[String]) -> Result<()> {
    let mut extra_langs: Vec<String> = Vec::new();
    for raw in langs {
        let lang = normalize_lang_code(raw)?;
        if lang != "en" && lang != "es" && !extra_langs.contains(&lang) {
            extra_langs.push(lang);
        }
    }
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
    let mut langs_line = "en, es".to_string();
    for lang in &extra_langs {
        langs_line.push_str(", ");
        langs_line.push_str(lang);
    }
    std::fs::write(
        dir.join("README.md"),
        format!(
            "# {name}\n\nVelvet project from template `{template}`.\n\n## Run\n\n```bash\nvelvet script check scripts/main.vel\nvelvet play .\nvelvet play . --lang es\nvelvet launch .\nvelvet document regions scripts/main_menu.vel\n```\n\n## VN comfort\n\n```bash\nvelvet narrative preview scripts/main.vel --lines 10\nvelvet narrative branches scripts/main.vel\nvelvet narrative check-assets .\n```\n\nScreens (`screen say_classic|say_minimal|nvl_page|menu`), game screens (`screen show|hide|call`, see commented demo in `scripts/main.vel`), mini-ATL (`show x at left with shake 0.4 10`, `hide x with fade`), named transforms (`transform bob {{ … }}` + `show x with bob`), `gallery.json`, `music.json` and `voice_cast.json` ship with this template.\n\n## Languages ({langs_line})\n\nAdd yours any time (game falls back to English for untranslated lines):\n\n```bash\nvelvet localization extract-story scripts/main.vel --project . --lang fr\n# translate tl/fr/strings.json, validate against the script, play:\nvelvet localization validate-story . --lang fr\nvelvet play . --lang fr\n```\n"
        ),
    )?;
    // VN comfort catalogs: gallery + music-room + voice cast.
    if template == "visual-novel" {
        std::fs::write(
            dir.join("gallery.json"),
            "{\"entries\": [{\"group\": \"endings\", \"id\": \"cg_rooftop\", \"path\": \"assets/cg/rooftop.png\", \"sort_order\": 1, \"title\": \"Rooftop Break\"}]}",
        )?;
        std::fs::write(
            dir.join("music.json"),
            "{\"tracks\": [{\"group\": \"main\", \"id\": \"soft\", \"path\": \"assets/music/soft.wav\", \"sort_order\": 1, \"title\": \"Soft Rain\", \"unlocked_by_default\": true}]}",
        )?;
        std::fs::write(
            dir.join("voice_cast.json"),
            "{\"default_volume\": 1.0, \"voices\": [{\"clip_prefix\": \"hero\", \"speaker\": \"hero\", \"volume\": 1.0}, {\"clip_prefix\": \"mira\", \"speaker\": \"friend\", \"volume\": 0.9}, {\"clip_prefix\": \"kai\", \"speaker\": \"rival\", \"volume\": 0.9}]}",
        )?;
        // Placeholder art + audio so check-assets + play work out of the box.
        // Replace every file under assets/ with your own art.
        for (rel, bytes) in VN_PLACEHOLDER_ASSETS {
            let out = dir.join(rel);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(out, bytes)?;
        }
        // Example stylesheet (try `velvet style check styles/ui.vcss`).
        std::fs::create_dir_all(dir.join("styles"))?;
        std::fs::write(dir.join("styles/ui.vcss"), VN_EXAMPLE_STYLE)?;
    }
    // EN+ES localization scaffold for visual-novel (S6) + any extra langs.
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
            // Extra languages prefilled with English source: playable day one.
            for lang in &extra_langs {
                let mut table = velvet_story::TranslationTable::new();
                for e in &cat.entries {
                    table.insert(e.key.clone(), e.source.clone());
                }
                let _ = velvet_story::write_tl_scaffold(&dir, &program, lang, &table);
            }
        }
    }
    println!("created {} (langs: {langs_line})", dir.display());
    Ok(())
}

fn spanish_hint(en: &str) -> String {
    match en {
        "Rain softens the city noise outside the window." => {
            "La lluvia suaviza el ruido de la ciudad tras la ventana.".into()
        }
        "I said I'd be here." => "Dije que estaria aqui.".into(),
        "I said I'd be here. I meant it." => "Dije que estaria aqui. Lo decia en serio.".into(),
        "You always say that." => "Siempre dices eso.".into(),
        "This {b}city{/b} never sleeps, {i}does it?{/i}" => {
            "Esta {b}ciudad{/b} nunca duerme, {i}verdad?{/i}".into()
        }
        "I keep promises." => "Cumplo mis promesas.".into(),
        "I keep my promises." => "Cumplo mis promesas.".into(),
        "Don't make this heavy." => "No lo pongas tan pesado.".into(),
        "Ask about Kai." => "Pregunta por Kai.".into(),
        "Then maybe tonight is different." => "Entonces tal vez esta noche sea distinta.".into(),
        "Look — the clouds are breaking." => "Mira, las nubes se abren.".into(),
        "Stay a little longer." => "Quedate un poco mas.".into(),
        "You walk home alone." => "Vuelves a casa solo.".into(),
        "You walk home alone. The rain has stopped." => {
            "Caminas solo a casa. La lluvia ha parado.".into()
        }
        "Still playing house?" => "Sigues jugando a la casita?".into(),
        "Stand with Mira" => "Quedate con Mira".into(),
        "Leave" => "Marchate".into(),
        "Ending: Warm Lights" => "Final: Luces calidas".into(),
        "Ending: Cool Air" => "Final: Aire fresco".into(),
        other => format!("ES: {other}"),
    }
}

/// Placeholder art + audio embedded from `templates/visual-novel/assets`
/// so `velvet new` works offline with zero downloads.
const VN_PLACEHOLDER_ASSETS: &[(&str, &[u8])] = &[
    (
        "assets/bg/room.png",
        include_bytes!("../../../templates/visual-novel/assets/bg/room.png"),
    ),
    (
        "assets/bg/rooftop.png",
        include_bytes!("../../../templates/visual-novel/assets/bg/rooftop.png"),
    ),
    (
        "assets/bg/street.png",
        include_bytes!("../../../templates/visual-novel/assets/bg/street.png"),
    ),
    (
        "assets/characters/hero.png",
        include_bytes!("../../../templates/visual-novel/assets/characters/hero.png"),
    ),
    (
        "assets/characters/friend.png",
        include_bytes!("../../../templates/visual-novel/assets/characters/friend.png"),
    ),
    (
        "assets/cg/rooftop.png",
        include_bytes!("../../../templates/visual-novel/assets/cg/rooftop.png"),
    ),
    (
        "assets/music/soft.wav",
        include_bytes!("../../../templates/visual-novel/assets/music/soft.wav"),
    ),
    (
        "assets/sfx/chime.wav",
        include_bytes!("../../../templates/visual-novel/assets/sfx/chime.wav"),
    ),
];

/// Example stylesheet embedded from `templates/visual-novel/styles`.
const VN_EXAMPLE_STYLE: &str = include_str!("../../../templates/visual-novel/styles/ui.vcss");

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
            r##"character hero { name: "Hero" color: "#ff4f8b" portrait: "assets/characters/hero.png" }
character friend { name: "Mira" color: "#4fc3ff" portrait: "assets/characters/friend.png" }
character rival { name: "Kai" color: "#ffaa44" }
state {
    trust: int = 0
    chapter: int = 1
}
scene main {
    screen say_classic
    background "assets/bg/room.png"
    music "assets/music/soft.wav" fade_in 1.0
    hero "I said I'd be here."
    show friend at right with fade
    friend "You always say that."
    // Game screens + named transforms (uncomment to try):
    // transform nod {
    //   y -6 0.8 ease_in_out, y 0 0.8 ease_in_out, repeat
    // }
    // screen ticket {
    //   title: "Ticket"
    //   textbutton keep action Return("keep")
    // }
    // show friend at right with nod
    // screen call ticket
    sound "assets/sfx/chime.wav"
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
    show friend.happy at center with zoom 0.3 0.2
    "Ending: Warm Lights"
    unlock gallery cg_rooftop
    unlock music soft
    end "good"
}
scene ending_lonely {
    "Ending: Cool Air"
    unlock music soft
    end "lonely"
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
pub fn cmd_template_install(
    name: String,
    template: String,
    out: PathBuf,
    langs: &[String],
) -> Result<()> {
    if !known_templates().contains(&template.as_str()) {
        bail!(
            "unknown template `{template}`; known: {}",
            known_templates().join(", ")
        );
    }
    cmd_new(name, template, out, langs)
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
            &[],
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
    fn visual_novel_extra_langs_are_prefilled_and_playable() {
        let dir = tempdir().unwrap();
        cmd_new(
            "demo".into(),
            "visual-novel".into(),
            dir.path().to_path_buf(),
            &["fr".to_string(), " JA ".to_string(), "es".to_string()],
        )
        .unwrap();
        let root = dir.path().join("demo");
        // es + normalized extras exist; en always exists
        for lang in ["es", "fr", "ja"] {
            assert!(root.join("tl").join(lang).join("strings.json").exists());
        }
        // extras prefill English source (playable day one), same keys as en
        let en: std::collections::BTreeMap<String, String> = serde_json::from_str(
            &std::fs::read_to_string(root.join("tl/en/strings.json")).unwrap(),
        )
        .unwrap();
        let fr: std::collections::BTreeMap<String, String> = serde_json::from_str(
            &std::fs::read_to_string(root.join("tl/fr/strings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(fr.keys().collect::<Vec<_>>(), en.keys().collect::<Vec<_>>());
        assert_eq!(
            fr.values().collect::<Vec<_>>(),
            en.values().collect::<Vec<_>>()
        );
        // es keeps real translations, not prefills
        let es = std::fs::read_to_string(root.join("tl/es/strings.json")).unwrap();
        assert!(es.contains("Cumplo mis promesas."));
        // fr actually plays to an ending
        let source = std::fs::read_to_string(root.join("scripts/main.vel")).unwrap();
        let program =
            velvet_story::load_program_from_source(&source, Some("main.vel"), "demo").unwrap();
        let mut session = velvet_story::VnSession::new(velvet_story::StoryPlayer::start(program))
            .with_project_root(&root);
        session.set_language("fr").unwrap();
        let mut steps = 0usize;
        loop {
            steps += 1;
            assert!(steps < 200, "fr playthrough did not end");
            match session.player().wait().clone() {
                velvet_story::StoryWait::Line => session.advance(),
                velvet_story::StoryWait::Choice => {
                    session.choose_arm(0).unwrap();
                }
                velvet_story::StoryWait::Ready => session.advance(),
                velvet_story::StoryWait::Ended => break,
                velvet_story::StoryWait::Pause { .. } => session.advance(),
                velvet_story::StoryWait::ScreenCall { .. } => {
                    session
                        .return_from_screen(velvet_story::StoryValue::Null)
                        .unwrap();
                }
                velvet_story::StoryWait::Host { .. } => break,
            }
        }
    }

    #[test]
    fn new_rejects_bad_lang_before_creating_anything() {
        let dir = tempdir().unwrap();
        let err = cmd_new(
            "demo".into(),
            "visual-novel".into(),
            dir.path().to_path_buf(),
            &["xx!".to_string()],
        )
        .unwrap_err();
        assert!(err.to_string().contains("bad language code"));
        assert!(!dir.path().join("demo").exists());
        assert!(normalize_lang_code("PT-BR").unwrap() == "pt-br");
    }

    #[test]
    fn visual_novel_scaffold_is_complete_and_playable() {
        let dir = tempdir().unwrap();
        cmd_new(
            "demo".into(),
            "visual-novel".into(),
            dir.path().to_path_buf(),
            &[],
        )
        .unwrap();
        let root = dir.path().join("demo");
        // catalogs + placeholder art ship with the scaffold
        for f in [
            "gallery.json",
            "music.json",
            "voice_cast.json",
            "assets/bg/room.png",
            "assets/characters/hero.png",
            "assets/cg/rooftop.png",
            "assets/music/soft.wav",
            "assets/sfx/chime.wav",
        ] {
            assert!(root.join(f).exists(), "missing {f}");
        }
        // every referenced asset exists
        crate::narrative_cmd::cmd_narrative_check_assets(root.clone()).unwrap();
        // branches are clean with named endings
        crate::narrative_cmd::cmd_narrative_branches(root.join("scripts/main.vel"), false).unwrap();
        // play to an ending, unlocks fire, saves persist them
        let source = std::fs::read_to_string(root.join("scripts/main.vel")).unwrap();
        let program =
            velvet_story::load_program_from_source(&source, Some("main.vel"), "demo").unwrap();
        let mut session = velvet_story::VnSession::new(velvet_story::StoryPlayer::start(program));
        session.gallery = velvet_story::Gallery::from_path(&root.join("gallery.json")).unwrap();
        session.music_room = velvet_story::MusicRoom::from_path(&root.join("music.json")).unwrap();
        let mut steps = 0usize;
        loop {
            steps += 1;
            assert!(steps < 200, "story did not end");
            match session.player().wait().clone() {
                velvet_story::StoryWait::Line => session.advance(),
                velvet_story::StoryWait::Choice => {
                    session.choose_arm(0).unwrap();
                }
                velvet_story::StoryWait::Ready => session.advance(),
                velvet_story::StoryWait::Ended => break,
                velvet_story::StoryWait::Pause { .. } => session.advance(),
                velvet_story::StoryWait::ScreenCall { .. } => {
                    session
                        .return_from_screen(velvet_story::StoryValue::Null)
                        .unwrap();
                }
                velvet_story::StoryWait::Host { .. } => break,
            }
        }
        assert!(session.gallery.is_unlocked("cg_rooftop"));
        assert!(session.music_room.is_unlocked("soft"));
        let session = session.with_save_dir(dir.path().join("saves"));
        let save = session.save_slot("slot_1").unwrap();
        assert!(save.gallery_unlocked.contains("cg_rooftop"));
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
