//! Velvet Story CLI: check, build, run, format, dump-ast, dump-lowered.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::i18n_extract::{extract, to_catalog};
use velvet_story_lang::pipeline::{
    build_path, check_path, dump_ast_json, dump_lowered_text, run_path,
};
use velvet_story_lang::{format_source, is_idempotent};
use velvet_story_lang::studio::{build_model, model_json};

/// `velvet story check`
pub fn cmd_story_check(path: PathBuf) -> Result<()> {
    let cmds = CommandRegistry::builtin();
    let r = check_path(&path, &cmds).map_err(|e| anyhow::anyhow!(e))?;
    for d in &r.diags {
        println!("{}", d.display());
    }
    if !r.ok {
        bail!("story check failed for {}", path.display());
    }
    let scenes = r
        .file
        .items
        .iter()
        .filter(|i| matches!(i, velvet_story_lang::ast::TopItem::Scene(_)))
        .count();
    println!("ok: {scenes} scene(s) in {}", path.display());
    Ok(())
}

/// `velvet story build`
pub fn cmd_story_build(path: PathBuf) -> Result<()> {
    let cmds = CommandRegistry::builtin();
    let r = build_path(&path, &cmds).map_err(|e| anyhow::anyhow!(e))?;
    for d in &r.check.diags {
        println!("{}", d.display());
    }
    if !r.ok {
        bail!("story build failed for {}", path.display());
    }
    let unit = r.lowered.as_ref().unwrap();
    println!(
        "ok: {} instruction(s), {} scene entr(y/ies), {} msg id(s)",
        unit.unit.code.len(),
        unit.unit.entry_scenes.len(),
        unit.msg_ids.len()
    );
    Ok(())
}

/// `velvet story run`
pub fn cmd_story_run(path: PathBuf, choice: usize) -> Result<()> {
    let cmds = CommandRegistry::builtin();
    let r = run_path(&path, &cmds, choice).map_err(|e| anyhow::anyhow!(e))?;
    for d in &r.build.check.diags {
        if d.is_error() {
            println!("{}", d.display());
        }
    }
    if !r.ok {
        bail!("story run failed for {}", path.display());
    }
    for line in &r.dialogue {
        println!("{line}");
    }
    for line in &r.log {
        println!("# {line}");
    }
    println!("=> steps={} state={:?}", r.steps, r.state);
    Ok(())
}

/// `velvet story format`
pub fn cmd_story_format(path: PathBuf, check: bool) -> Result<()> {
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let pretty = format_source(&source);
    if !is_idempotent(&source) {
        // still write if format produces stable output
        let _ = is_idempotent(&pretty);
    }
    if check {
        if pretty != source {
            bail!("{} needs formatting", path.display());
        }
        println!("ok: formatted {}", path.display());
        return Ok(());
    }
    std::fs::write(&path, pretty)?;
    println!("formatted {}", path.display());
    Ok(())
}

/// `velvet story dump-ast`
pub fn cmd_story_dump_ast(path: PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let json = dump_ast_json(&source, &file).map_err(|e| anyhow::anyhow!(e))?;
    println!("{json}");
    Ok(())
}

/// `velvet story dump-lowered`
pub fn cmd_story_dump_lowered(path: PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let cmds = CommandRegistry::builtin();
    let text = dump_lowered_text(&source, &file, &cmds).map_err(|e| anyhow::anyhow!(e))?;
    println!("{text}");
    Ok(())
}

/// `velvet story studio-model` (JSON for Studio).
pub fn cmd_story_studio_model(path: PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let cmds = CommandRegistry::builtin();
    let model = build_model(&source, &file, &cmds);
    println!("{}", model_json(&model)?);
    Ok(())
}

/// `velvet story extract-loc`
pub fn cmd_story_extract_loc(path: PathBuf, out: Option<PathBuf>) -> Result<()> {
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let extracted = extract(&source, &file);
    let cat = to_catalog(&extracted, "source");
    let json = cat.to_json()?;
    if let Some(p) = out {
        std::fs::write(&p, &json)?;
        println!("wrote {} ({} keys)", p.display(), extracted.entries.len());
    } else {
        println!("{json}");
    }
    Ok(())
}
