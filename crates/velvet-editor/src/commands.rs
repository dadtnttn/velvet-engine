//! Command palette actions for Velvet Studio.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use velvet_script_format::format_source;
use velvet_script_lsp::analyze;

use crate::asset_panel;
use crate::console::{Console, LogLevel};
use crate::inspector::{self, Selection};
use crate::project_browser::{list_files, load_project_info, scaffold_project};
use crate::script_panel::{self, ScriptPanel};
use crate::story_lang;

/// Result of running a palette command.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Human-readable summary.
    pub message: String,
    /// Whether the action is considered successful.
    pub ok: bool,
    /// Optional extra detail lines.
    pub details: Vec<String>,
}

impl CommandResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ok: true,
            details: Vec::new(),
        }
    }

    pub fn ok_details(message: impl Into<String>, details: Vec<String>) -> Self {
        Self {
            message: message.into(),
            ok: true,
            details,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ok: false,
            details: Vec::new(),
        }
    }
}

/// Known palette command identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    /// Run diagnostics on all scripts.
    Check,
    /// Format a script file.
    Fmt,
    /// List hierarchy / symbols.
    Hierarchy,
    /// List assets.
    Assets,
    /// Inspect selection / project.
    Inspect,
    /// New scene stub.
    NewScene,
    /// New project scaffold.
    NewProject,
    /// Console dump.
    Console,
    /// Help / list commands.
    Help,
    /// Analyze open / path script.
    Analyze,
    /// Outline a `.vstory` via velvet-story-lang Studio model.
    StoryOutline,
    /// Branch map for a classic `.vel` story (reachable endings, broken jumps).
    Branches,
    /// Check story asset references exist under the project.
    CheckAssets,
    /// Dry-run dialogue preview (no window).
    VnPreview,
}

impl CommandId {
    /// Parse from user token.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "check" | "c" | "diagnostics" => Some(Self::Check),
            "fmt" | "format" => Some(Self::Fmt),
            "hierarchy" | "h" | "tree" => Some(Self::Hierarchy),
            "assets" | "asset" | "a" => Some(Self::Assets),
            "inspect" | "inspector" | "i" => Some(Self::Inspect),
            "new-scene" | "new_scene" | "scene" => Some(Self::NewScene),
            "new-project" | "new_project" | "new" => Some(Self::NewProject),
            "console" | "log" => Some(Self::Console),
            "help" | "?" => Some(Self::Help),
            "analyze" | "lsp" => Some(Self::Analyze),
            "story-outline" | "story_outline" | "vstory" | "outline-story" => {
                Some(Self::StoryOutline)
            }
            "branches" | "branch" | "branch-map" | "branch_map" => Some(Self::Branches),
            "check-assets" | "check_assets" | "assets-check" => Some(Self::CheckAssets),
            "vn-preview" | "vn_preview" | "preview-vn" | "preview" => Some(Self::VnPreview),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Fmt => "fmt",
            Self::Hierarchy => "hierarchy",
            Self::Assets => "assets",
            Self::Inspect => "inspect",
            Self::NewScene => "new-scene",
            Self::NewProject => "new-project",
            Self::Console => "console",
            Self::Help => "help",
            Self::Analyze => "analyze",
            Self::StoryOutline => "story-outline",
            Self::Branches => "branches",
            Self::CheckAssets => "check-assets",
            Self::VnPreview => "vn-preview",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Check => "Run diagnostics on all .vel scripts",
            Self::Fmt => "Format a .vel file (fmt <path>)",
            Self::Hierarchy => "Print project hierarchy and symbols",
            Self::Assets => "List assets with optional type filter",
            Self::Inspect => "Show inspector for selection / project",
            Self::NewScene => "Create a stub scene script (new-scene <name>)",
            Self::NewProject => "Scaffold project (new-project <name> [template])",
            Self::Console => "Show console lines (optional level filter)",
            Self::Help => "List palette commands",
            Self::Analyze => "Analyze a script file",
            Self::StoryOutline => "Outline a .vstory (scenes, diags, includes)",
            Self::Branches => "Branch map for a .vel story (branches <file>)",
            Self::CheckAssets => "Check .vel asset refs exist (check-assets [file|dir])",
            Self::VnPreview => "Dialogue dry-run preview (vn-preview <file> [lines])",
        }
    }
}

