//! Studio application state: panels, console, selection, command shell.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use velvet_ui::prelude::*;

use crate::commands::{self, CommandContext, CommandId, CommandResult};
use crate::console::{Console, LogLevel};
use crate::inspector::{self, Selection};
use crate::project_browser::{load_project_info, scaffold_project};
use crate::script_panel::ScriptPanel;

/// Studio app state.
pub struct StudioApp {
    /// Project root.
    pub root: PathBuf,
    /// Ring-buffer console.
    pub console: Console,
    /// Selection for inspector.
    pub selection: Selection,
    /// Open script buffers.
    pub scripts: ScriptPanel,
    /// UI tree for layout experiments / future docking.
    pub ui: UiTree,
}

impl StudioApp {
    /// Open project directory.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let root = path.canonicalize().unwrap_or(path);
        let mut app = Self {
            root,
            console: Console::with_capacity(1024),
            selection: Selection::None,
            scripts: ScriptPanel::new(),
            ui: UiTree::with_root("studio"),
        };
        app.console.info(format!("opened {}", app.root.display()));
        if let Some(p) = load_project_info(&app.root)? {
            app.console
                .info(format!("project {} v{}", p.name, p.version));
            app.selection = Selection::Project;
        }
        // Build a simple dock UI structure (host for future egui panels)
        let root_id = app.ui.root().unwrap();
        let _ = app
            .ui
            .add_node("hierarchy", Label::widget("Hierarchy"), Some(root_id));
        let _ = app
            .ui
            .add_node("inspector", Label::widget("Inspector"), Some(root_id));
        let _ = app
            .ui
            .add_node("console", Label::widget("Console"), Some(root_id));
        let _ = app
            .ui
            .add_node("assets", Label::widget("Assets"), Some(root_id));
        let _ = app
            .ui
            .add_node("scripts", Label::widget("Scripts"), Some(root_id));
        Ok(app)
    }

    /// Create project from template.
    pub fn create_project(name: &str, template: &str, out: impl AsRef<Path>) -> Result<()> {
        let dir = scaffold_project(name, template, out.as_ref())?;
        println!("created {}", dir.display());
        Ok(())
    }

    /// Log helper (info).
    pub fn log(&mut self, level: &str, message: impl Into<String>) {
        let lvl = LogLevel::parse(level).unwrap_or(LogLevel::Info);
        self.console.log(lvl, message);
    }

    /// Print hierarchy via command palette.
    pub fn print_hierarchy(&mut self) {
        match self.run_command(CommandId::Hierarchy, &[]) {
            Ok(r) => print_result(&r),
            Err(e) => eprintln!("error: {e:#}"),
        }
    }

    /// Diagnostics on all scripts; returns file count.
    pub fn check_all(&mut self) -> Result<usize> {
        let r = self.run_command(CommandId::Check, &[])?;
        print_result(&r);
        let (n, _, _) = commands::check_all_scripts(&self.root)?;
        Ok(n)
    }

    /// Run a palette command.
    pub fn run_command(&mut self, id: CommandId, args: &[&str]) -> Result<CommandResult> {
        let mut ctx = CommandContext {
            root: &self.root,
            console: &mut self.console,
            selection: &mut self.selection,
            scripts: &mut self.scripts,
        };
        commands::dispatch(&mut ctx, id, args)
    }

    /// Interactive shell with expanded command set.
    pub fn run_shell(mut self) -> Result<()> {
        println!("Velvet Studio shell — project {}", self.root.display());
        println!(
            "commands: hierarchy | check | select <file> | select-symbol <file> <name> | \
             inspect | fmt <file> | open <file> | assets [filter] | new-scene <name> | \
             analyze <file> | console [level] [text] | palette [query] | help | quit"
        );
        let stdin = io::stdin();
        loop {
            print!("studio> ");
            let _ = io::stdout().flush();
            let mut line = String::new();
            if stdin.read_line(&mut line)? == 0 {
                break;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            let cmd = parts[0];
            let args = &parts[1..];

            match cmd {
                "quit" | "exit" | "q" => break,
                "help" | "?" => {
                    let r = self.run_command(CommandId::Help, &[])?;
                    print_result(&r);
                }
                "palette" => {
                    let q = args.first().copied().unwrap_or("");
                    for c in commands::filter_commands(q) {
                        println!("  {} — {}", c.name(), c.description());
                    }
                }
                "hierarchy" | "h" => self.print_hierarchy(),
                "check" | "c" => {
                    let n = self.check_all()?;
                    self.console.info(format!("checked {n} files"));
                }
                "console" => {
                    let r = self.run_command(CommandId::Console, args)?;
                    print_result(&r);
                }
                "select" => {
                    if let Some(f) = args.first() {
                        if *f == "project" {
                            self.selection = Selection::Project;
                        } else {
                            let path = self.root.join(f);
                            self.selection = Selection::File(path.clone());
                            self.console.info(format!("selected {}", path.display()));
                        }
                        let report = inspector::inspect(&self.root, &self.selection)?;
                        inspector::print_report(&report);
                    } else {
                        println!("usage: select <file>|project");
                    }
                }
                "select-symbol" => {
                    if args.len() < 2 {
                        println!("usage: select-symbol <file> <name>");
                    } else {
                        let file = self.root.join(args[0]);
                        let name = args[1];
                        if let Some(sym) = inspector::find_symbol_in_file(&self.root, &file, name)?
                        {
                            self.selection = Selection::Symbol {
                                file: file.clone(),
                                name: sym.name.clone(),
                                kind: sym.kind.clone(),
                            };
                            let report = inspector::inspect(&self.root, &self.selection)?;
                            inspector::print_report(&report);
                        } else {
                            println!("symbol `{name}` not found in {}", file.display());
                        }
                    }
                }
                "inspect" | "i" => {
                    let r = self.run_command(CommandId::Inspect, args)?;
                    print_result(&r);
                }
                "fmt" => {
                    if args.is_empty() {
                        println!("usage: fmt <file>");
                    } else {
                        let r = self.run_command(CommandId::Fmt, args)?;
                        print_result(&r);
                    }
                }
                "open" => {
                    if let Some(f) = args.first() {
                        match self.scripts.open(&self.root, Path::new(f)) {
                            Ok(idx) => {
                                let path = self.scripts.buffers[idx].path.clone();
                                self.selection = Selection::File(path.clone());
                                self.console
                                    .info(format!("opened buffer {idx}: {}", path.display()));
                                if let Some(buf) = self.scripts.active_mut() {
                                    for line in buf.summary() {
                                        println!("  {line}");
                                    }
                                }
                            }
                            Err(e) => eprintln!("error: {e:#}"),
                        }
                    } else {
                        println!("usage: open <file.vel>");
                    }
                }
                "buffers" => {
                    for (i, path, dirty) in self.scripts.list_paths() {
                        let mark = if dirty { "*" } else { " " };
                        let active = if self.scripts.active == Some(i) {
                            ">"
                        } else {
                            " "
                        };
                        println!("{active}{mark} [{i}] {}", path.display());
                    }
                }
                "assets" | "a" => {
                    let r = self.run_command(CommandId::Assets, args)?;
                    print_result(&r);
                }
                "new-scene" => {
                    let r = self.run_command(CommandId::NewScene, args)?;
                    print_result(&r);
                }
                "analyze" => {
                    let r = self.run_command(CommandId::Analyze, args)?;
                    print_result(&r);
                }
                "level" => {
                    if let Some(l) = args.first().and_then(|s| LogLevel::parse(s)) {
                        self.console.set_min_level(l);
                        println!("console min level = {}", l);
                    } else {
                        println!("usage: level <trace|debug|info|warn|error>");
                    }
                }
                other => {
                    // Try palette dispatch by name
                    if let Some(id) = CommandId::parse(other) {
                        match self.run_command(id, args) {
                            Ok(r) => print_result(&r),
                            Err(e) => eprintln!("error: {e:#}"),
                        }
                    } else {
                        println!("unknown command: {other} (try `help` or `palette`)");
                    }
                }
            }
        }
        Ok(())
    }
}

