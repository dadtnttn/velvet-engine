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
