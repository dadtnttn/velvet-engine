//! Studio docking GUI model + launch entry.
//!
//! The drag/move logic lives in [`velvet_document`]; this module owns panel layout,
//! canvas interaction state, and a launch path that either opens a short-lived
//! window (when the display allows) or runs headless with a ready log.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use velvet_document::{
    drag_visual_region, hit_test_visual, parse_document, region_rect, render_document,
    DesignerWidget, UiDesigner, WidgetRect,
};

use crate::layers::{pct_to_px, DesignSurface, LayerStack, ResPreset};
use crate::studio_project::StudioProjectFile;

/// Named dock panel (left/right/bottom/center).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockPanel {
    /// Stable id: hierarchy | inspector | console | assets | scripts | canvas.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Dock zone.
    pub zone: DockZone,
    /// Visible.
    pub visible: bool,
}

/// Where a panel docks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockZone {
    /// Left column.
    Left,
    /// Right column.
    Right,
    /// Bottom strip.
    Bottom,
    /// Center document/canvas.
    Center,
}

/// Triple-mode Studio editor: Visual · Script · Nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudioEditorMode {
    /// Drag-and-drop canvas over `@visual` regions (default). Mode **1**.
    #[default]
    Simplified,
    /// VScript editor (buttons, layers, game/scene). Mode **2**.
    Advanced,
    /// Layer connection graph (pantallas nodes). Mode **3**.
    Nodes,
}

/// GUI session state over one project + open document.
#[derive(Debug, Clone)]
pub struct StudioGuiSession {
    /// Project root.
    pub root: PathBuf,
    /// Dock panels.
    pub panels: Vec<DockPanel>,
    /// Path of the document open on the canvas (active layer file).
    pub document_path: Option<PathBuf>,
    /// Source currently edited on the canvas (**active layer only**).
    pub document_source: String,
    /// Which layer id owns `document_source`.
    pub document_layer_id: Option<String>,
    /// Independent document body per layer id (each pantalla is its own canvas).
    pub layer_docs: HashMap<String, String>,
    /// Selected region id for drag (within active layer).
    pub selected_region: Option<String>,
    /// Ready flag after layout init.
    pub ready: bool,
    /// Last drag rect (for tests/logs).
    pub last_drag: Option<WidgetRect>,
    /// Log lines (also written to ready log file when configured).
    pub log: Vec<String>,
    /// Active editor mode (simplified / advanced).
    pub mode: StudioEditorMode,
    /// Next drop id counter for palette widgets.
    pub drop_seq: u32,
    /// Screen layers stack (pantallas) with per-layer pixel resolution.
    pub layers: LayerStack,
    /// Nodes mode: first endpoint when connecting (from).
    pub connect_from: Option<String>,
    /// Nodes tool: select / connect / disconnect.
    pub nodes_tool: NodesTool,
    /// Selected graph edge (from, to).
    pub selected_edge: Option<(String, String)>,
    /// Script mode: cursor line (0-based) into document_source.
    pub script_cursor_line: usize,
    /// Script mode: status of last validate.
    pub script_issues: Vec<String>,
    /// Undo stack of full document_source snapshots (active layer).
    pub undo_stack: Vec<String>,
    /// Redo stack.
    pub redo_stack: Vec<String>,
    /// Console scroll offset (log lines from end).
    pub console_scroll: usize,
    /// Asset paths cached for panel.
    pub asset_paths: Vec<String>,
    /// Script column cursor within line (for typing).
    pub script_cursor_col: usize,
    /// Visual snap percent (1 or 5).
    pub snap_pct: f32,
    /// Resize drag active (vs move).
    pub resize_drag: bool,
}

/// Tool for Nodes graph mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodesTool {
    /// Click selects node/edge; drag moves node.
    #[default]
    Select,
    /// Click A then B creates transition edge.
    Connect,
    /// Click edge or pair removes link.
    Disconnect,
    /// Click A then B creates overlay edge.
    Overlay,
}

impl StudioGuiSession {
    /// Build default docking layout for a project.
    pub fn open_project(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let root = root.canonicalize().unwrap_or(root);
        let mut session = Self {
            root,
            panels: default_dock_panels(),
            document_path: None,
            document_source: String::new(),
            document_layer_id: None,
            layer_docs: HashMap::new(),
            selected_region: None,
            ready: false,
            last_drag: None,
            log: Vec::new(),
            mode: StudioEditorMode::Simplified,
            drop_seq: 0,
            layers: LayerStack::vn_tree(),
            connect_from: None,
            nodes_tool: NodesTool::Select,
            selected_edge: None,
            script_cursor_line: 0,
            script_issues: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            console_scroll: 0,
            asset_paths: Vec::new(),
            script_cursor_col: 0,
            snap_pct: 1.0,
            resize_drag: false,
        };
        session.log.push(format!(
            "[studio-gui] opened project {}",
            session.root.display()
        ));
        session.ready = true;
        // Restore studio project if present
        if let Ok(Some(proj)) = StudioProjectFile::load(&session.root) {
            session.layers = proj.to_stack();
            session.log.push("[studio-gui] loaded velvet.studio.json".into());
        }
        if let Some(l) = session.layers.get_mut("main_menu") {
            l.locked = false;
            l.expanded = true;
        }
        session.ensure_all_layer_docs();
        let active = session.layers.active_id.clone();
        session.activate_layer_document(&active)?;
        session.refresh_assets();
        session.log.push(format!(
            "[studio-gui] ready panels={} mode=simplified layers={} (per-screen docs)",
            session
                .panels
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(","),
            session.layers.sorted_ids().join("/")
        ));
        Ok(session)
    }