/// All palette commands for help.
pub fn all_commands() -> &'static [CommandId] {
    &[
        CommandId::Check,
        CommandId::Fmt,
        CommandId::Hierarchy,
        CommandId::Assets,
        CommandId::Inspect,
        CommandId::NewScene,
        CommandId::NewProject,
        CommandId::Console,
        CommandId::Analyze,
        CommandId::StoryOutline,
        CommandId::Branches,
        CommandId::CheckAssets,
        CommandId::VnPreview,
        CommandId::Help,
    ]
}

/// Shared context for command execution.
pub struct CommandContext<'a> {
    /// Project root.
    pub root: &'a Path,
    /// Console.
    pub console: &'a mut Console,
    /// Selection.
    pub selection: &'a mut Selection,
    /// Script buffers.
    pub scripts: &'a mut ScriptPanel,
}

/// Dispatch a palette command with free-form args.
pub fn dispatch(
    ctx: &mut CommandContext<'_>,
    id: CommandId,
    args: &[&str],
) -> Result<CommandResult> {
    match id {
        CommandId::Help => Ok(help_result()),
        CommandId::Check => cmd_check(ctx),
        CommandId::Fmt => cmd_fmt(ctx, args),
        CommandId::Hierarchy => cmd_hierarchy(ctx),
        CommandId::Assets => cmd_assets(ctx, args),
        CommandId::Inspect => cmd_inspect(ctx, args),
        CommandId::NewScene => cmd_new_scene(ctx, args),
        CommandId::NewProject => cmd_new_project(ctx, args),
        CommandId::Console => cmd_console(ctx, args),
        CommandId::Analyze => cmd_analyze(ctx, args),
        CommandId::StoryOutline => cmd_story_outline(ctx, args),
        CommandId::Branches => cmd_branches(ctx, args),
        CommandId::CheckAssets => cmd_check_assets(ctx, args),
        CommandId::VnPreview => cmd_vn_preview(ctx, args),
    }
}

fn cmd_story_outline(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let rel = args.first().copied().unwrap_or("stories/main.vstory");
    let path = if Path::new(rel).is_absolute() {
        PathBuf::from(rel)
    } else {
        ctx.root.join(rel)
    };
    if !path.exists() {
        bail!("story file not found: {}", path.display());
    }
    let model = story_lang::story_studio_model_path(&path)
        .map_err(|e| anyhow::anyhow!("story model: {e}"))?;
    let mut details: Vec<String> = model
        .scenes
        .iter()
        .map(|s| {
            let origin = s.origin_file.as_deref().unwrap_or(model.file.as_str());
            format!("scene {} @ {} (line {})", s.name, origin, s.line)
        })
        .collect();
    for d in &model.diagnostics {
        details.push(format!("diag [{}] {}", d.code, d.message));
    }
    ctx.console.log(
        LogLevel::Info,
        format!(
            "story-outline {} scenes={} diags={}",
            path.display(),
            model.scenes.len(),
            model.diagnostics.len()
        ),
    );
    Ok(CommandResult::ok_details(
        format!(
            "outline {} ({} scene(s))",
            path.display(),
            model.scenes.len()
        ),
        details,
    ))
}

fn resolve_vel(ctx: &CommandContext<'_>, arg: Option<&str>, default: &str) -> PathBuf {
    let rel = arg.unwrap_or(default);
    if Path::new(rel).is_absolute() {
        PathBuf::from(rel)
    } else {
        ctx.root.join(rel)
    }
}

fn cmd_branches(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let path = resolve_vel(ctx, args.first().copied(), "scripts/main.vel");
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let program = velvet_story::load_program_from_source(&source, Some(&file), "branches")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let map = velvet_story::BranchMap::analyze(&program);
    let mut details = vec![map.to_text().trim().to_string()];
    for e in &map.edges {
        details.push(format!("{} -> {} ({})", e.from, e.to, e.reason));
    }
    ctx.console.log(
        LogLevel::Info,
        format!("branches {} endings={}", path.display(), map.endings.len()),
    );
    if !map.broken_jumps.is_empty() {
        return Ok(CommandResult {
            message: format!("branches: BROKEN {}", map.broken_jumps.join(", ")),
            ok: false,
            details,
        });
    }
    Ok(CommandResult::ok_details(
        format!(
            "branches {} ({} scenes, {} endings)",
            path.display(),
            map.scenes.len(),
            map.endings.len()
        ),
        details,
    ))
}

