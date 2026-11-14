//! Velvet Story CLI: check, build, run, format, dump-ast, dump-program.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::i18n_extract::{extract, to_catalog};
use velvet_story_lang::pipeline::{
    build_story_program, check_path, dump_ast_json, dump_program_json, run_path_product,
};
use velvet_story_lang::studio::{build_model, model_json};
use velvet_story_lang::{format_source, is_idempotent};

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

/// `velvet story build` — the StoryProgram (canonical IR) StoryPlayer runs.
pub fn cmd_story_build(path: PathBuf) -> Result<()> {
    let cmds = CommandRegistry::builtin();
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let prog = build_story_program(
        &source,
        &file,
        &cmds,
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("story"),
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    println!(
        "ok: StoryProgram entry={} scenes={}",
        prog.entry,
        prog.scenes.len()
    );
    Ok(())
}

/// `velvet story run` — plays the story on StoryPlayer, taking choice `choice` at every menu.
pub fn cmd_story_run(path: PathBuf, choice: usize) -> Result<()> {
    let cmds = CommandRegistry::builtin();
    let r = run_path_product(&path, &cmds, choice)
        .map_err(|e| anyhow::anyhow!("story run failed for {}: {e}", path.display()))?;
    for line in &r.dialogue {
        println!("{line}");
    }
    println!("=> steps={} ended={} vars={:?}", r.steps, r.ended, r.vars);
    Ok(())
}

/// `velvet story format`
pub fn cmd_story_format(path: PathBuf, check: bool) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
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
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let json = dump_ast_json(&source, &file).map_err(|e| anyhow::anyhow!(e))?;
    println!("{json}");
    Ok(())
}

/// `velvet story dump-program` — the StoryProgram as JSON (developer tool).
pub fn cmd_story_dump_program(path: PathBuf) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let cmds = CommandRegistry::builtin();
    let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("story");
    let json = dump_program_json(&source, &file, &cmds, title).map_err(|e| anyhow::anyhow!(e))?;
    println!("{json}");
    Ok(())
}

/// `velvet story studio-model` (JSON for Studio).
pub fn cmd_story_studio_model(path: PathBuf) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let cmds = CommandRegistry::builtin();
    let model = build_model(&source, &file, &cmds);
    println!("{}", model_json(&model)?);
    Ok(())
}

/// `velvet story extract-loc`
pub fn cmd_story_extract_loc(path: PathBuf, out: Option<PathBuf>) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
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