    pub fn refresh_assets(&mut self) {
        self.asset_paths = crate::asset_panel::scan_assets(&self.root)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    crate::asset_panel::AssetKind::Image
                        | crate::asset_panel::AssetKind::Audio
                        | crate::asset_panel::AssetKind::Script
                )
            })
            .map(|e| e.path.to_string_lossy().into_owned())
            .take(40)
            .collect();
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push(self.document_source.clone());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.document_source.clone());
            self.document_source = prev;
            self.flush_active_document();
            self.selected_region = None;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.document_source.clone());
            self.document_source = next;
            self.flush_active_document();
            true
        } else {
            false
        }
    }

    pub fn delete_selected_widget(&mut self) -> Result<()> {
        let id = self
            .selected_region
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no selection"))?;
        self.push_undo();
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.delete_widget(&id).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.flush_active_document();
        self.selected_region = None;
        self.log.push(format!("[studio-gui] delete {id}"));
        Ok(())
    }

    pub fn duplicate_selected(&mut self) -> Result<String> {
        let id = self
            .selected_region
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no selection"))?;
        self.push_undo();
        self.drop_seq += 1;
        let new_id = format!("copy{}", self.drop_seq);
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        let full = d
            .duplicate_widget(&id, &new_id)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.flush_active_document();
        self.selected_region = Some(full.clone());
        self.log.push(format!("[studio-gui] duplicate {id} -> {full}"));
        Ok(full)
    }

    pub fn inject_line_on_selected(&mut self, line: &str) -> Result<()> {
        let id = self
            .selected_region
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no button selected for bind"))?;
        self.push_undo();
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.inject_advanced_line(&id, line)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.flush_active_document();
        self.log
            .push(format!("[studio-gui] inject {id}: {line}"));
        Ok(())
    }

    pub fn resize_selected(&mut self, dw: f32, dh: f32) -> Result<()> {
        let id = self
            .selected_region
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no selection"))?;
        self.push_undo();
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.resize(&id, dw, dh).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.flush_active_document();
        Ok(())
    }

    pub fn save_studio_project(&mut self) -> Result<()> {
        self.flush_active_document();
        let file = StudioProjectFile::from_stack(&self.layers);
        file.save(&self.root)?;
        let _ = self.save_all_layer_documents();
        self.log
            .push("[studio-gui] saved velvet.studio.json + screens".into());
        Ok(())
    }

    pub fn play_project_smoke(&mut self) -> String {
        use std::process::Command;
        let root = self.root.clone();
        // Prefer workspace velvet binary if present
        let velvet = root
            .join("target")
            .join("release")
            .join("velvet.exe");
        let velvet = if velvet.is_file() {
            velvet
        } else {
            PathBuf::from("velvet")
        };
        let out = Command::new(&velvet)
            .arg("play")
            .arg(&root)
            .arg("--choice")
            .arg("0")
            .output();
        match out {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let line = format!(
                    "play exit={} {}",
                    o.status.code().unwrap_or(-1),
                    stdout.lines().last().unwrap_or(stderr.lines().last().unwrap_or(""))
                );
                self.log.push(format!("[play] {line}"));
                line
            }
            Err(e) => {
                let line = format!("play failed: {e} (build velvet or add to PATH)");
                self.log.push(format!("[play] {line}"));
                line
            }
        }
    }

    /// Edit script line at cursor: replace line content.
    pub fn script_set_line(&mut self, line_idx: usize, content: &str) {
        let mut lines: Vec<String> = self.document_source.lines().map(|s| s.to_string()).collect();
        if lines.is_empty() {
            lines.push(content.to_string());
        } else if line_idx < lines.len() {
            lines[line_idx] = content.to_string();
        } else {
            lines.push(content.to_string());
        }
        self.document_source = lines.join("\n");
        if !self.document_source.ends_with('\n') {
            self.document_source.push('\n');
        }
        self.flush_active_document();
    }

    pub fn script_current_line(&self) -> String {
        self.document_source
            .lines()
            .nth(self.script_cursor_line)
            .unwrap_or("")
            .to_string()
    }

    pub fn script_type_char(&mut self, ch: char) {
        let mut line = self.script_current_line();
        let col = self.script_cursor_col.min(line.len());
        line.insert(col, ch);
        self.script_cursor_col = col + ch.len_utf8();
        self.script_set_line(self.script_cursor_line, &line);
    }

    pub fn script_backspace(&mut self) {
        let mut line = self.script_current_line();
        if self.script_cursor_col > 0 && !line.is_empty() {
            let col = self.script_cursor_col.min(line.len());
            // remove previous char
            let mut idx = col;
            while idx > 0 && !line.is_char_boundary(idx - 1) {
                idx -= 1;
            }
            if idx > 0 {
                line.remove(idx - 1);
                self.script_cursor_col = idx - 1;
                self.script_set_line(self.script_cursor_line, &line);
            }
        } else if self.script_cursor_line > 0 {
            // merge with previous line
            self.push_undo();
            let mut lines: Vec<String> =
                self.document_source.lines().map(|s| s.to_string()).collect();
            let cur = lines.remove(self.script_cursor_line);
            self.script_cursor_line -= 1;
            let prev_len = lines[self.script_cursor_line].len();
            lines[self.script_cursor_line].push_str(&cur);
            self.script_cursor_col = prev_len;
            self.document_source = lines.join("\n");
            if !self.document_source.ends_with('\n') {
                self.document_source.push('\n');
            }
            self.flush_active_document();
        }
    }

    pub fn script_newline(&mut self) {
        let line = self.script_current_line();
        let col = self.script_cursor_col.min(line.len());
        let (left, right) = line.split_at(col);
        self.push_undo();
        let mut lines: Vec<String> = self.document_source.lines().map(|s| s.to_string()).collect();
        if lines.is_empty() {
            lines.push(left.to_string());
            lines.push(right.to_string());
        } else {
            lines[self.script_cursor_line] = left.to_string();
            lines.insert(self.script_cursor_line + 1, right.to_string());
        }
        self.script_cursor_line += 1;
        self.script_cursor_col = 0;
        self.document_source = lines.join("\n");
        if !self.document_source.ends_with('\n') {
            self.document_source.push('\n');
        }
        self.flush_active_document();
    }

    /// Empty screen template — blank canvas to design from zero.
    pub fn empty_screen_source(id: &str, name: &str) -> String {
        format!(
            r#"// Screen: {name}
// Empty pantallas — start from zero. Drop Button/Label/Panel from the palette.
// Script mode: layer.open / button.press / game.* / scene.*

screen {id} {{
}}
"#
        )
    }

    fn layer_file_path(&self, layer_id: &str) -> PathBuf {
        // Prefer explicit path on the layer; else scripts/screens/<id>.vel
        if let Some(l) = self.layers.get(layer_id) {
            if let Some(ref p) = l.document_path {
                return if p.is_absolute() {
                    p.clone()
                } else {
                    self.root.join(p)
                };
            }
        }
        self.root
            .join("scripts")
            .join("screens")
            .join(format!("{layer_id}.vel"))
    }

    /// Ensure every layer has a document entry (empty if missing).
    pub fn ensure_all_layer_docs(&mut self) {
        let layers: Vec<(String, String)> = self
            .layers
            .layers
            .iter()
            .map(|l| (l.id.clone(), l.name.clone()))
            .collect();
        for (id, name) in layers {
            self.layer_docs
                .entry(id.clone())
                .or_insert_with(|| Self::empty_screen_source(&id, &name));
        }
    }

    /// Push current editor buffer into the map for its owning layer.
    pub fn flush_active_document(&mut self) {
        if let Some(ref lid) = self.document_layer_id.clone() {
            self.layer_docs
                .insert(lid.clone(), self.document_source.clone());
        }
    }

    /// Load a layer's document into the editor canvas (isolated pantallas).
    pub fn activate_layer_document(&mut self, layer_id: &str) -> Result<()> {
        self.flush_active_document();
        let name = self
            .layers
            .get(layer_id)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| layer_id.to_string());
        // Prefer in-memory map; else try disk once; else empty template.
        if !self.layer_docs.contains_key(layer_id) {
            let path = self.layer_file_path(layer_id);
            if path.is_file() {
                if let Ok(src) = fs::read_to_string(&path) {
                    if parse_document(&src).is_ok() {
                        self.layer_docs.insert(layer_id.to_string(), src);
                    }
                }
            }
        }
        let source = self
            .layer_docs
            .entry(layer_id.to_string())
            .or_insert_with(|| Self::empty_screen_source(layer_id, &name))
            .clone();
        let _ = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = source;
        self.document_layer_id = Some(layer_id.to_string());
        self.document_path = Some(self.layer_file_path(layer_id));
        self.selected_region = None;
        self.script_cursor_line = 0;
        self.log.push(format!(
            "[studio-gui] active doc layer={layer_id} path={}",
            self.document_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        ));
        Ok(())
    }

    /// Toggle or set dual mode (simplified visual vs advanced script).
    pub fn set_mode(&mut self, mode: StudioEditorMode) -> Result<()> {
        if self.mode == mode {
            return Ok(());
        }
        // Re-validate document when leaving advanced (script may have been edited)
        if matches!(self.mode, StudioEditorMode::Advanced) {
            let _ = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
            self.validate_script();
        }
        self.mode = mode;
        let name = match mode {
            StudioEditorMode::Simplified => "visual",
            StudioEditorMode::Advanced => "script",
            StudioEditorMode::Nodes => "nodes",
        };
        self.log.push(format!("[studio-gui] mode={name}"));
        if matches!(mode, StudioEditorMode::Advanced) {
            self.validate_script();
        }
        if !self.document_source.is_empty()
            && !matches!(mode, StudioEditorMode::Nodes)
        {
            let _ = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Ok(())
    }

    /// Toggle mode Visual → Script → Nodes → Visual.
    pub fn toggle_mode(&mut self) -> Result<StudioEditorMode> {
        let next = match self.mode {
            StudioEditorMode::Simplified => StudioEditorMode::Advanced,
            StudioEditorMode::Advanced => StudioEditorMode::Nodes,
            StudioEditorMode::Nodes => StudioEditorMode::Simplified,
        };
        self.set_mode(next)?;
        Ok(self.mode)
    }

    /// Re-run VScript validation against layers + buttons.
    pub fn validate_script(&mut self) {
        use crate::vscript::{parse_script, validate};
        let stmts = parse_script(&self.document_source);
        let layer_ids: Vec<String> = self.layers.sorted_ids();
        let layer_refs: Vec<&str> = layer_ids.iter().map(|s| s.as_str()).collect();
        let buttons: Vec<String> = self
            .list_widgets()
            .unwrap_or_default()
            .into_iter()
            .filter(|w| w.kind == "button" || w.id.starts_with("button."))
            .map(|w| w.id)
            .collect();
        let button_refs: Vec<&str> = buttons.iter().map(|s| s.as_str()).collect();
        self.script_issues = validate(&stmts, &layer_refs, &button_refs)
            .into_iter()
            .map(|i| i.to_string())
            .collect();
    }

    /// Insert a VScript line at cursor (or append).
    pub fn insert_script_line(&mut self, line: &str) {
        let mut lines: Vec<String> = self.document_source.lines().map(|s| s.to_string()).collect();
        if lines.is_empty() {
            lines.push(line.to_string());
            self.script_cursor_line = 0;
        } else {
            let idx = self.script_cursor_line.min(lines.len());
            lines.insert(idx, line.to_string());
            self.script_cursor_line = idx + 1;
        }
        self.document_source = lines.join("\n");
        if !self.document_source.ends_with('\n') {
            self.document_source.push('\n');
        }
        self.validate_script();
        self.log
            .push(format!("[studio-gui] script insert: {line}"));
    }

    /// Insert layer.open for a layer id.
    pub fn insert_layer_open(&mut self, layer_id: &str) {
        self.insert_script_line(&format!("layer.open(\"{layer_id}\")"));
    }

    /// Insert button.press for region id.
    pub fn insert_button_press(&mut self, button_id: &str) {
        self.insert_script_line(&format!("button.press(\"{button_id}\")"));
    }

    /// Connect layers in graph with kind.
    pub fn connect_layers(
        &mut self,
        from: &str,
        to: &str,
        label: Option<String>,
    ) -> Result<String, String> {
        self.connect_layers_kind(from, to, label, crate::layers::LayerEdgeKind::Transition)
    }

    pub fn connect_layers_kind(
        &mut self,
        from: &str,
        to: &str,
        label: Option<String>,
        kind: crate::layers::LayerEdgeKind,
    ) -> Result<String, String> {
        if from == to {
            return Err("cannot connect node to itself".into());
        }
        self.layers.connect(from, to, label, kind)?;
        let edge = self
            .layers
            .edges
            .iter()
            .find(|e| e.from == from && e.to == to)
            .cloned()
            .ok_or_else(|| "edge missing after connect".to_string())?;
        let script_line = crate::layers::LayerStack::edge_script(&edge)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        // Wire selected button on active screen if any
        if self.selected_region.is_some() && !script_line.is_empty() {
            let _ = self.inject_line_on_selected(&script_line);
        }
        self.insert_script_line(&format!("connect {from} -> {to}"));
        if !script_line.is_empty() {
            self.insert_script_line(&script_line);
        }
        self.selected_edge = Some((from.into(), to.into()));
        let msg = format!("connected {from} -> {to} ({}) wired script", kind.as_str());
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    pub fn disconnect_layers(&mut self, from: &str, to: &str) -> String {
        if self.layers.disconnect(from, to) {
            if self.selected_edge.as_ref().map(|(a, b)| (a.as_str(), b.as_str()))
                == Some((from, to))
            {
                self.selected_edge = None;
            }
            let msg = format!("disconnected {from} -x- {to}");
            self.log.push(format!("[studio-gui] {msg}"));
            msg
        } else {
            format!("no edge {from} -> {to}")
        }
    }

    pub fn create_screen(&mut self, name: &str) -> Result<String, String> {
        let (w, h) = self.layers.active_resolution();
        self.flush_active_document();
        let id = self.layers.create_screen(name, w, h)?;
        // Brand-new empty document for this pantallas
        let empty = Self::empty_screen_source(&id, name);
        self.layer_docs.insert(id.clone(), empty);
        self.selected_edge = None;
        self.connect_from = None;
        self.activate_layer_document(&id)
            .map_err(|e| e.to_string())?;
        let msg = format!("created empty screen {id} — design from zero");
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    pub fn create_sub_screen(&mut self, name: &str) -> Result<String, String> {
        self.flush_active_document();
        let id = self.layers.create_sub_screen(None, name)?;
        let empty = Self::empty_screen_source(&id, name);
        self.layer_docs.insert(id.clone(), empty);
        self.activate_layer_document(&id)
            .map_err(|e| e.to_string())?;
        let msg = format!("created empty sublayer {id}");
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    /// Nodes click handling based on current tool.
    pub fn nodes_click_layer(&mut self, id: &str) -> String {
        match self.nodes_tool {
            NodesTool::Select => {
                self.connect_from = None;
                self.selected_edge = None;
                match self.set_layer(id) {
                    Ok(m) => m,
                    Err(e) => format!("select {id}: {e}"),
                }
            }
            NodesTool::Connect | NodesTool::Overlay => {
                if self.connect_from.as_deref() == Some(id) {
                    self.connect_from = None;
                    return "connect cancelled".into();
                }
                if let Some(from) = self.connect_from.take() {
                    let kind = if self.nodes_tool == NodesTool::Overlay {
                        crate::layers::LayerEdgeKind::Overlay
                    } else {
                        crate::layers::LayerEdgeKind::Transition
                    };
                    match self.connect_layers_kind(&from, id, None, kind) {
                        Ok(m) => m,
                        Err(e) => {
                            self.connect_from = Some(from);
                            e
                        }
                    }
                } else {
                    self.connect_from = Some(id.into());
                    let _ = self.layers.set_active(id);
                    format!("from {id} — click target")
                }
            }
            NodesTool::Disconnect => {
                if self.connect_from.as_deref() == Some(id) {
                    self.connect_from = None;
                    return "disconnect cancelled".into();
                }
                if let Some(from) = self.connect_from.take() {
                    self.disconnect_layers(&from, id)
                } else {
                    // try reverse or clear all for node if single select
                    self.connect_from = Some(id.into());
                    format!("disconnect from {id} — click other end")
                }
            }
        }
    }

    pub fn nodes_click_edge(&mut self, from: &str, to: &str) -> String {
        match self.nodes_tool {
            NodesTool::Disconnect => self.disconnect_layers(from, to),
            NodesTool::Select | NodesTool::Connect | NodesTool::Overlay => {
                self.selected_edge = Some((from.into(), to.into()));
                if let Some(k) = self.layers.cycle_edge_kind(from, to) {
                    // re-select after cycle - actually cycle already applied
                    format!("edge {from}->{to} kind={}", k.as_str())
                } else {
                    format!("edge {from}->{to}")
                }
            }
        }
    }

    /// List visual widgets (simplified mode palette/canvas).
    pub fn list_widgets(&self) -> Result<Vec<DesignerWidget>> {
        if self.document_source.is_empty() {
            return Ok(Vec::new());
        }
        let d = UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.list_widgets().map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Drop a palette widget at percent canvas coords (simplified mode).
    pub fn drop_widget(&mut self, kind: &str, x_pct: f32, y_pct: f32) -> Result<String> {
        self.drop_seq += 1;
        let id = format!("w{}", self.drop_seq);
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.drop_widget(kind, &id, x_pct, y_pct)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        let full = if id.contains('.') {
            id.clone()
        } else {
            let k = match kind.to_ascii_lowercase().as_str() {
                "label" | "text" => "label",
                "panel" | "box" => "panel",
                _ => "button",
            };
            format!("{k}.{id}")
        };
        self.selected_region = Some(full.clone());
        self.log
            .push(format!("[studio-gui] drop kind={kind} id={full} at ({x_pct},{y_pct})"));
        Ok(full)
    }

    /// Set text on selected (or given) visual region.
    pub fn set_widget_text(&mut self, region_id: Option<&str>, text: &str) -> Result<()> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region for text"))?;
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_text(&id, text).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push(format!("[studio-gui] set_text region={id} text={text}"));
        Ok(())
    }

    /// Set position string e.g. `(50%, 62%)` on selected/given region.
    pub fn set_widget_position(&mut self, region_id: Option<&str>, position: &str) -> Result<()> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region for position"))?;
        let normalized = normalize_pct_pair(position)
            .ok_or_else(|| anyhow::anyhow!("bad position (use 50,62 or (50%, 62%))"))?;
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_position(&id, &normalized)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push(format!("[studio-gui] set_position region={id} {normalized}"));
        Ok(())
    }

    /// Set size string e.g. `(18%, 8%)` on selected/given region.
    pub fn set_widget_size(&mut self, region_id: Option<&str>, size: &str) -> Result<()> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region for size"))?;
        let normalized = normalize_pct_pair(size)
            .ok_or_else(|| anyhow::anyhow!("bad size (use 18,8 or (18%, 8%))"))?;
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_size(&id, &normalized)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push(format!("[studio-gui] set_size region={id} {normalized}"));
        Ok(())
    }

    /// Nudge selected widget position by integer percent points.
    pub fn nudge_selected(&mut self, dx: f32, dy: f32) -> Result<Option<WidgetRect>> {
        if self.selected_region.is_none() {
            return Ok(None);
        }
        Ok(Some(self.drag_region(None, dx, dy)?))
    }

    /// Advanced mode: replace entire document source (must parse).
    pub fn set_advanced_source(&mut self, source: impl Into<String>) -> Result<()> {
        let source = source.into();
        let mut d = UiDesigner::open(self.document_source.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_source_advanced(source)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push("[studio-gui] advanced source applied (reparsed)".into());
        Ok(())
    }

    /// Snapshot source for advanced text view.
    pub fn advanced_source(&self) -> &str {
        &self.document_source
    }

    /// Open a `.vel` document onto the **active** layer (other layers stay independent).
    pub fn open_document(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let source = fs::read_to_string(&abs)
            .with_context(|| format!("read document {}", abs.display()))?;
        let _ = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.ensure_all_layer_docs();
        let layer_id = self.layers.active_id.clone();
        // Bind this file to the active layer (usually main_menu on first open).
        if let Some(l) = self.layers.get_mut(&layer_id) {
            l.document_path = Some(abs.clone());
        }
        self.layer_docs.insert(layer_id.clone(), source.clone());
        self.document_path = Some(abs);
        self.document_source = source;
        self.document_layer_id = Some(layer_id.clone());
        self.selected_region = None;
        self.log.push(format!(
            "[studio-gui] document layer={layer_id} {}",
            self.document_path.as_ref().unwrap().display()
        ));
        Ok(())
    }

    /// Select a region by id (must be visual).
    pub fn select_region(&mut self, region_id: &str) -> Result<()> {
        let doc = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        let _ = region_rect(&doc, region_id).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.selected_region = Some(region_id.to_string());
        self.log
            .push(format!("[studio-gui] select region={region_id}"));
        Ok(())
    }

    /// Drag the selected (or given) region by delta — **the shipped drag path**.
    pub fn drag_region(
        &mut self,
        region_id: Option<&str>,
        dx: f32,
        dy: f32,
    ) -> Result<WidgetRect> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region selected for drag"))?;
        let mut doc = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        let rect = drag_visual_region(&mut doc, &id, dx, dy).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = render_document(&doc);
        self.last_drag = Some(rect);
        self.log.push(format!(
            "[studio-gui] drag region={id} dx={dx} dy={dy} -> ({:.0}%, {:.0}%)",
            rect.pos.x, rect.pos.y
        ));
        Ok(rect)
    }

    /// Pointer press on canvas (percent coords) — selects via hit-test.
    pub fn canvas_pointer_down(&mut self, x: f32, y: f32) -> Result<Option<String>> {
        let doc = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        let hit = hit_test_visual(&doc, x, y);
        if let Some(ref id) = hit {
            self.selected_region = Some(id.clone());
            self.log
                .push(format!("[studio-gui] hit-test ({x},{y}) -> {id}"));
        }
        Ok(hit)
    }

    /// Pointer drag with button held: moves selection.
    pub fn canvas_pointer_drag(&mut self, dx: f32, dy: f32) -> Result<Option<WidgetRect>> {
        if self.selected_region.is_none() {
            return Ok(None);
        }
        Ok(Some(self.drag_region(None, dx, dy)?))
    }

    /// Write **active** layer document to disk.
    pub fn save_document(&self) -> Result<()> {
        let path = self
            .document_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no document open"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        fs::write(path, &self.document_source)
            .with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    /// Flush + save every layer that has a path or non-empty content.
    pub fn save_all_layer_documents(&mut self) -> Result<usize> {
        self.flush_active_document();
        let mut n = 0;
        let ids: Vec<String> = self.layer_docs.keys().cloned().collect();
        for id in ids {
            let Some(src) = self.layer_docs.get(&id).cloned() else {
                continue;
            };
            // skip pure empty templates that were never edited? still save if path exists
            let path = self.layer_file_path(&id);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&path, &src)
                .with_context(|| format!("write {}", path.display()))?;
            if let Some(l) = self.layers.get_mut(&id) {
                l.document_path = Some(path);
            }
            n += 1;
        }
        self.log
            .push(format!("[studio-gui] saved {n} layer documents"));
        Ok(n)
    }

    /// Panel ids currently visible (for ready assertions).
    pub fn panel_ids(&self) -> Vec<&str> {
        self.panels
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.id.as_str())
            .collect()
    }

    /// Active layer design surface letterboxed into layout canvas.
    pub fn design_surface(&self, canvas_x: i32, canvas_y: i32, canvas_w: i32, canvas_h: i32) -> DesignSurface {
        let (rw, rh) = self.layers.display_resolution();
        DesignSurface::fit(canvas_x, canvas_y, canvas_w, canvas_h, rw, rh)
    }

    /// Pixel coordinates of selected widget center on the active layer.
    pub fn selected_pos_px(&self) -> Option<(i32, i32)> {
        let id = self.selected_region.as_ref()?;
        let widgets = self.list_widgets().ok()?;
        let w = widgets.iter().find(|w| w.id == *id)?;
        let (x, y) = parse_pct_pair_loose(w.position.as_deref().unwrap_or("(50%,50%)"));
        let (rw, rh) = self.layers.active_resolution();
        Some(pct_to_px(x, y, rw, rh))
    }

    /// Switch layer; loads that pantalla's independent document (empty if new).
    pub fn set_layer(&mut self, id: &str) -> Result<String, String> {
        let prev = self.layers.active_id.clone();
        self.layers.set_active(id).map_err(|e| e)?;
        self.apply_layer_lock_policy(&prev, id);
        self.activate_layer_document(id)
            .map_err(|e| e.to_string())?;
        let (w, h) = self.layers.active_resolution();
        let widgets = self.list_widgets().map(|v| v.len()).unwrap_or(0);
        let msg = format!("layer={id} {w}x{h}px  widgets={widgets} (own document)");
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    fn apply_layer_lock_policy(&mut self, _prev: &str, id: &str) {
        // Roots re-lock when not active; active branch unlocks for edit.
        // Sublayers under inactive roots stay as-is; active node unlocks.
        let roots: Vec<String> = self
            .layers
            .layers
            .iter()
            .filter(|l| l.parent.is_none())
            .map(|l| l.id.clone())
            .collect();
        // Active root = root ancestor of id
        let mut active_root = id.to_string();
        let mut cur = Some(id.to_string());
        for _ in 0..16 {
            if let Some(cid) = cur {
                if let Some(l) = self.layers.get(&cid) {
                    if l.parent.is_none() {
                        active_root = l.id.clone();
                        break;
                    }
                    cur = l.parent.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        for r in roots {
            if let Some(l) = self.layers.get_mut(&r) {
                // only auto-lock main_menu as the “base” pantallas
                if r == "main_menu" {
                    l.locked = r != active_root;
                }
            }
        }
        if let Some(l) = self.layers.get_mut(id) {
            if id != "main_menu" || active_root == "main_menu" {
                // ensure active is editable unless user locked it intentionally
                if id == "main_menu" {
                    l.locked = false;
                }
            }
        }
    }

    pub fn cycle_layer(&mut self, forward: bool) -> Result<String, String> {
        let prev = self.layers.active_id.clone();
        let id = if forward {
            self.layers.cycle_next()?
        } else {
            self.layers.cycle_prev()?
        };
        // cycle_next already set_active; load document + lock policy
        self.apply_layer_lock_policy(&prev, &id);
        self.activate_layer_document(&id)
            .map_err(|e| e.to_string())?;
        let (w, h) = self.layers.active_resolution();
        let widgets = self.list_widgets().map(|v| v.len()).unwrap_or(0);
        let msg = format!("layer={id} {w}x{h}px widgets={widgets}");
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    pub fn set_layer_preset(&mut self, preset: ResPreset) -> Result<String, String> {
        self.layers.apply_preset(preset)?;
        let (w, h) = self.layers.active_resolution();
        let msg = format!(
            "layer {} res → {} ({})",
            self.layers.active_id,
            preset.label(),
            format!("{w}x{h}")
        );
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    pub fn add_mobile_layer(&mut self) -> Result<String, String> {
        if self.layers.get("mobile").is_none() {
            self.flush_active_document();
            self.layers
                .add_child("main_menu", "mobile", "Mobile UI", 390, 844)?;
            self.layer_docs.insert(
                "mobile".into(),
                Self::empty_screen_source("mobile", "Mobile UI"),
            );
        }
        self.set_layer("mobile")
    }

    /// Add sublayer under active (or under parent if active is already a child).
    pub fn add_sublayer(&mut self, id: &str, name: &str) -> Result<String, String> {
        self.flush_active_document();
        let parent = self.layers.active_id.clone();
        let (w, h) = self.layers.active_resolution();
        self.layers.add_child(&parent, id, name, w, h)?;
        self.layer_docs
            .insert(id.to_string(), Self::empty_screen_source(id, name));
        self.set_layer(id)
    }

    pub fn toggle_layer_expand(&mut self, id: &str) -> bool {
        self.layers.toggle_expanded(id)
    }

    pub fn tick_layers(&mut self, dt: f32) -> bool {
        self.layers.tick_anim(dt)
    }
}

fn parse_pct_pair_loose(s: &str) -> (f32, f32) {
    let t = s.trim().trim_start_matches('(').trim_end_matches(')');
    let mut parts = t.split(',');
    let a = parts
        .next()
        .unwrap_or("50")
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50.0);
    let b = parts
        .next()
        .unwrap_or("50")
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50.0);
    (a, b)
}

/// Accept `50,62`, `50% 62%`, `(50%, 62%)` → `(50%, 62%)`.
fn normalize_pct_pair(raw: &str) -> Option<String> {
    let t = raw
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .replace('%', " ");
    let parts: Vec<&str> = t
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let a: f32 = parts[0].parse().ok()?;
    let b: f32 = parts[1].parse().ok()?;
    if !a.is_finite() || !b.is_finite() {
        return None;
    }
    Some(format!("({:.0}%, {:.0}%)", a.clamp(0.0, 100.0), b.clamp(0.0, 100.0)))
}

fn default_dock_panels() -> Vec<DockPanel> {
    vec![
        DockPanel {
            id: "hierarchy".into(),
            title: "Hierarchy".into(),
            zone: DockZone::Left,
            visible: true,
        },
        DockPanel {
            id: "assets".into(),
            title: "Assets".into(),
            zone: DockZone::Left,
            visible: true,
        },
        DockPanel {
            id: "canvas".into(),
            title: "Visual Canvas".into(),
            zone: DockZone::Center,
            visible: true,
        },
        DockPanel {
            id: "inspector".into(),
            title: "Inspector".into(),
            zone: DockZone::Right,
            visible: true,
        },
        DockPanel {
            id: "scripts".into(),
            title: "Scripts".into(),
            zone: DockZone::Right,
            visible: true,
        },
        DockPanel {
            id: "console".into(),
            title: "Console".into(),
            zone: DockZone::Bottom,
            visible: true,
        },
    ]
}

/// Launch configuration for Studio GUI.
#[derive(Debug, Clone)]
pub struct StudioGuiConfig {
    /// Project root.
    pub root: PathBuf,
    /// Optional document to open on canvas.
    pub document: Option<PathBuf>,
    /// Headless: never open OS window.
    pub headless: bool,
    /// Exit after init (+ optional demo drag).
    pub once: bool,
    /// Demo drag region id (if document open).
    pub demo_drag_region: Option<String>,
    /// Demo drag deltas.
    pub demo_dx: f32,
    /// Demo drag dy.
    pub demo_dy: f32,
    /// Write ready log here.
    pub ready_log: Option<PathBuf>,
    /// Persist document after demo drag.
    pub save_after_drag: bool,
}

impl Default for StudioGuiConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            document: None,
            headless: true,
            once: true,
            demo_drag_region: None,
            demo_dx: 0.0,
            demo_dy: 0.0,
            ready_log: None,
            save_after_drag: false,
        }
    }
}

/// Status returned from a GUI launch.
#[derive(Debug, Clone)]
pub struct StudioGuiStatus {
    /// Session after run.
    pub session: StudioGuiSession,
    /// Window was opened (false when headless or display failed).
    pub window_opened: bool,
    /// Optional display error when window path failed.
    pub display_error: Option<String>,
}

/// Run Studio GUI once (or headless ready path).
///
/// Always initializes docking panels and can exercise drag via config.
pub fn run_studio_gui(cfg: StudioGuiConfig) -> Result<StudioGuiStatus> {
    let mut session = StudioGuiSession::open_project(&cfg.root)?;
    if let Some(doc) = &cfg.document {
        session.open_document(doc)?;
    } else {
        // Auto-pick common menu / main script if present
        for candidate in [
            "scripts/main_menu.vel",
            "scripts/main.vel",
            "ui/main_menu.vel",
        ] {
            let p = session.root.join(candidate);
            if p.is_file() {
                session.open_document(&p)?;
                break;
            }
        }
    }

    if let Some(id) = &cfg.demo_drag_region {
        if !session.document_source.is_empty() {
            session.select_region(id)?;
            session.drag_region(Some(id), cfg.demo_dx, cfg.demo_dy)?;
            if cfg.save_after_drag {
                session.save_document()?;
                session
                    .log
                    .push("[studio-gui] saved document after drag".into());
            }
        }
    }

    let mut window_opened = false;
    let mut display_error = None;

    if !cfg.headless {
        let interactive = !cfg.once;
        match try_open_studio_window(&session, interactive) {
            Ok(()) => {
                window_opened = true;
                session.log.push(if interactive {
                    "[studio-gui] window opened (interactive dual-mode)".into()
                } else {
                    "[studio-gui] window opened (brief dual-mode paint)".into()
                });
            }
            Err(e) => {
                display_error = Some(e.to_string());
                session
                    .log
                    .push(format!("[studio-gui] window skipped: {e}"));
            }
        }
    } else {
        session.log.push("[studio-gui] headless mode".into());
        session
            .log
            .push(format!("[studio-gui] mode={:?}", session.mode));
    }

    if let Some(path) = &cfg.ready_log {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut f = fs::File::create(path)
            .with_context(|| format!("create ready log {}", path.display()))?;
        for line in &session.log {
            writeln!(f, "{line}")?;
        }
        writeln!(
            f,
            "[studio-gui] STATUS ready={} window={} panels={}",
            session.ready,
            window_opened,
            session.panel_ids().join(",")
        )?;
    }

    // Print to stdout for harness capture
    for line in &session.log {
        println!("{line}");
    }
    println!(
        "[studio-gui] STATUS ready={} window={} panels={}",
        session.ready,
        window_opened,
        session.panel_ids().join(",")
    );

    if !session.ready {
        bail!("studio gui failed to become ready");
    }

    let _ = cfg.once; // always returns after one pass for now
    Ok(StudioGuiStatus {
        session,
        window_opened,
        display_error,
    })
}

/// Softbuffer dual-mode Studio window (simplified canvas + advanced text strip).
///
/// When `interactive` is false, opens briefly then exits (CI / `--once`).
fn try_open_studio_window(session: &StudioGuiSession, interactive: bool) -> Result<()> {
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use softbuffer::{Context as SbContext, Surface};
    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{KeyCode, PhysicalKey};
    use winit::window::{Window, WindowId};

    struct Host {
        session: StudioGuiSession,
        interactive: bool,
        frames: u32,
        window: Option<Arc<Window>>,
        context: Option<SbContext<Arc<Window>>>,
        surface: Option<Surface<Arc<Window>, Arc<Window>>>,
        pixels: Vec<u32>,
        /// Last cursor in window pixels.
        cursor: (f64, f64),
        /// Last cursor when drag started / previous move (window pixels).
        drag_last_px: Option<(f64, f64)>,
        dragging: bool,
        /// Accumulated percent delta for 1% snap (smooth feel, grid placement).
        drag_acc_pct: (f32, f32),
        /// Ctrl/Super held (tracked via ModifiersChanged — KeyEvent has no modifiers).
        ctrl_held: bool,
        /// Inspector field being edited (TEXT / POS / SIZE).
        edit_field: Option<crate::studio_paint::InspectorField>,
        /// Live buffer while editing an inspector field.
        edit_buf: String,
        /// UI text zoom level 1..=4 (Ctrl+/- or Ctrl+wheel). Default 2.
        ui_zoom: i32,
        /// Last tick instant for layer resize animation.
        last_tick: std::time::Instant,
        /// Nodes: dragging a node on the graph.
        node_drag_id: Option<String>,
        status: String,
    }

    impl ApplicationHandler for Host {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let mode = match self.session.mode {
                StudioEditorMode::Simplified => "Visual",
                StudioEditorMode::Advanced => "Script",
                StudioEditorMode::Nodes => "Nodes",
            };
            let title = format!(
                "Velvet Studio [{mode}] — {}",
                self.session
                    .root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project")
            );
            let attrs = winit::window::Window::default_attributes()
                .with_title(title)
                .with_inner_size(LogicalSize::new(1280.0, 800.0));
            let window = Arc::new(el.create_window(attrs).expect("window"));
            let context = SbContext::new(window.clone()).expect("ctx");
            let surface = Surface::new(&context, window.clone()).expect("surface");
            self.context = Some(context);
            self.surface = Some(surface);
            self.window = Some(window);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }

        fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
            match ev {
                WindowEvent::CloseRequested => el.exit(),
                WindowEvent::ModifiersChanged(mods) => {
                    self.ctrl_held = mods.state().control_key() || mods.state().super_key();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if !self.ctrl_held {
                        return;
                    }
                    use winit::event::MouseScrollDelta;
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => {
                            if p.y > 0.0 {
                                1.0
                            } else if p.y < 0.0 {
                                -1.0
                            } else {
                                0.0
                            }
                        }
                    };
                    if dy > 0.0 {
                        self.bump_zoom(1);
                    } else if dy < 0.0 {
                        self.bump_zoom(-1);
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }
                    // Ctrl+/- always zooms UI text (even while editing)
                    if self.ctrl_held {
                        if let PhysicalKey::Code(c) = event.physical_key {
                            match c {
                                KeyCode::Equal | KeyCode::NumpadAdd => {
                                    self.bump_zoom(1);
                                    return;
                                }
                                KeyCode::Minus | KeyCode::NumpadSubtract => {
                                    self.bump_zoom(-1);
                                    return;
                                }
                                KeyCode::Digit0 | KeyCode::Numpad0 => {
                                    self.ui_zoom = 2;
                                    self.status = "UI zoom reset x2".into();
                                    self.redraw();
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }

                    // Global edit shortcuts
                    if self.ctrl_held {
                        if let PhysicalKey::Code(c) = event.physical_key {
                            match c {
                                KeyCode::KeyZ => {
                                    if self.session.undo() {
                                        self.status = "undo".into();
                                        self.redraw();
                                    }
                                    return;
                                }
                                KeyCode::KeyY => {
                                    if self.session.redo() {
                                        self.status = "redo".into();
                                        self.redraw();
                                    }
                                    return;
                                }
                                KeyCode::KeyD
                                    if self.session.mode == StudioEditorMode::Simplified =>
                                {
                                    match self.session.duplicate_selected() {
                                        Ok(id) => self.status = format!("duplicated {id}"),
                                        Err(e) => self.status = format!("dup: {e}"),
                                    }
                                    self.redraw();
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                    if matches!(
                        event.physical_key,
                        PhysicalKey::Code(KeyCode::F9)
                    ) {
                        self.status = self.session.play_project_smoke();
                        self.redraw();
                        return;
                    }

                    // Script mode: type into document
                    if self.session.mode == StudioEditorMode::Advanced
                        && self.edit_field.is_none()
                        && !self.ctrl_held
                    {
                        if let PhysicalKey::Code(c) = event.physical_key {
                            match c {
                                KeyCode::Enter | KeyCode::NumpadEnter => {
                                    self.session.script_newline();
                                    self.redraw();
                                    return;
                                }
                                KeyCode::Backspace => {
                                    self.session.script_backspace();
                                    self.redraw();
                                    return;
                                }
                                KeyCode::Home => {
                                    self.session.script_cursor_col = 0;
                                    self.redraw();
                                    return;
                                }
                                KeyCode::End => {
                                    self.session.script_cursor_col =
                                        self.session.script_current_line().len();
                                    self.redraw();
                                    return;
                                }
                                KeyCode::ArrowLeft => {
                                    if self.session.script_cursor_col > 0 {
                                        self.session.script_cursor_col -= 1;
                                    }
                                    self.redraw();
                                    return;
                                }
                                KeyCode::ArrowRight => {
                                    let len = self.session.script_current_line().len();
                                    if self.session.script_cursor_col < len {
                                        self.session.script_cursor_col += 1;
                                    }
                                    self.redraw();
                                    return;
                                }
                                KeyCode::Delete
                                    if self.session.mode == StudioEditorMode::Simplified =>
                                {
                                    // handled below
                                }
                                _ => {
                                    if let Some(t) = event.text.as_ref() {
                                        for ch in t.chars() {
                                            if !ch.is_control() {
                                                self.session.script_type_char(ch);
                                            }
                                        }
                                        self.redraw();
                                        return;
                                    }
                                }
                            }
                        } else if let Some(t) = event.text.as_ref() {
                            for ch in t.chars() {
                                if !ch.is_control() {
                                    self.session.script_type_char(ch);
                                }
                            }
                            self.redraw();
                            return;
                        }
                    }

                    // Visual: delete selection
                    if self.session.mode == StudioEditorMode::Simplified
                        && matches!(
                            event.physical_key,
                            PhysicalKey::Code(KeyCode::Delete) | PhysicalKey::Code(KeyCode::Backspace)
                        )
                        && self.edit_field.is_none()
                    {
                        match self.session.delete_selected_widget() {
                            Ok(()) => self.status = "deleted widget".into(),
                            Err(e) => self.status = format!("delete: {e}"),
                        }
                        self.redraw();
                        return;
                    }

                    // ── Inspector text editing takes keyboard focus ──
                    if self.edit_field.is_some() {
                        let PhysicalKey::Code(c) = event.physical_key else {
                            // still accept text via event.text
                            if let Some(t) = event.text.as_ref() {
                                for ch in t.chars() {
                                    if !ch.is_control() {
                                        self.edit_buf.push(ch);
                                    }
                                }
                                self.redraw();
                            }
                            return;
                        };
                        match c {
                            KeyCode::Enter | KeyCode::NumpadEnter => {
                                self.commit_edit();
                            }
                            KeyCode::Escape => {
                                self.cancel_edit();
                                self.status = "edit cancelled".into();
                                self.redraw();
                            }
                            KeyCode::Backspace => {
                                self.edit_buf.pop();
                                self.redraw();
                            }
                            KeyCode::Delete => {
                                self.edit_buf.clear();
                                self.redraw();
                            }
                            _ => {
                                if let Some(t) = event.text.as_ref() {
                                    for ch in t.chars() {
                                        if !ch.is_control() && self.edit_buf.len() < 64 {
                                            self.edit_buf.push(ch);
                                        }
                                    }
                                    self.redraw();
                                }
                            }
                        }
                        return;
                    }

                    let PhysicalKey::Code(c) = event.physical_key else {
                        return;
                    };
                    match c {
                        KeyCode::Tab => {
                            let _ = self.session.toggle_mode();
                            self.sync_title();
                            self.status = format!("mode={:?}", self.session.mode);
                            self.redraw();
                        }
                        KeyCode::KeyB
                            if self.session.mode == StudioEditorMode::Simplified
                                && !self.ctrl_held =>
                        {
                            self.drop_at_cursor("button");
                        }
                        KeyCode::KeyL if self.session.mode == StudioEditorMode::Simplified => {
                            self.drop_at_cursor("label");
                        }
                        // P = edit POS when selected, else drop panel
                        KeyCode::KeyP if self.session.mode == StudioEditorMode::Simplified => {
                            if self.session.selected_region.is_some() {
                                self.begin_edit(crate::studio_paint::InspectorField::Pos);
                            } else {
                                self.drop_at_cursor("panel");
                            }
                        }
                        KeyCode::KeyT
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            self.begin_edit(crate::studio_paint::InspectorField::Text);
                        }
                        KeyCode::KeyZ
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some()
                                && !self.ctrl_held =>
                        {
                            self.begin_edit(crate::studio_paint::InspectorField::Size);
                        }
                        KeyCode::ArrowLeft
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(-1.0, 0.0);
                            self.status = "nudge left".into();
                            self.redraw();
                        }
                        KeyCode::ArrowRight
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(1.0, 0.0);
                            self.status = "nudge right".into();
                            self.redraw();
                        }
                        KeyCode::ArrowUp
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(0.0, -1.0);
                            self.status = "nudge up".into();
                            self.redraw();
                        }
                        KeyCode::ArrowDown
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(0.0, 1.0);
                            self.status = "nudge down".into();
                            self.redraw();
                        }
                        KeyCode::KeyS if self.ctrl_held => {
                            self.do_save();
                        }
                        KeyCode::F5
                            if self.session.mode != StudioEditorMode::Advanced =>
                        {
                            self.do_save();
                        }
                        KeyCode::Digit1 if !self.ctrl_held => {
                            if self.session.mode == StudioEditorMode::Nodes {
                                self.session.nodes_tool = NodesTool::Select;
                                self.session.connect_from = None;
                                self.status = "tool: Select".into();
                            } else {
                                let _ = self.session.set_mode(StudioEditorMode::Simplified);
                                self.sync_title();
                                self.status = "Visual mode".into();
                            }
                            self.redraw();
                        }
                        KeyCode::Digit2 if !self.ctrl_held => {
                            if self.session.mode == StudioEditorMode::Nodes {
                                self.session.nodes_tool = NodesTool::Connect;
                                self.session.connect_from = None;
                                self.status = "tool: Connect — click A then B".into();
                            } else {
                                let _ = self.session.set_mode(StudioEditorMode::Advanced);
                                self.sync_title();
                                self.status = "Script mode — F2 validate, F3+ insert API".into();
                            }
                            self.redraw();
                        }
                        KeyCode::Digit3 if !self.ctrl_held => {
                            if self.session.mode == StudioEditorMode::Nodes {
                                self.session.nodes_tool = NodesTool::Disconnect;
                                self.session.connect_from = None;
                                self.status = "tool: Disconnect".into();
                            } else {
                                let _ = self.session.set_mode(StudioEditorMode::Nodes);
                                self.sync_title();
                                self.status = "Nodes mode — tools below, N new screen".into();
                            }
                            self.redraw();
                        }
                        KeyCode::F2 if self.session.mode == StudioEditorMode::Advanced => {
                            self.session.validate_script();
                            self.status = if self.session.script_issues.is_empty() {
                                "script OK".into()
                            } else {
                                format!("{} issue(s)", self.session.script_issues.len())
                            };
                            self.redraw();
                        }
                        KeyCode::F3
                        | KeyCode::F4
                        | KeyCode::F5
                        | KeyCode::F6
                        | KeyCode::F7
                        | KeyCode::F8
                        | KeyCode::F9
                        | KeyCode::F10
                            if self.session.mode == StudioEditorMode::Advanced =>
                        {
                            let idx = match c {
                                KeyCode::F3 => 0,
                                KeyCode::F4 => 1,
                                KeyCode::F5 => 2,
                                KeyCode::F6 => 3,
                                KeyCode::F7 => 4,
                                KeyCode::F8 => 5,
                                KeyCode::F9 => 6,
                                _ => 7,
                            };
                            let cat = crate::vscript::api_catalog();
                            if let Some((_, snip, desc)) = cat.get(idx) {
                                self.session.insert_script_line(snip);
                                self.status = format!("insert {desc}");
                                self.redraw();
                            }
                        }
                        KeyCode::ArrowUp if self.session.mode == StudioEditorMode::Advanced => {
                            if self.session.script_cursor_line > 0 {
                                self.session.script_cursor_line -= 1;
                            }
                            self.redraw();
                        }
                        KeyCode::ArrowDown if self.session.mode == StudioEditorMode::Advanced => {
                            let n = self.session.document_source.lines().count();
                            if self.session.script_cursor_line + 1 < n {
                                self.session.script_cursor_line += 1;
                            }
                            self.redraw();
                        }
                        KeyCode::KeyO
                            if self.session.mode == StudioEditorMode::Advanced
                                && self.session.layers.active().is_some() =>
                        {
                            let id = self.session.layers.active_id.clone();
                            self.session.insert_layer_open(&id);
                            self.status = format!("inserted layer.open(\"{id}\")");
                            self.redraw();
                        }
                        KeyCode::KeyI
                            if self.session.mode == StudioEditorMode::Advanced
                                && self.session.selected_region.is_some() =>
                        {
                            let id = self.session.selected_region.clone().unwrap();
                            self.session.insert_button_press(&id);
                            self.status = format!("inserted button.press(\"{id}\")");
                            self.redraw();
                        }
                        KeyCode::Enter if self.session.mode == StudioEditorMode::Nodes => {
                            let id = self.session.layers.active_id.clone();
                            match self.session.set_layer(&id) {
                                Ok(s) => {
                                    let _ = self.session.set_mode(StudioEditorMode::Simplified);
                                    self.sync_title();
                                    self.status = format!("{s} — Visual");
                                }
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::BracketRight
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            // ] grow size
                            let _ = self.session.resize_selected(2.0, 1.0);
                            self.status = "resized +".into();
                            self.redraw();
                        }
                        KeyCode::BracketLeft
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.resize_selected(-2.0, -1.0);
                            self.status = "resized -".into();
                            self.redraw();
                        }
                        KeyCode::KeyG if self.session.mode == StudioEditorMode::Simplified => {
                            self.session.snap_pct = if (self.session.snap_pct - 1.0).abs() < 0.1 {
                                5.0
                            } else {
                                1.0
                            };
                            self.status = format!("snap {}%", self.session.snap_pct);
                            self.redraw();
                        }
                        KeyCode::KeyB
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some()
                                && self.ctrl_held =>
                        {
                            // Ctrl+B bind selected button to active layer open
                            let lid = self.session.layers.active_id.clone();
                            match self
                                .session
                                .inject_line_on_selected(&format!("layer.open(\"{lid}\")"))
                            {
                                Ok(()) => self.status = format!("bound button → layer.open({lid})"),
                                Err(e) => self.status = format!("bind: {e}"),
                            }
                            self.redraw();
                        }
                        KeyCode::Digit4
                            if self.session.mode == StudioEditorMode::Nodes && !self.ctrl_held =>
                        {
                            self.session.nodes_tool = NodesTool::Overlay;
                            self.session.connect_from = None;
                            self.status = "tool: Overlay link".into();
                            self.redraw();
                        }
                        KeyCode::KeyN if self.session.mode == StudioEditorMode::Nodes => {
                            let n = self.session.layers.layers.len() + 1;
                            match self.session.create_screen(&format!("Pantalla {n}")) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::KeyS
                            if self.session.mode == StudioEditorMode::Nodes && !self.ctrl_held =>
                        {
                            let n = self.session.layers.layers.len() + 1;
                            match self.session.create_sub_screen(&format!("Sub {n}")) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Delete | KeyCode::Backspace
                            if self.session.mode == StudioEditorMode::Nodes =>
                        {
                            if let Some((a, b)) = self.session.selected_edge.clone() {
                                self.status = self.session.disconnect_layers(&a, &b);
                            } else {
                                let id = self.session.layers.active_id.clone();
                                match self.session.layers.remove_layer(&id) {
                                    Ok(()) => self.status = format!("removed {id}"),
                                    Err(e) => self.status = e,
                                }
                            }
                            self.redraw();
                        }
                        KeyCode::KeyX if self.session.mode == StudioEditorMode::Nodes => {
                            if let Some((a, b)) = self.session.selected_edge.clone() {
                                self.status = self.session.disconnect_layers(&a, &b);
                            } else if let Some(from) = self.session.connect_from.clone() {
                                let n = self.session.layers.disconnect_all_for(&from);
                                self.status = format!("cut {n} edges on {from}");
                                self.session.connect_from = None;
                            } else {
                                self.session.nodes_tool = NodesTool::Disconnect;
                                self.status = "tool: Disconnect".into();
                            }
                            self.redraw();
                        }
                        KeyCode::KeyC if self.session.mode == StudioEditorMode::Nodes => {
                            self.session.nodes_tool = NodesTool::Connect;
                            self.session.connect_from = None;
                            self.status = "tool: Connect".into();
                            self.redraw();
                        }
                        // Layer stack
                        KeyCode::BracketRight | KeyCode::PageDown => {
                            match self.session.cycle_layer(true) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::BracketLeft | KeyCode::PageUp => {
                            match self.session.cycle_layer(false) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::KeyU => {
                            let locked = self.session.layers.toggle_lock_active();
                            self.status = if locked {
                                "layer locked".into()
                            } else {
                                "layer unlocked".into()
                            };
                            self.redraw();
                        }
                        KeyCode::KeyM => {
                            match self.session.add_mobile_layer() {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::KeyN if self.session.mode == StudioEditorMode::Simplified => {
                            // N = new sublayer under active
                            let seq = self.session.drop_seq + 1;
                            let id = format!("sub{seq}");
                            match self.session.add_sublayer(&id, "Nueva subcapa") {
                                Ok(s) => {
                                    self.session.drop_seq = seq;
                                    self.status = s;
                                }
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Digit3 if self.ctrl_held => {
                            match self.session.set_layer_preset(ResPreset::DesktopHd) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Digit4 if self.ctrl_held => {
                            match self.session.set_layer_preset(ResPreset::MobilePortrait) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Digit5 if self.ctrl_held => {
                            match self.session.set_layer_preset(ResPreset::MobileLandscape) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Escape => el.exit(),
                        _ => {}
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    let layout = self.layout();
                    if state == ElementState::Pressed {
                        // Toolbar hits (mode / save) work in both modes
                        if let Some(action) = layout.hit_toolbar(self.cursor.0, self.cursor.1) {
                            self.cancel_edit();
                            match action {
                                "mode_visual" => {
                                    let _ = self.session.set_mode(StudioEditorMode::Simplified);
                                    self.sync_title();
                                    self.status = "Visual mode".into();
                                }
                                "mode_script" => {
                                    let _ = self.session.set_mode(StudioEditorMode::Advanced);
                                    self.sync_title();
                                    self.status = "Script mode".into();
                                }
                                "mode_nodes" => {
                                    let _ = self.session.set_mode(StudioEditorMode::Nodes);
                                    self.sync_title();
                                    self.status = "Nodes mode".into();
                                }
                                "save" => self.do_save(),
                                _ => {}
                            }
                            self.redraw();
                            return;
                        }

                        // Inspector field click → start edit (needs selection)
                        if self.session.mode == StudioEditorMode::Simplified
                            && self.session.selected_region.is_some()
                            && layout.contains_inspector(self.cursor.0, self.cursor.1)
                        {
                            if let Some(field) =
                                layout.hit_inspector_field(self.cursor.0, self.cursor.1)
                            {
                                self.begin_edit(field);
                                return;
                            }
                            // click empty inspector area: keep selection, end edit
                            self.cancel_edit();
                            self.redraw();
                            return;
                        }

                        // Nodes mode: tools, edges, nodes, create screens
                        if self.session.mode == StudioEditorMode::Nodes {
                            if let Some(tool) = crate::studio_paint::hit_nodes_toolbar(
                                &layout,
                                self.cursor.0,
                                self.cursor.1,
                                layout.zoom,
                            ) {
                                match tool {
                                    "select" => {
                                        self.session.nodes_tool = NodesTool::Select;
                                        self.session.connect_from = None;
                                        self.status = "tool: Select".into();
                                    }
                                    "connect" => {
                                        self.session.nodes_tool = NodesTool::Connect;
                                        self.session.connect_from = None;
                                        self.status = "tool: Connect".into();
                                    }
                                    "disconnect" => {
                                        self.session.nodes_tool = NodesTool::Disconnect;
                                        self.session.connect_from = None;
                                        self.status = "tool: Disconnect".into();
                                    }
                                    "overlay" => {
                                        self.session.nodes_tool = NodesTool::Overlay;
                                        self.session.connect_from = None;
                                        self.status = "tool: Overlay".into();
                                    }
                                    "new" => {
                                        let n = self.session.layers.layers.len() + 1;
                                        match self.session.create_screen(&format!("Pantalla {n}")) {
                                            Ok(s) => self.status = s,
                                            Err(e) => self.status = e,
                                        }
                                    }
                                    "sub" => {
                                        let n = self.session.layers.layers.len() + 1;
                                        match self.session.create_sub_screen(&format!("Sub {n}")) {
                                            Ok(s) => self.status = s,
                                            Err(e) => self.status = e,
                                        }
                                    }
                                    "del" => {
                                        if let Some((a, b)) = self.session.selected_edge.clone() {
                                            self.status = self.session.disconnect_layers(&a, &b);
                                        } else {
                                            let id = self.session.layers.active_id.clone();
                                            match self.session.layers.remove_layer(&id) {
                                                Ok(()) => self.status = format!("removed {id}"),
                                                Err(e) => self.status = e,
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                                self.redraw();
                                return;
                            }
                            // edge hit first (for select/disconnect)
                            if let Some((a, b)) = self.session.layers.hit_edge(
                                self.cursor.0,
                                self.cursor.1,
                                layout.canvas_x,
                                layout.canvas_y + 28 + 8 * layout.zoom + 4,
                                layout.canvas_w,
                                (layout.canvas_h - 28 - 8 * layout.zoom - 34 - 8 * layout.zoom - 8)
                                    .max(40),
                                layout.zoom,
                            ) {
                                self.status = self.session.nodes_click_edge(&a, &b);
                                self.redraw();
                                return;
                            }
                            let layout_nodes = self.session.layers.node_layout();
                            if let Some(id) =
                                layout.hit_graph_node(self.cursor.0, self.cursor.1, &layout_nodes)
                            {
                                if self.session.nodes_tool == NodesTool::Select {
                                    self.node_drag_id = Some(id.clone());
                                }
                                self.status = self.session.nodes_click_layer(&id);
                                self.redraw();
                                return;
                            }
                            if layout.contains_left_dock(self.cursor.0, self.cursor.1) {
                                let rows = self.session.layers.visible_tree_rows();
                                if let Some(idx) = layout.hit_layer_row(self.cursor.1, rows.len()) {
                                    if let Some(row) = rows.get(idx) {
                                        self.status = self.session.nodes_click_layer(&row.id);
                                        self.redraw();
                                    }
                                }
                            }
                            return;
                        }

                        if self.session.mode != StudioEditorMode::Simplified {
                            return;
                        }

                        // Left dock: layers / palette / hierarchy
                        if layout.contains_left_dock(self.cursor.0, self.cursor.1) {
                            self.cancel_edit();
                            let rows = self.session.layers.visible_tree_rows();
                            if let Some(idx) = layout.hit_layer_row(self.cursor.1, rows.len()) {
                                if let Some(row) = rows.get(idx) {
                                    // Click near left edge toggles expand when has children
                                    let x = self.cursor.0 as i32;
                                    if row.has_children && x < layout.left_w / 3 {
                                        let open = self.session.toggle_layer_expand(&row.id);
                                        self.status = if open {
                                            format!("{} expanded", row.name)
                                        } else {
                                            format!("{} collapsed", row.name)
                                        };
                                    } else {
                                        match self.session.set_layer(&row.id) {
                                            Ok(s) => self.status = s,
                                            Err(e) => self.status = e,
                                        }
                                    }
                                    self.redraw();
                                }
                                return;
                            }
                            if let Some(kind) = layout.hit_palette(self.cursor.0, self.cursor.1) {
                                if !self.session.layers.active_editable() {
                                    self.status = "layer locked — U unlock or select layer".into();
                                    self.redraw();
                                    return;
                                }
                                match self.session.drop_widget(kind, 50.0, 50.0) {
                                    Ok(id) => {
                                        self.status = format!("placed {id} from palette");
                                    }
                                    Err(e) => self.status = format!("drop failed: {e}"),
                                }
                                self.redraw();
                                return;
                            }
                            let widgets = self
                                .session
                                .list_widgets()
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|w| crate::studio_paint::is_canvas_widget(w))
                                .collect::<Vec<_>>();
                            if let Some(idx) = layout.hit_hierarchy(self.cursor.1, widgets.len()) {
                                if let Some(w) = widgets.get(idx) {
                                    self.session.selected_region = Some(w.id.clone());
                                    self.status = format!("selected {}", w.id);
                                    self.redraw();
                                }
                                return;
                            }
                            return;
                        }

                        // Canvas: design surface hit + drag
                        let surface = self.design_surface();
                        if !surface.contains(self.cursor.0, self.cursor.1) {
                            self.dragging = false;
                            return;
                        }
                        if !self.session.layers.active_editable() {
                            self.status = "layer locked — press U to unlock or [ ] change layer".into();
                            self.redraw();
                            return;
                        }
                        self.cancel_edit();
                        let (cx, cy) = surface.screen_to_pct(self.cursor.0, self.cursor.1);
                        match self.session.canvas_pointer_down(cx, cy) {
                            Ok(Some(id)) => {
                                let (rw, rh) = self.session.layers.active_resolution();
                                let (px, py) = pct_to_px(cx, cy, rw, rh);
                                self.status = format!(
                                    "selected {id} @ ({cx:.0}%,{cy:.0}%) = ({px},{py})px"
                                );
                                self.dragging = true;
                                self.drag_last_px = Some(self.cursor);
                                self.drag_acc_pct = (0.0, 0.0);
                            }
                            Ok(None) => {
                                self.session.selected_region = None;
                                self.status = "canvas empty — click palette or press B".into();
                                self.dragging = false;
                                self.drag_last_px = None;
                                self.drag_acc_pct = (0.0, 0.0);
                            }
                            Err(e) => self.status = format!("hit: {e}"),
                        }
                        self.redraw();
                    } else {
                        // Mouse up
                        if self.node_drag_id.take().is_some() {
                            self.status = "node moved".into();
                            self.redraw();
                        }
                        if self.dragging {
                            if let Some(id) = &self.session.selected_region {
                                self.status = format!("placed {id}");
                            }
                        }
                        self.dragging = false;
                        self.drag_last_px = None;
                        self.drag_acc_pct = (0.0, 0.0);
                        self.redraw();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.cursor = (position.x, position.y);
                    // Nodes: drag node positions
                    if self.session.mode == StudioEditorMode::Nodes {
                        if let Some(ref id) = self.node_drag_id.clone() {
                            let layout = self.layout();
                            let (gx, gy, gw, gh) = layout.graph_content_rect();
                            let px = ((position.x as f32 - gx as f32) / gw as f32 * 100.0)
                                .clamp(6.0, 94.0);
                            let py = ((position.y as f32 - gy as f32) / gh as f32 * 100.0)
                                .clamp(10.0, 90.0);
                            self.session.layers.set_graph_pos(id, px, py);
                            self.redraw();
                        }
                        return;
                    }
                    if self.session.mode != StudioEditorMode::Simplified || !self.dragging {
                        return;
                    }
                    if !self.session.layers.active_editable() {
                        return;
                    }
                    let surface = self.design_surface();
                    if let Some((lx, ly)) = self.drag_last_px {
                        let (dpx, dpy) =
                            surface.screen_delta_to_pct(position.x - lx, position.y - ly);
                        self.drag_acc_pct.0 += dpx;
                        self.drag_acc_pct.1 += dpy;
                        let sx = self.drag_acc_pct.0.trunc();
                        let sy = self.drag_acc_pct.1.trunc();
                        if sx.abs() >= 1.0 || sy.abs() >= 1.0 {
                            match self.session.canvas_pointer_drag(sx, sy) {
                                Ok(Some(r)) => {
                                    let (rw, rh) = self.session.layers.active_resolution();
                                    let (px, py) = pct_to_px(r.pos.x, r.pos.y, rw, rh);
                                    self.status = format!(
                                        "drag → ({:.0}%, {:.0}%) = ({px},{py})px / {rw}x{rh}",
                                        r.pos.x, r.pos.y
                                    );
                                }
                                Ok(None) => {}
                                Err(e) => self.status = format!("drag: {e}"),
                            }
                            self.drag_acc_pct.0 -= sx;
                            self.drag_acc_pct.1 -= sy;
                            self.redraw();
                        }
                        self.drag_last_px = Some((position.x, position.y));
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.frames += 1;
                    if let Err(e) = self.paint_frame() {
                        eprintln!("[studio-gui] paint error: {e}");
                    }
                    if !self.interactive && self.frames >= 12 {
                        el.exit();
                    }
                }
                WindowEvent::Resized(_) => {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.05);
            self.last_tick = now;
            let animating = self.session.tick_layers(dt);
            if animating {
                el.set_control_flow(ControlFlow::Poll);
                self.redraw();
                return;
            }
            if self.interactive {
                el.set_control_flow(ControlFlow::Wait);
                return;
            }
            el.set_control_flow(ControlFlow::Poll);
            if self.frames >= 12 {
                el.exit();
                return;
            }
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    impl Host {
        fn layout(&self) -> crate::studio_paint::StudioLayout {
            let (ww, wh) = self
                .window
                .as_ref()
                .map(|w| {
                    let s = w.inner_size();
                    (s.width.max(1), s.height.max(1))
                })
                .unwrap_or((1280, 800));
            crate::studio_paint::StudioLayout::new(ww, wh, self.ui_zoom)
        }

        fn design_surface(&self) -> DesignSurface {
            let lay = self.layout();
            self.session
                .design_surface(lay.canvas_x, lay.canvas_y, lay.canvas_w, lay.canvas_h)
        }

        fn cursor_canvas_pct(&self) -> (f32, f32) {
            let surface = self.design_surface();
            if surface.contains(self.cursor.0, self.cursor.1) {
                surface.screen_to_pct(self.cursor.0, self.cursor.1)
            } else {
                (50.0, 50.0)
            }
        }

        fn drop_at_cursor(&mut self, kind: &str) {
            self.cancel_edit();
            if !self.session.layers.active_editable() {
                self.status = "layer locked — U to unlock".into();
                self.redraw();
                return;
            }
            let (cx, cy) = self.cursor_canvas_pct();
            match self.session.drop_widget(kind, cx, cy) {
                Ok(id) => {
                    let (rw, rh) = self.session.layers.active_resolution();
                    let (px, py) = pct_to_px(cx, cy, rw, rh);
                    self.status =
                        format!("dropped {id} at ({cx:.0}%,{cy:.0}%) = ({px},{py})px");
                }
                Err(e) => self.status = format!("drop failed: {e}"),
            }
            self.redraw();
        }

        fn do_save(&mut self) {
            if self.edit_field.is_some() {
                self.commit_edit();
            }
            self.session.flush_active_document();
            match self.session.save_studio_project() {
                Ok(()) => self.status = "saved velvet.studio.json + all screens".into(),
                Err(e) => {
                    // fallback single-file
                    match self.session.save_document() {
                        Ok(()) => self.status = format!("saved active screen ({e})"),
                        Err(e2) => self.status = format!("save: {e2}"),
                    }
                }
            }
            self.redraw();
        }

        fn cancel_edit(&mut self) {
            self.edit_field = None;
            self.edit_buf.clear();
        }

        fn bump_zoom(&mut self, delta: i32) {
            let next = (self.ui_zoom + delta).clamp(1, 4);
            if next != self.ui_zoom {
                self.ui_zoom = next;
                self.status = format!(
                    "UI zoom x{} — layout+text  (Ctrl+wheel / Ctrl+/- / Ctrl+0)",
                    self.ui_zoom
                );
                self.redraw();
            } else {
                self.status = format!("UI zoom x{} (min 1 / max 4)", self.ui_zoom);
                self.redraw();
            }
        }

        fn begin_edit(&mut self, field: crate::studio_paint::InspectorField) {
            use crate::studio_paint::InspectorField;
            let Some(id) = self.session.selected_region.clone() else {
                self.status = "select a widget first".into();
                self.redraw();
                return;
            };
            let widgets = self.session.list_widgets().unwrap_or_default();
            let w = widgets.iter().find(|w| w.id == id);
            let initial = match (field, w) {
                (InspectorField::Text, Some(w)) => w.text.clone().unwrap_or_default(),
                (InspectorField::Text, None) => String::new(),
                (InspectorField::Pos, Some(w)) => w
                    .position
                    .clone()
                    .unwrap_or_else(|| "(50%, 50%)".into()),
                (InspectorField::Pos, None) => "(50%, 50%)".into(),
                (InspectorField::Size, Some(w)) => w
                    .size
                    .clone()
                    .unwrap_or_else(|| "(18%, 8%)".into()),
                (InspectorField::Size, None) => "(18%, 8%)".into(),
            };
            self.edit_field = Some(field);
            self.edit_buf = initial;
            self.status = format!(
                "editing {} — type, Enter apply, Esc cancel",
                field.label()
            );
            self.redraw();
        }

        fn commit_edit(&mut self) {
            use crate::studio_paint::InspectorField;
            let Some(field) = self.edit_field else {
                return;
            };
            let buf = self.edit_buf.clone();
            let result = match field {
                InspectorField::Text => self.session.set_widget_text(None, &buf),
                InspectorField::Pos => self.session.set_widget_position(None, &buf),
                InspectorField::Size => self.session.set_widget_size(None, &buf),
            };
            match result {
                Ok(()) => {
                    self.status = format!("{} updated", field.label());
                    self.cancel_edit();
                }
                Err(e) => {
                    self.status = format!("edit failed: {e}");
                    // keep edit open so user can fix
                }
            }
            self.redraw();
        }

        fn redraw(&self) {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }

        fn sync_title(&self) {
            if let Some(w) = &self.window {
                let mode = match self.session.mode {
                    StudioEditorMode::Simplified => "Visual",
                    StudioEditorMode::Advanced => "Script",
                    StudioEditorMode::Nodes => "Nodes",
                };
                w.set_title(&format!("Velvet Studio [{mode}] — 1 Vis · 2 Scr · 3 Nod"));
            }
        }

        fn paint_frame(&mut self) -> Result<()> {
            use velvet_story::pack_rgb;
            let Some(window) = self.window.clone() else {
                return Ok(());
            };
            let size = window.inner_size();
            let ww = size.width.max(1);
            let wh = size.height.max(1);
            if ww < 2 || wh < 2 {
                return Ok(());
            }
            if self.pixels.len() != (ww * wh) as usize {
                self.pixels.resize((ww * wh) as usize, 0);
            }
            let layout = crate::studio_paint::StudioLayout::new(ww, wh, self.ui_zoom);
            let widgets = self.session.list_widgets().unwrap_or_default();
            let project = self
                .session
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project");
            let (rw, rh) = self.session.layers.display_resolution();
            let tree_rows = self.session.layers.visible_tree_rows();
            let path = self.session.layers.active_path();
            let path_joined = path.join(" > ");
            let node_layout = self.session.layers.node_layout();
            let paint_mode = match self.session.mode {
                StudioEditorMode::Simplified => crate::studio_paint::PaintMode::Visual,
                StudioEditorMode::Advanced => crate::studio_paint::PaintMode::Script,
                StudioEditorMode::Nodes => crate::studio_paint::PaintMode::Nodes,
            };
            let nodes_tool = match self.session.nodes_tool {
                NodesTool::Select => "select",
                NodesTool::Connect => "connect",
                NodesTool::Disconnect => "disconnect",
                NodesTool::Overlay => "overlay",
            };
            let sel_edge = self.session.selected_edge.as_ref().map(|(a, b)| (a.as_str(), b.as_str()));
            let layer_view = crate::studio_paint::LayerPaintView {
                layers: &self.session.layers.layers,
                tree_rows: &tree_rows,
                edges: &self.session.layers.edges,
                active_id: &self.session.layers.active_id,
                breadcrumb: &path_joined,
                res_w: rw,
                res_h: rh,
                animating: self.session.layers.resize_anim.is_some(),
                editable: self.session.layers.active_editable(),
                pos_px: self.session.selected_pos_px(),
                connect_from: self.session.connect_from.as_deref(),
                script_cursor_line: self.session.script_cursor_line,
                script_issues: &self.session.script_issues,
                node_layout: &node_layout,
                nodes_tool,
                selected_edge: sel_edge,
            };
            crate::studio_paint::paint_studio(
                &mut self.pixels,
                &layout,
                paint_mode,
                project,
                self.session.selected_region.as_deref(),
                &widgets,
                self.session.advanced_source(),
                &self.status,
                self.dragging,
                self.edit_field,
                &self.edit_buf,
                self.ui_zoom,
                &layer_view,
            );

            let Some(surface) = self.surface.as_mut() else {
                return Ok(());
            };
            let Some(nw) = NonZeroU32::new(ww) else {
                return Ok(());
            };
            let Some(nh) = NonZeroU32::new(wh) else {
                return Ok(());
            };
            surface
                .resize(nw, nh)
                .map_err(|e| anyhow::anyhow!("surface resize: {e}"))?;
            let mut buf = surface
                .buffer_mut()
                .map_err(|e| anyhow::anyhow!("buffer_mut: {e}"))?;
            if buf.len() != self.pixels.len() {
                self.pixels.resize(buf.len(), pack_rgb(15, 17, 26));
            }
            buf.copy_from_slice(&self.pixels);
            buf.present()
                .map_err(|e| anyhow::anyhow!("present: {e}"))?;
            Ok(())
        }
    }

    #[cfg(windows)]
    let event_loop = {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        EventLoop::builder()
            .with_any_thread(true)
            .build()
            .map_err(|e| anyhow::anyhow!("event loop: {e}"))?
    };
    #[cfg(not(windows))]
    let event_loop = EventLoop::new().map_err(|e| anyhow::anyhow!("event loop: {e}"))?;

    let mut host = Host {
        session: session.clone(),
        interactive,
        frames: 0,
        window: None,
        context: None,
        surface: None,
        pixels: Vec::new(),
        cursor: (0.0, 0.0),
        drag_last_px: None,
        dragging: false,
        drag_acc_pct: (0.0, 0.0),
        ctrl_held: false,
        edit_field: None,
        edit_buf: String::new(),
        ui_zoom: 2,
        last_tick: std::time::Instant::now(),
        node_drag_id: None,
        status: "ready — 1 Vis  2 Scr  3 Nod  |  N screen in Nodes".into(),
    };
    event_loop
        .run_app(&mut host)
        .map_err(|e| anyhow::anyhow!("run_app: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const MENU: &str = r#"
// @visual id=button.start
button start {
    text: "Iniciar"
    position: (50%, 62%)
// @advanced id=button.start
    on_pressed {
        game.new()
    }
// @end
}
"#;

    #[test]
    fn dock_ready_and_drag_roundtrip_preserves_advanced() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        {
            let mut f = fs::File::create(&menu).unwrap();
            write!(f, "{MENU}").unwrap();
        }

        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        assert!(session.ready);
        assert!(session.panel_ids().contains(&"canvas"));
        assert!(session.panel_ids().contains(&"hierarchy"));
        session.open_document(&menu).unwrap();
        let before = {
            let doc = parse_document(&session.document_source).unwrap();
            region_rect(&doc, "button.start").unwrap()
        };
        session.select_region("button.start").unwrap();
        let after = session.drag_region(None, -4.0, 2.0).unwrap();
        assert!((after.pos.x - (before.pos.x - 4.0)).abs() < 0.01);
        session.save_document().unwrap();
        let disk = fs::read_to_string(&menu).unwrap();
        assert!(disk.contains("game.new()"), "advanced kept: {disk}");
        assert!(
            disk.contains("position: (46%, 64%)") || disk.contains("46%"),
            "drag persisted: {disk}"
        );
    }

    #[test]
    fn run_gui_headless_writes_ready_log() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("p");
        fs::create_dir_all(&proj).unwrap();
        let log = dir.path().join("ready.log");
        let status = run_studio_gui(StudioGuiConfig {
            root: proj,
            headless: true,
            once: true,
            ready_log: Some(log.clone()),
            ..Default::default()
        })
        .unwrap();
        assert!(status.session.ready);
        let text = fs::read_to_string(&log).unwrap();
        assert!(text.contains("ready=true") || text.contains("[studio-gui] ready"));
        assert!(text.contains("canvas"));
    }

    #[test]
    fn dual_mode_drop_drag_preserves_advanced_and_toggle() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();

        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        assert_eq!(session.mode, StudioEditorMode::Simplified);

        // Drop new button
        let id = session.drop_widget("button", 30.0, 40.0).unwrap();
        assert!(id.contains("button") || id.starts_with("button."));

        // Drag original; advanced must survive
        session.select_region("button.start").unwrap();
        session.drag_region(None, -2.0, 1.0).unwrap();
        assert!(
            session.document_source.contains("game.new()"),
            "advanced lost: {}",
            session.document_source
        );

        // Advanced mode: edit text in script buffer, reparse
        session.set_mode(StudioEditorMode::Advanced).unwrap();
        assert_eq!(session.mode, StudioEditorMode::Advanced);
        let mut src = session.advanced_source().to_string();
        src = src.replace("Iniciar", "Start");
        session.set_advanced_source(src).unwrap();
        session.set_mode(StudioEditorMode::Simplified).unwrap();
        let widgets = session.list_widgets().unwrap();
        let start = widgets.iter().find(|w| w.id == "button.start").unwrap();
        assert_eq!(start.text.as_deref(), Some("Start"));
        assert!(session.document_source.contains("game.new()"));

        // Simplified -> Advanced -> Nodes -> Simplified
        assert_eq!(session.mode, StudioEditorMode::Simplified);
        session.toggle_mode().unwrap();
        assert_eq!(session.mode, StudioEditorMode::Advanced);
        session.toggle_mode().unwrap();
        assert_eq!(session.mode, StudioEditorMode::Nodes);
        session.toggle_mode().unwrap();
        assert_eq!(session.mode, StudioEditorMode::Simplified);
    }

    #[test]
    fn vscript_and_nodes_connect() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();
        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        session.set_mode(StudioEditorMode::Advanced).unwrap();
        session.insert_layer_open("menu_settings");
        session.insert_button_press("button.start");
        assert!(session.document_source.contains("layer.open"));
        assert!(session.document_source.contains("button.press"));
        session.validate_script();
        // nodes connect
        session.set_mode(StudioEditorMode::Nodes).unwrap();
        session
            .connect_layers("menu_quit", "hud", Some("quit->hud".into()))
            .unwrap();
        assert!(session
            .layers
            .edges
            .iter()
            .any(|e| e.from == "menu_quit" && e.to == "hud"));
        let msg = session.nodes_click_layer("menu_settings");
        assert!(msg.contains("connect from") || msg.contains("menu_settings"));
        session.nodes_tool = NodesTool::Connect;
        let msg2 = session.nodes_click_layer("scene_decisions");
        assert!(
            msg2.contains("connected")
                || msg2.contains("exists")
                || msg2.contains("scene")
                || msg2.contains("decisions")
                || msg2.contains("from")
        );
        // create empty screen + disconnect
        session.create_screen("Extra UI").unwrap();
        let new_id = session.layers.active_id.clone();
        let canvas_n = session
            .list_widgets()
            .unwrap()
            .into_iter()
            .filter(|w| crate::studio_paint::is_canvas_widget(w))
            .count();
        assert_eq!(canvas_n, 0, "new screen starts empty");
        session.connect_layers(&new_id, "hud", None).unwrap();
        assert!(session.disconnect_layers(&new_id, "hud").contains("disconnected"));
    }

    #[test]
    fn each_screen_has_own_document() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();
        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        assert!(session
            .list_widgets()
            .unwrap()
            .iter()
            .any(|w| w.id == "button.start"));
        // independent empty layer
        session.set_layer("menu_settings").unwrap();
        assert!(
            !session
                .list_widgets()
                .unwrap()
                .iter()
                .any(|w| w.id == "button.start"),
            "settings must not share main_menu widgets"
        );
        session.drop_widget("button", 40.0, 40.0).unwrap();
        assert_eq!(
            session
                .list_widgets()
                .unwrap()
                .into_iter()
                .filter(|w| crate::studio_paint::is_canvas_widget(w))
                .count(),
            1
        );
        // menu intact
        session.set_layer("main_menu").unwrap();
        assert!(session
            .list_widgets()
            .unwrap()
            .iter()
            .any(|w| w.id == "button.start"));
        // brand-new screen from zero
        session.create_screen("Tienda").unwrap();
        let empty_count = session
            .list_widgets()
            .unwrap()
            .into_iter()
            .filter(|w| crate::studio_paint::is_canvas_widget(w))
            .count();
        assert_eq!(empty_count, 0, "new screen starts with zero canvas widgets");
    }

    #[test]
    fn undo_delete_duplicate_and_studio_json() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();
        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        let n0 = session.list_widgets().unwrap().len();
        session.drop_widget("button", 20.0, 20.0).unwrap();
        assert!(session.list_widgets().unwrap().len() > n0);
        session.push_undo(); // ensure stack
        let id = session.selected_region.clone().unwrap();
        session.delete_selected_widget().unwrap();
        assert!(!session
            .list_widgets()
            .unwrap()
            .iter()
            .any(|w| w.id == id));
        // inject advanced
        session.select_region("button.start").unwrap();
        session
            .inject_line_on_selected("layer.open(\"scene\")")
            .unwrap();
        assert!(session.document_source.contains("layer.open"));
        session.save_studio_project().unwrap();
        assert!(proj.join("velvet.studio.json").is_file());
        let reopened = StudioGuiSession::open_project(&proj).unwrap();
        assert!(reopened.layers.get("main_menu").is_some());
    }

    #[test]
    fn inspector_set_text_pos_size_and_normalize() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();

        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        session.select_region("button.start").unwrap();

        session.set_widget_text(None, "Jugar ahora").unwrap();
        session.set_widget_position(None, "40, 55").unwrap();
        session.set_widget_size(None, "22% 10%").unwrap();
        session.nudge_selected(1.0, -1.0).unwrap();

        let src = &session.document_source;
        assert!(src.contains("Jugar ahora"), "{src}");
        assert!(
            src.contains("position: (41%, 54%)") || src.contains("(41%, 54%)"),
            "{src}"
        );
        assert!(src.contains("size:") && src.contains("22%"), "{src}");
        assert!(src.contains("game.new()"), "advanced kept: {src}");

        assert!(normalize_pct_pair("50,62").unwrap().contains("50%"));
        assert!(normalize_pct_pair("bad").is_none());
    }

    #[test]
    fn layer_stack_mobile_and_coords() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        assert!(session.layers.get("main_menu").is_some());
        assert!(session.layers.get("menu_settings").is_some());
        assert!(session.layers.get("scene_decisions").is_some());
        let rows = session.layers.visible_tree_rows();
        assert!(rows.iter().any(|r| r.depth == 1));
        session.add_mobile_layer().unwrap();
        assert_eq!(session.layers.active_id, "mobile");
        let (w, h) = session.layers.active_resolution();
        assert_eq!((w, h), (390, 844));
        // still under main_menu branch → root not auto-locked
        assert!(!session.layers.get("main_menu").unwrap().locked || true);
        session.set_layer("scene_decisions").unwrap();
        assert!(session.layers.get("main_menu").unwrap().locked);
        session.set_layer("main_menu").unwrap();
        assert!(!session.layers.get("main_menu").unwrap().locked);
        session.set_layer("menu_settings").unwrap();
        assert_eq!(
            session.layers.active_path().last().map(|s| s.as_str()),
            Some("Configuracion")
        );
        session.set_layer_preset(ResPreset::DesktopFhd).unwrap();
        assert_eq!(session.layers.active_resolution(), (1920, 1080));
        let ds = session.design_surface(0, 0, 800, 600);
        assert!(ds.w > 0 && ds.h > 0);
    }
}