fn cmd_check_assets(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let rel = args.first().copied().unwrap_or("scripts/main.vel");
    let p = resolve_vel(ctx, Some(rel), "scripts/main.vel");
    let (root, vel) = if p.is_file() {
        let root = p
            .parent()
            .and_then(|q| {
                if q.file_name().map(|n| n == "scripts").unwrap_or(false) {
                    q.parent()
                } else {
                    Some(q)
                }
            })
            .map(|q| q.to_path_buf())
            .unwrap_or_else(|| ctx.root.to_path_buf());
        (root, p)
    } else {
        (p.clone(), p.join("scripts/main.vel"))
    };
    let source = fs::read_to_string(&vel).with_context(|| format!("read {}", vel.display()))?;
    let file = vel.to_string_lossy().to_string();
    let program = velvet_story::load_program_from_source(&source, Some(&file), "assets")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let report = velvet_story::check_project_assets(&root, &program);
    let details: Vec<String> = report
        .missing
        .iter()
        .map(|m| format!("missing: {}", m.display()))
        .collect();
    ctx.console.log(
        LogLevel::Info,
        format!(
            "check-assets {} checked={} missing={}",
            vel.display(),
            report.checked,
            report.missing.len()
        ),
    );
    if !report.is_ok() {
        return Ok(CommandResult {
            message: format!("assets: {} missing", report.missing.len()),
            ok: false,
            details,
        });
    }
    Ok(CommandResult::ok(format!(
        "assets ok ({} checked)",
        report.checked
    )))
}

fn cmd_vn_preview(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let path = resolve_vel(ctx, args.first().copied(), "scripts/main.vel");
    let lines: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(12);
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy().to_string();
    let program = velvet_story::load_program_from_source(&source, Some(&file), "preview")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut player = velvet_story::StoryPlayer::start(program);
    let mut details = Vec::new();
    let mut steps = 0usize;
    while details.len() < lines.max(1) && steps < 2000 {
        steps += 1;
        match player.wait().clone() {
            velvet_story::StoryWait::Line => {
                let (plain, _) = velvet_story::say_plain_and_cps(player.current_text());
                details.push(format!(
                    "[{}] {}: {}",
                    player.scene_name(),
                    player.current_speaker_name(),
                    plain.replace('\n', " / ")
                ));
                player.advance();
            }
            velvet_story::StoryWait::Choice => {
                let opts: Vec<String> = player.choices().iter().map(|c| c.text.clone()).collect();
                details.push(format!("choice: {}", opts.join(" | ")));
                if player.choose(0).is_err() {
                    break;
                }
            }
            velvet_story::StoryWait::Ready => player.advance(),
            velvet_story::StoryWait::Pause { .. } => player.skip_pause(),
            velvet_story::StoryWait::ScreenCall { id } => {
                details.push(format!("screen call: {id}"));
                if player
                    .return_from_screen(velvet_story::StoryValue::Null)
                    .is_err()
                {
                    break;
                }
            }
            velvet_story::StoryWait::Host { .. } | velvet_story::StoryWait::Ended => break,
        }
    }
    ctx.console.log(
        LogLevel::Info,
        format!("vn-preview {} lines={}", path.display(), details.len()),
    );
    Ok(CommandResult::ok_details(
        format!("preview {} ({} lines)", path.display(), details.len()),
        details,
    ))
}

fn help_result() -> CommandResult {
    let details = all_commands()
        .iter()
        .map(|c| format!("{} — {}", c.name(), c.description()))
        .collect();
    CommandResult::ok_details("palette commands", details)
}

/// Check all scripts under root; returns file count and issue count.
pub fn check_all_scripts(root: &Path) -> Result<(usize, usize, Vec<String>)> {
    let scripts = list_files(root, "vel")?;
    let mut issues = 0usize;
    let mut details = Vec::new();
    for s in &scripts {
        let src = fs::read_to_string(s).with_context(|| format!("read {}", s.display()))?;
        let a = analyze(&src, Some(&s.to_string_lossy()));
        for d in &a.diagnostics {
            details.push(format!(
                "{}:{}:{}: {}",
                s.display(),
                d.line + 1,
                d.character + 1,
                d.message
            ));
            issues += 1;
        }
        if let Ok(formatted) = format_source(&src) {
            if formatted.is_empty() && !src.trim().is_empty() {
                details.push(format!("{}: formatter produced empty output", s.display()));
                issues += 1;
            }
        }
    }
    Ok((scripts.len(), issues, details))
}

