//! Narrative block editing via shipped CLI (simplified authoring path).

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_document::{NarrativeBlock, NarrativeDocument, NarrativeGraph};

/// Load a `.vel` story into narrative blocks, append dialogue + decision, write back.
pub fn cmd_narrative_edit(
    path: PathBuf,
    scene: String,
    speaker: Option<String>,
    dialogue: String,
    choice_a: Option<String>,
    jump_a: Option<String>,
    choice_b: Option<String>,
    jump_b: Option<String>,
) -> Result<()> {
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut doc = NarrativeDocument::from_source(&source).map_err(|e| anyhow::anyhow!("{e}"))?;

    if doc.scene_mut(&scene).is_none() {
        doc.add_scene(&scene);
    }

    doc.push_dialogue(&scene, speaker.as_deref(), &dialogue)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if let (Some(a), Some(ja), Some(b), Some(jb)) = (
        choice_a.as_deref(),
        jump_a.as_deref(),
        choice_b.as_deref(),
        jump_b.as_deref(),
    ) {
        // Ensure targets exist
        if doc.scene_mut(ja).is_none() {
            doc.add_scene(ja);
            doc.push_dialogue(ja, None, format!("Reached {ja}."))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if let Some(sc) = doc.scene_mut(ja) {
                sc.blocks.push(NarrativeBlock::Ending {
                    id: Some(ja.into()),
                });
            }
        }
        if doc.scene_mut(jb).is_none() {
            doc.add_scene(jb);
            doc.push_dialogue(jb, None, format!("Reached {jb}."))
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if let Some(sc) = doc.scene_mut(jb) {
                sc.blocks.push(NarrativeBlock::Ending {
                    id: Some(jb.into()),
                });
            }
        }
        doc.push_binary_decision(&scene, a, ja, b, jb)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    let issues = doc.validate();
    if !issues.is_empty() {
        for i in &issues {
            eprintln!("warn: {i}");
        }
    }

    let out = doc.to_source();
    fs::write(&path, out).with_context(|| format!("write {}", path.display()))?;
    println!("ok: narrative edited scene `{scene}` in {}", path.display());

    // Graph validation snapshot
    let g = NarrativeGraph::from_narrative(&doc);
    let gv = g.validate();
    println!(
        "graph: {} nodes, {} edges, issues={}",
        g.node_count(),
        g.edge_count(),
        gv.issues.len()
    );
    Ok(())
}

/// Branch map for a classic `.vel` story: endings reachability + broken jumps.
/// Exits non-zero when jumps are broken (CI gate for VN branches).
pub fn cmd_narrative_branches(path: PathBuf, dot: bool) -> Result<()> {
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let program = velvet_story::load_program_from_source(&source, Some(&file), "branches")
        .map_err(|e| anyhow::anyhow!("load {}: {e}", path.display()))?;
    let map = velvet_story::BranchMap::analyze(&program);
    print!("{}", map.to_text());
    if dot {
        println!("--- dot ---");
        print!("{}", map.to_dot());
    }
    if !map.broken_jumps.is_empty() {
        bail!(
            "broken jumps: {} (see {} for scene list)",
            map.broken_jumps.join(", "),
            path.display()
        );
    }
    if !map.unreachable_endings.is_empty() {
        println!(
            "warn: unreachable endings: {}",
            map.unreachable_endings.join(", ")
        );
    }
    Ok(())
}

/// Check that every asset a story references exists under the project dir.
/// `path` is a `.vel` file or a project directory (uses `scripts/main.vel`).
pub fn cmd_narrative_check_assets(path: PathBuf) -> Result<()> {
    let (root, vel) = resolve_story_file(&path)?;
    let source = fs::read_to_string(&vel).with_context(|| format!("read {}", vel.display()))?;
    let file = vel.to_string_lossy().to_string();
    let program = velvet_story::load_program_from_source(&source, Some(&file), "assets")
        .map_err(|e| anyhow::anyhow!("load {}: {e}", vel.display()))?;
    let report = velvet_story::check_project_assets(&root, &program);
    print!("{}", report.to_text());
    if !report.is_ok() {
        bail!(
            "{} missing asset(s) under {}",
            report.missing.len(),
            root.display()
        );
    }
    Ok(())
}

/// Dry-run preview: walk the story from entry following first choices and
/// print dialogue lines with screens/motions (no window needed).
pub fn cmd_narrative_preview(path: PathBuf, lines: usize, scene: Option<String>) -> Result<()> {
    use velvet_story::StoryPlayer;
    let (_root, vel) = resolve_story_file(&path)?;
    let source = fs::read_to_string(&vel).with_context(|| format!("read {}", vel.display()))?;
    let file = vel.to_string_lossy().to_string();
    let program = velvet_story::load_program_from_source(&source, Some(&file), "preview")
        .map_err(|e| anyhow::anyhow!("load {}: {e}", vel.display()))?;
    let mut player = StoryPlayer::start(program);
    if let Some(s) = scene {
        player.jump_to_scene(&s);
    }
    let mut shown = 0usize;
    let mut steps = 0usize;
    println!(
        "preview {} (first-choice walk, max {lines} lines):",
        vel.display()
    );
    while shown < lines.max(1) && steps < 2000 {
        steps += 1;
        match player.wait().clone() {
            velvet_story::StoryWait::Line => {
                let (plain, _) = velvet_story::say_plain_and_cps(player.current_text());
                println!(
                    "[{}] {}: {}",
                    player.scene_name(),
                    player.current_speaker_name(),
                    plain.replace('\n', " / ")
                );
                shown += 1;
                player.advance();
            }
            velvet_story::StoryWait::Choice => {
                let opts: Vec<String> = player.choices().iter().map(|c| c.text.clone()).collect();
                println!("[{}] choice: {}", player.scene_name(), opts.join(" | "));
                if player.choose(0).is_err() {
                    break;
                }
            }
            velvet_story::StoryWait::Ready => player.advance(),
            velvet_story::StoryWait::Pause { .. } => player.skip_pause(),
            velvet_story::StoryWait::ScreenCall { id } => {
                println!(
                    "[{}] screen call: {id} (returning null)",
                    player.scene_name()
                );
                if player
                    .return_from_screen(velvet_story::StoryValue::Null)
                    .is_err()
                {
                    break;
                }
            }
            velvet_story::StoryWait::Host { .. } => break,
            velvet_story::StoryWait::Ended => {
                println!("-- ended --");
                break;
            }
        }
    }
    // Also list screens + motions + movies declared in the walked program.
    let program = player.program();
    let mut screens = std::collections::BTreeSet::new();
    let mut motions = std::collections::BTreeSet::new();
    let mut movies = std::collections::BTreeSet::new();
    for scene in program.scenes.values() {
        collect_preview_marks(&scene.ops, &mut screens, &mut motions, &mut movies);
    }
    if !screens.is_empty() {
        println!(
            "screens: {}",
            screens.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    if !motions.is_empty() {
        println!(
            "motions: {}",
            motions.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    if !movies.is_empty() {
        println!(
            "movies: {}",
            movies.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    Ok(())
}

fn collect_preview_marks(
    ops: &[velvet_story::StoryOp],
    screens: &mut std::collections::BTreeSet<String>,
    motions: &mut std::collections::BTreeSet<String>,
    movies: &mut std::collections::BTreeSet<String>,
) {
    use velvet_story::StoryOp;
    for op in ops {
        match op {
            StoryOp::Screen { id } => {
                screens.insert(id.clone());
            }
            StoryOp::ShowGameScreen { id, .. } | StoryOp::CallGameScreen { id, .. } => {
                screens.insert(format!("game:{id}"));
            }
            StoryOp::Movie { path, .. } => {
                movies.insert(path.clone());
            }
            StoryOp::Show {
                motion: Some(m), ..
            } => {
                motions.insert(m.kind.clone());
            }
            StoryOp::Show {
                transform: Some(t), ..
            } => {
                motions.insert(format!("transform:{t}"));
            }
            StoryOp::Choice { options } => {
                for arm in options {
                    collect_preview_marks(&arm.body, screens, motions, movies);
                }
            }
            StoryOp::If {
                then_ops, else_ops, ..
            } => {
                collect_preview_marks(then_ops, screens, motions, movies);
                collect_preview_marks(else_ops, screens, motions, movies);
            }
            _ => {}
        }
    }
}

/// Resolve a `.vel` file or project dir to `(project_root, vel_file)`.
fn resolve_story_file(path: &PathBuf) -> Result<(PathBuf, PathBuf)> {
    use std::path::Path;
    let p = Path::new(path);
    if p.is_file() {
        let root = p.parent().map(|q| q.to_path_buf()).unwrap_or(".".into());
        // project root = parent of `scripts/` when conventional layout
        let root = if root.file_name().map(|n| n == "scripts").unwrap_or(false) {
            root.parent().map(|q| q.to_path_buf()).unwrap_or(root)
        } else {
            root
        };
        return Ok((root, p.to_path_buf()));
    }
    // directory: velvet.project root with scripts/main.vel
    for cand in ["scripts/main.vel", "main.vel"] {
        let vel = p.join(cand);
        if vel.is_file() {
            return Ok((p.to_path_buf(), vel));
        }
    }
    bail!(
        "no story found: {} (expected a .vel file or project dir with scripts/main.vel)",
        path.display()
    );
}

/// Validate narrative graph for a file.
pub fn cmd_narrative_graph(path: PathBuf) -> Result<()> {
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let doc = NarrativeDocument::from_source(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    let g = NarrativeGraph::from_narrative(&doc);
    println!("nodes: {}", g.node_count());
    for n in &g.nodes {
        println!(
            "  [{}] {} @({:.0},{:.0})",
            n.kind.kind_label(),
            n.id,
            n.position.0,
            n.position.1
        );
    }
    println!("edges: {}", g.edge_count());
    for e in &g.edges {
        println!(
            "  {} -> {} ({:?}) {}",
            e.from,
            e.to,
            e.kind,
            e.label.as_deref().unwrap_or("")
        );
    }
    let v = g.validate();
    if v.is_ok() && v.unreachable.is_empty() {
        println!("validation: ok");
    } else {
        for i in &v.issues {
            println!("  issue: {i}");
        }
        if !v.unreachable.is_empty() {
            println!("  unreachable: {}", v.unreachable.join(", "));
        }
        if !v.missing_targets.is_empty() {
            bail!("graph validation failed");
        }
    }
    Ok(())
}

trait GraphNodeKindStr {
    fn kind_label(self) -> &'static str;
}

impl GraphNodeKindStr for velvet_document::GraphNodeKind {
    fn kind_label(self) -> &'static str {
        match self {
            velvet_document::GraphNodeKind::Scene => "scene",
            velvet_document::GraphNodeKind::Ending => "ending",
        }
    }
}

/// Mutate a level JSON: move entity, paint tile, save, reload check.
pub fn cmd_level_mutate(path: PathBuf, entity_id: String, x: f32, y: f32) -> Result<()> {
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut doc =
        velvet_document::LevelDocument::from_json(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    if !doc.move_entity(&entity_id, x, y) {
        bail!("entity not found: {entity_id}");
    }
    if let Some(layer) = doc.layers.first_mut() {
        let _ = layer.paint(2, 2, 7);
    }
    let out = doc.to_json().map_err(|e| anyhow::anyhow!("{e}"))?;
    fs::write(&path, &out)?;
    let again =
        velvet_document::LevelDocument::from_json(&out).map_err(|e| anyhow::anyhow!("{e}"))?;
    let e = again
        .entities
        .iter()
        .find(|e| e.id == entity_id)
        .ok_or_else(|| anyhow::anyhow!("reload missing entity"))?;
    if (e.position.x - x).abs() > 0.01 || (e.position.y - y).abs() > 0.01 {
        bail!("position not preserved after reload");
    }
    println!(
        "ok: level {} entity {entity_id} -> ({x},{y}); tiles filled={}",
        path.display(),
        again.layers.first().map(|l| l.filled_count()).unwrap_or(0)
    );
    Ok(())
}