fn print_result(r: &CommandResult) {
    println!("{}", r.message);
    for d in &r.details {
        println!("  {d}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_panel;
    use tempfile::tempdir;

    #[test]
    fn scaffold_check_and_panels() {
        let dir = tempdir().unwrap();
        StudioApp::create_project("demo", "visual-novel", dir.path()).unwrap();
        let mut app = StudioApp::open(dir.path().join("demo")).unwrap();
        let n = app.check_all().unwrap();
        assert!(n >= 1);

        // Assets panel
        let assets = asset_panel::scan_assets(&app.root).unwrap();
        assert!(!assets.is_empty());

        // New scene stub
        let r = app.run_command(CommandId::NewScene, &["epilogue"]).unwrap();
        assert!(r.ok);

        // Open script buffer
        let scripts = crate::project_browser::list_files(&app.root, "vel").unwrap();
        assert!(!scripts.is_empty());
        let rel = scripts[0].strip_prefix(&app.root).unwrap();
        app.scripts.open(&app.root, rel).unwrap();
        assert_eq!(app.scripts.len(), 1);
        app.scripts.format_active().unwrap();

        // Inspector
        app.selection = Selection::Project;
        let report = inspector::inspect(&app.root, &app.selection).unwrap();
        assert!(report.title.contains("Project") || !report.lines.is_empty());

        // Console ring
        app.console.warn("test warn");
        assert!(app.console.len() >= 2);
    }

    #[test]
    fn hierarchy_command() {
        let dir = tempdir().unwrap();
        StudioApp::create_project("h", "top-down-rpg", dir.path()).unwrap();
        let mut app = StudioApp::open(dir.path().join("h")).unwrap();
        let r = app.run_command(CommandId::Hierarchy, &[]).unwrap();
        assert!(r.ok);
        assert!(r
            .details
            .iter()
            .any(|l| l.contains("Scripts") || l.contains("scripts") || l.contains("Project")));
    }
}