fn cmd_check(ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
    let (files, issues, details) = check_all_scripts(ctx.root)?;
    ctx.console.log(
        if issues > 0 {
            LogLevel::Warn
        } else {
            LogLevel::Info
        },
        format!("check: {files} file(s), {issues} issue(s)"),
    );
    Ok(CommandResult::ok_details(
        format!("checked {files} script(s), {issues} issue(s)"),
        details,
    ))
}

fn cmd_fmt(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let rel = args
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("usage: fmt <file>"))?;
    let path = resolve(ctx.root, rel);
    script_panel::format_file_on_disk(&path)?;
    ctx.console.info(format!("formatted {}", path.display()));
    // Refresh buffer if open
    if let Some(buf) = ctx.scripts.buffers.iter_mut().find(|b| b.path == path) {
        let _ = buf.reload();
    }
    Ok(CommandResult::ok(format!("formatted {}", path.display())))
}

fn cmd_hierarchy(ctx: &mut CommandContext<'_>) -> Result<CommandResult> {
    let mut details = Vec::new();
    details.push(format!("Project: {}", ctx.root.display()));
    if let Ok(Some(p)) = load_project_info(ctx.root) {
        details.push(format!("  name: {}", p.name));
        details.push(format!("  modules: {}", p.modules.join(", ")));
        details.push(format!("  entry: {}", p.entry_scene));
    }
    details.push("Scripts:".into());
    if let Ok(scripts) = list_files(ctx.root, "vel") {
        for s in scripts {
            let rel = s.strip_prefix(ctx.root).unwrap_or(&s);
            details.push(format!("  {}", rel.display()));
            if let Ok(src) = fs::read_to_string(&s) {
                let a = analyze(&src, Some(&s.to_string_lossy()));
                for sym in a.symbols {
                    details.push(format!(
                        "    - {} ({}) @{}:{}",
                        sym.name,
                        sym.kind,
                        sym.line + 1,
                        sym.character + 1
                    ));
                }
            }
        }
    }
    details.push("Assets:".into());
    let assets = asset_panel::scan_assets(ctx.root).unwrap_or_default();
    for a in assets.iter().take(40) {
        details.push(format!("  [{}] {}", a.kind.as_str(), a.relative.display()));
    }
    if assets.len() > 40 {
        details.push(format!("  … {} more", assets.len() - 40));
    }
    ctx.console.info("hierarchy printed");
    Ok(CommandResult::ok_details("hierarchy", details))
}

fn cmd_assets(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let filter = asset_panel::parse_filter_args(args);
    let all = asset_panel::scan_assets(ctx.root)?;
    let filtered = asset_panel::filter_assets(&all, &filter);
    let details = asset_panel::format_listing(&filtered, true);
    ctx.console.info(format!(
        "assets: {} shown / {} total",
        filtered.len(),
        all.len()
    ));
    Ok(CommandResult::ok_details(
        format!("{} asset(s)", filtered.len()),
        details,
    ))
}

fn cmd_inspect(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    if let Some(path) = args.first() {
        if *path == "project" {
            *ctx.selection = Selection::Project;
        } else {
            *ctx.selection = Selection::File(resolve(ctx.root, path));
        }
    }
    let report = inspector::inspect(ctx.root, ctx.selection)?;
    ctx.console.debug(format!("inspect: {}", report.title));
    Ok(CommandResult::ok_details(
        report.title.clone(),
        report.lines,
    ))
}

/// Create a minimal scene stub under scripts/ or scenes/.
pub fn create_scene_stub(root: &Path, name: &str) -> Result<PathBuf> {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        bail!("invalid scene name");
    }
    let dir = if root.join("scripts").exists() {
        root.join("scripts")
    } else {
        let d = root.join("scenes");
        fs::create_dir_all(&d)?;
        d
    };
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{safe}.vel"));
    if path.exists() {
        bail!("scene already exists: {}", path.display());
    }
    let body = format!(
        r#"// Scene stub generated by Velvet Studio
scene {safe} {{
    "TODO: write dialogue for scene `{safe}`."
}}
"#
    );
    fs::write(&path, body)?;
    Ok(path)
}

fn cmd_new_scene(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let name = args
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("usage: new-scene <name>"))?;
    let path = create_scene_stub(ctx.root, name)?;
    *ctx.selection = Selection::File(path.clone());
    ctx.console
        .info(format!("created scene {}", path.display()));
    Ok(CommandResult::ok(format!("created {}", path.display())))
}

fn cmd_new_project(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let name = args
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("usage: new-project <name> [template]"))?;
    let template = args.get(1).copied().unwrap_or("visual-novel");
    let parent = ctx.root;
    let dir = scaffold_project(name, template, parent)?;
    ctx.console
        .info(format!("scaffolded {} ({template})", dir.display()));
    Ok(CommandResult::ok(format!("created {}", dir.display())))
}

fn cmd_console(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    if let Some(level) = args.first().and_then(|s| LogLevel::parse(s)) {
        ctx.console.set_min_level(level);
    }
    if let Some(text) = args.get(1) {
        ctx.console.set_text_filter(Some((*text).to_string()));
    }
    let lines = ctx.console.format_filtered();
    Ok(CommandResult::ok_details(
        format!("{} console line(s)", lines.len()),
        lines,
    ))
}

fn cmd_analyze(ctx: &mut CommandContext<'_>, args: &[&str]) -> Result<CommandResult> {
    let rel = args
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("usage: analyze <file>"))?;
    let path = resolve(ctx.root, rel);
    let a = script_panel::analyze_file_on_disk(&path)?;
    let mut details = Vec::new();
    details.push(format!("symbols: {}", a.symbols.len()));
    for s in &a.symbols {
        details.push(format!(
            "  {} ({}) @{}:{}",
            s.name,
            s.kind,
            s.line + 1,
            s.character + 1
        ));
    }
    details.push(format!("diagnostics: {}", a.diagnostics.len()));
    for d in &a.diagnostics {
        details.push(format!(
            "  {}:{}: {}",
            d.line + 1,
            d.character + 1,
            d.message
        ));
    }
    ctx.console.info(format!(
        "analyze {}: {} sym / {} diag",
        path.display(),
        a.symbols.len(),
        a.diagnostics.len()
    ));
    Ok(CommandResult::ok_details(
        format!("analyzed {}", path.display()),
        details,
    ))
}

fn resolve(root: &Path, rel: &str) -> PathBuf {
    let p = PathBuf::from(rel);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

/// Filter palette commands by fuzzy substring.
pub fn filter_commands(query: &str) -> Vec<CommandId> {
    let q = query.to_ascii_lowercase();
    if q.is_empty() {
        return all_commands().to_vec();
    }
    all_commands()
        .iter()
        .copied()
        .filter(|c| c.name().contains(&q) || c.description().to_ascii_lowercase().contains(&q))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::Console;
    use tempfile::tempdir;

    #[test]
    fn check_and_new_scene() {
        let dir = tempdir().unwrap();
        scaffold_project("demo", "visual-novel", dir.path()).unwrap();
        let root = dir.path().join("demo");
        let (n, issues, details) = check_all_scripts(&root).unwrap();
        assert_eq!(n, 1, "Studio scaffold contains one main story script");
        assert_eq!(issues, 0, "details={details:?}");

        let path = create_scene_stub(&root, "intro").unwrap();
        assert!(path.exists());
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("scene intro"));
    }

    #[test]
    fn dispatch_help_and_assets() {
        let dir = tempdir().unwrap();
        scaffold_project("p", "visual-novel", dir.path()).unwrap();
        let root = dir.path().join("p");
        let mut console = Console::default();
        let mut selection = Selection::None;
        let mut scripts = ScriptPanel::new();
        let mut ctx = CommandContext {
            root: &root,
            console: &mut console,
            selection: &mut selection,
            scripts: &mut scripts,
        };
        let r = dispatch(&mut ctx, CommandId::Help, &[]).unwrap();
        assert!(r.ok);
        assert_eq!(r.message, "palette commands");
        assert_eq!(r.details.len(), all_commands().len());
        assert!(r.details.iter().any(|line| line.starts_with("check —")));

        let r = dispatch(&mut ctx, CommandId::Assets, &["script"]).unwrap();
        assert!(r.ok);
        assert!(r.message.ends_with("asset(s)"));
        assert!(r
            .details
            .iter()
            .all(|line| line.contains("Script") || line.contains("script")));
    }

    #[test]
    fn filter_commands_fuzzy() {
        let c = filter_commands("check");
        assert!(c.contains(&CommandId::Check));
    }

    #[test]
    fn check_all_scripts_on_scaffold() {
        let dir = tempdir().unwrap();
        scaffold_project("vn", "visual-novel", dir.path()).unwrap();
        let root = dir.path().join("vn");
        let (n, issues, details) = check_all_scripts(&root).unwrap();
        assert_eq!(n, 1);
        assert_eq!(issues, 0, "details={details:?}");
    }

    #[test]
    fn create_scene_and_check() {
        let dir = tempdir().unwrap();
        scaffold_project("p", "visual-novel", dir.path()).unwrap();
        let root = dir.path().join("p");
        let path = create_scene_stub(&root, "chapter_two").unwrap();
        assert!(path.exists());
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("scene chapter_two {"));
        let (n, issues, details) = check_all_scripts(&root).unwrap();
        assert_eq!(n, 2);
        assert_eq!(issues, 0, "details={details:?}");
    }

    #[test]
    fn dispatch_check_command() {
        let dir = tempdir().unwrap();
        scaffold_project("p2", "visual-novel", dir.path()).unwrap();
        let root = dir.path().join("p2");
        let mut console = Console::default();
        let mut selection = Selection::None;
        let mut scripts = ScriptPanel::new();
        let mut ctx = CommandContext {
            root: &root,
            console: &mut console,
            selection: &mut selection,
            scripts: &mut scripts,
        };
        let r = dispatch(&mut ctx, CommandId::Check, &[]).unwrap();
        assert!(r.ok);
        assert_eq!(r.message, "checked 1 script(s), 0 issue(s)");
        assert!(r.details.is_empty());
    }

    #[test]
    fn filter_commands_empty_and_partial() {
        let all = filter_commands("");
        assert_eq!(all, all_commands());
        let help = filter_commands("hel");
        assert_eq!(help, vec![CommandId::Help]);
    }

    #[test]
    fn dispatch_branches_and_preview_on_scaffold() {
        let dir = tempdir().unwrap();
        scaffold_project("p3", "visual-novel", dir.path()).unwrap();
        let root = dir.path().join("p3");
        let mut console = Console::default();
        let mut selection = Selection::None;
        let mut scripts = ScriptPanel::new();
        let mut ctx = CommandContext {
            root: &root,
            console: &mut console,
            selection: &mut selection,
            scripts: &mut scripts,
        };
        let r = dispatch(&mut ctx, CommandId::Branches, &["scripts/main.vel"]).unwrap();
        assert!(r.ok, "{}", r.message);
        assert!(r.details.iter().any(|l| l.starts_with("scenes=")));
        let r = dispatch(&mut ctx, CommandId::VnPreview, &["scripts/main.vel", "4"]).unwrap();
        assert!(r.ok, "{}", r.message);
        assert!(!r.details.is_empty());
        // complete scaffold ships placeholder art: check-assets passes;
        // the embedded fallback (no templates/ dir) still reports honestly
        let r = dispatch(&mut ctx, CommandId::CheckAssets, &["scripts/main.vel"]).unwrap();
        if root.join("gallery.json").exists() {
            assert!(r.ok, "{}", r.message);
        } else {
            assert!(!r.ok);
            assert!(r.details.iter().any(|l| l.starts_with("missing:")));
        }
    }

    #[test]
    fn scaffold_copies_catalogs_and_placeholder_assets() {
        let dir = tempdir().unwrap();
        let root = scaffold_project("p4", "visual-novel", dir.path()).unwrap();
        // full copy only when the workspace templates/ dir is reachable;
        // otherwise the embedded minimal fallback has no catalogs to check
        if !root.join("gallery.json").exists() {
            return;
        }
        for f in [
            "music.json",
            "voice_cast.json",
            "assets/bg/room.png",
            "assets/music/soft.wav",
        ] {
            assert!(root.join(f).exists(), "missing scaffold file {f}");
        }
    }

    #[test]
    fn dispatch_story_outline_includes_child_scene() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let stories = root.join("stories");
        fs::create_dir_all(&stories).unwrap();
        fs::write(
            stories.join("part.vstory"),
            "scene from_include\nnarrator:\n    hi\nend\n",
        )
        .unwrap();
        fs::write(
            stories.join("main.vstory"),
            "include \"part.vstory\"\n\nscene start\nnarrator:\n    root\nend\n",
        )
        .unwrap();
        let mut console = Console::default();
        let mut selection = Selection::None;
        let mut scripts = ScriptPanel::new();
        let mut ctx = CommandContext {
            root,
            console: &mut console,
            selection: &mut selection,
            scripts: &mut scripts,
        };
        let r = dispatch(&mut ctx, CommandId::StoryOutline, &["stories/main.vstory"]).unwrap();
        assert!(r.ok, "{}", r.message);
        let blob = r.details.join("\n");
        assert!(
            blob.contains("from_include"),
            "outline must list included scene: {blob}"
        );
        assert!(
            blob.contains("start"),
            "outline must list root scene: {blob}"
        );
    }
}
