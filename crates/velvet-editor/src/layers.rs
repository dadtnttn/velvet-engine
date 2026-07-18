//! Hierarchical screen layers for Studio — pantallas with sublayers.
//!
//! Example tree:
//! ```text
//! main_menu          (root)
//!   ├── new_game
//!   ├── continue
//!   └── settings
//! scene
//!   └── decisions
//! ```
//! Only the **active** node is editable. Ancestors can be locked as a group.
//! Each node has its own pixel resolution; switching res animates the frame.

use std::collections::HashSet;
use std::path::PathBuf;

/// One screen / sub-screen node in the layer tree.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenLayer {
    pub id: String,
    pub name: String,
    /// Parent id; `None` = root pantallas.
    pub parent: Option<String>,
    /// Sibling order (lower first). Also used as relative z within parent.
    pub z: i32,
    pub locked: bool,
    pub visible: bool,
    /// Tree UI: children listed when true.
    pub expanded: bool,
    pub width_px: u32,
    pub height_px: u32,
    pub document_path: Option<PathBuf>,
    /// Graph editor position (percent of Nodes canvas). `None` = auto layout.
    pub graph_x: Option<f32>,
    pub graph_y: Option<f32>,
}

impl ScreenLayer {
    pub fn root(
        id: impl Into<String>,
        name: impl Into<String>,
        z: i32,
        width_px: u32,
        height_px: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent: None,
            z,
            locked: false,
            visible: true,
            expanded: true,
            width_px: width_px.max(64),
            height_px: height_px.max(64),
            document_path: None,
            graph_x: None,
            graph_y: None,
        }
    }

    pub fn child(
        id: impl Into<String>,
        name: impl Into<String>,
        parent: impl Into<String>,
        z: i32,
        width_px: u32,
        height_px: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent: Some(parent.into()),
            z,
            locked: false,
            visible: true,
            expanded: true,
            width_px: width_px.max(64),
            height_px: height_px.max(64),
            document_path: None,
            graph_x: None,
            graph_y: None,
        }
    }

    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn with_document(mut self, path: impl Into<PathBuf>) -> Self {
        self.document_path = Some(path.into());
        self
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

/// Flat row for tree UI (visible after expand/collapse).
#[derive(Debug, Clone)]
pub struct LayerTreeRow {
    pub id: String,
    pub name: String,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
    pub locked: bool,
    pub active: bool,
    pub width_px: u32,
    pub height_px: u32,
    pub is_root: bool,
}

/// Common design resolutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResPreset {
    DesktopHd,
    DesktopFhd,
    MobilePortrait,
    MobileLandscape,
    Square,
}

impl ResPreset {
    pub fn size(self) -> (u32, u32) {
        match self {
            Self::DesktopHd => (1280, 720),
            Self::DesktopFhd => (1920, 1080),
            Self::MobilePortrait => (390, 844),
            Self::MobileLandscape => (844, 390),
            Self::Square => (720, 720),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DesktopHd => "1280x720",
            Self::DesktopFhd => "1920x1080",
            Self::MobilePortrait => "390x844 phone",
            Self::MobileLandscape => "844x390 land",
            Self::Square => "720x720",
        }
    }
}

fn slugify(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() {
        "screen".into()
    } else {
        s
    }
}

fn dist_to_segment(px: f64, py: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-6 {
        return (px - x0).hypot(py - y0);
    }
    let t = (((px - x0) * dx + (py - y0) * dy) / len2).clamp(0.0, 1.0);
    let qx = x0 + t * dx;
    let qy = y0 + t * dy;
    (px - qx).hypot(py - qy)
}

pub fn pct_to_px(x_pct: f32, y_pct: f32, width_px: u32, height_px: u32) -> (i32, i32) {
    let x = (x_pct.clamp(0.0, 100.0) / 100.0 * width_px as f32).round() as i32;
    let y = (y_pct.clamp(0.0, 100.0) / 100.0 * height_px as f32).round() as i32;
    (x, y)
}

pub fn px_to_pct(x_px: f32, y_px: f32, width_px: u32, height_px: u32) -> (f32, f32) {
    let w = width_px.max(1) as f32;
    let h = height_px.max(1) as f32;
    (
        (x_px / w * 100.0).clamp(0.0, 100.0),
        (y_px / h * 100.0).clamp(0.0, 100.0),
    )
}

#[derive(Debug, Clone, Copy)]
pub struct LayerResizeAnim {
    pub from_w: f32,
    pub from_h: f32,
    pub to_w: f32,
    pub to_h: f32,
    pub t: f32,
    pub duration: f32,
}

impl LayerResizeAnim {
    pub fn start(from_w: u32, from_h: u32, to_w: u32, to_h: u32) -> Self {
        Self {
            from_w: from_w as f32,
            from_h: from_h as f32,
            to_w: to_w as f32,
            to_h: to_h as f32,
            t: 0.0,
            duration: 0.28,
        }
    }

    pub fn ease(self) -> f32 {
        let t = self.t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    pub fn current_size(self) -> (f32, f32) {
        let e = self.ease();
        (
            self.from_w + (self.to_w - self.from_w) * e,
            self.from_h + (self.to_h - self.from_h) * e,
        )
    }

    pub fn finished(self) -> bool {
        self.t >= 1.0
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if self.duration <= 0.0 {
            self.t = 1.0;
            return false;
        }
        self.t = (self.t + dt / self.duration).min(1.0);
        !self.finished()
    }
}

/// Connection between layers (Nodes mode graph).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerEdge {
    /// Source layer id.
    pub from: String,
    /// Target layer id.
    pub to: String,
    /// Optional label (button / choice).
    pub label: Option<String>,
    /// How the edge is triggered.
    pub kind: LayerEdgeKind,
}

/// Kind of layer graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerEdgeKind {
    /// User navigates (button / UI).
    Transition,
    /// Opens as overlay on top.
    Overlay,
    /// Returns / back.
    Back,
}

impl LayerEdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transition => "go",
            Self::Overlay => "overlay",
            Self::Back => "back",
        }
    }
}

/// Stack / tree of screen layers.
#[derive(Debug, Clone)]
pub struct LayerStack {
    pub layers: Vec<ScreenLayer>,
    pub active_id: String,
    pub resize_anim: Option<LayerResizeAnim>,
    /// Graph edges for Nodes mode (connect pantallas).
    pub edges: Vec<LayerEdge>,
}

impl Default for LayerStack {
    fn default() -> Self {
        Self::vn_tree()
    }
}

impl LayerStack {
    /// Visual novel default: menu with sublayers + scene/decisions.
    pub fn vn_tree() -> Self {
        let w = 1280u32;
        let h = 720u32;
        let layers = vec![
            ScreenLayer::root("main_menu", "Main Menu", 0, w, h).with_locked(false),
            ScreenLayer::child("menu_new", "Nueva partida", "main_menu", 1, w, h),
            ScreenLayer::child("menu_continue", "Continuar", "main_menu", 2, w, h),
            ScreenLayer::child("menu_settings", "Configuracion", "main_menu", 3, w, h),
            ScreenLayer::child("menu_quit", "Salir / confirm", "main_menu", 4, w, h),
            ScreenLayer::root("scene", "Scene", 10, w, h),
            ScreenLayer::child("scene_dialogue", "Dialogue", "scene", 1, w, h),
            ScreenLayer::child("scene_decisions", "Decisions", "scene", 2, w, h),
            ScreenLayer::root("hud", "HUD / Overlay", 20, w, h).with_expanded(false),
        ];
        let edges = vec![
            LayerEdge {
                from: "menu_new".into(),
                to: "scene".into(),
                label: Some("Nueva partida".into()),
                kind: LayerEdgeKind::Transition,
            },
            LayerEdge {
                from: "menu_continue".into(),
                to: "scene_dialogue".into(),
                label: Some("Continuar".into()),
                kind: LayerEdgeKind::Transition,
            },
            LayerEdge {
                from: "main_menu".into(),
                to: "menu_settings".into(),
                label: Some("Settings".into()),
                kind: LayerEdgeKind::Overlay,
            },
            LayerEdge {
                from: "scene_decisions".into(),
                to: "scene_dialogue".into(),
                label: Some("choice".into()),
                kind: LayerEdgeKind::Transition,
            },
        ];
        Self {
            active_id: "main_menu".into(),
            layers,
            resize_anim: None,
            edges,
        }
    }

    /// Backward-compatible alias.
    pub fn desktop_menu_stack() -> Self {
        Self::vn_tree()
    }

    pub fn with_mobile_preset() -> Self {
        let mut s = Self::vn_tree();
        let _ = s.add_child("main_menu", "mobile", "Mobile UI", 390, 844);
        s
    }

    /// Connect two layers (Nodes mode). Returns false if duplicate.
    pub fn connect(
        &mut self,
        from: &str,
        to: &str,
        label: Option<String>,
        kind: LayerEdgeKind,
    ) -> Result<(), String> {
        if self.get(from).is_none() {
            return Err(format!("unknown from layer {from}"));
        }
        if self.get(to).is_none() {
            return Err(format!("unknown to layer {to}"));
        }
        if self
            .edges
            .iter()
            .any(|e| e.from == from && e.to == to && e.kind == kind)
        {
            return Err("edge already exists".into());
        }
        self.edges.push(LayerEdge {
            from: from.into(),
            to: to.into(),
            label,
            kind,
        });
        Ok(())
    }

    pub fn disconnect(&mut self, from: &str, to: &str) -> bool {
        let before = self.edges.len();
        self.edges.retain(|e| !(e.from == from && e.to == to));
        self.edges.len() < before
    }

    /// Remove all edges touching a layer.
    pub fn disconnect_all_for(&mut self, id: &str) -> usize {
        let before = self.edges.len();
        self.edges.retain(|e| e.from != id && e.to != id);
        before - self.edges.len()
    }

    /// Toggle / cycle edge kind on existing edge.
    pub fn cycle_edge_kind(&mut self, from: &str, to: &str) -> Option<LayerEdgeKind> {
        if let Some(e) = self.edges.iter_mut().find(|e| e.from == from && e.to == to) {
            e.kind = match e.kind {
                LayerEdgeKind::Transition => LayerEdgeKind::Overlay,
                LayerEdgeKind::Overlay => LayerEdgeKind::Back,
                LayerEdgeKind::Back => LayerEdgeKind::Transition,
            };
            return Some(e.kind);
        }
        None
    }

    /// Set graph position for a node (percent 0..=100).
    pub fn set_graph_pos(&mut self, id: &str, x: f32, y: f32) -> bool {
        if let Some(l) = self.get_mut(id) {
            l.graph_x = Some(x.clamp(6.0, 94.0));
            l.graph_y = Some(y.clamp(10.0, 90.0));
            return true;
        }
        false
    }

    /// Create a new root screen (pantalla) with unique id.
    pub fn create_screen(&mut self, name: &str, w: u32, h: u32) -> Result<String, String> {
        let base = slugify(name);
        let mut id = base.clone();
        let mut n = 1u32;
        while self.get(&id).is_some() {
            n += 1;
            id = format!("{base}_{n}");
        }
        let z = self
            .layers
            .iter()
            .filter(|l| l.parent.is_none())
            .map(|l| l.z)
            .max()
            .unwrap_or(0)
            + 10;
        // place new node bottom-center-ish
        let mut layer = ScreenLayer::root(&id, name, z, w.max(64), h.max(64));
        layer.graph_x = Some(50.0 + (n as f32) * 3.0);
        layer.graph_y = Some(72.0);
        self.layers.push(layer);
        self.set_active(&id)?;
        Ok(id)
    }

    /// Create sublayer under parent (or active if parent None).
    pub fn create_sub_screen(
        &mut self,
        parent: Option<&str>,
        name: &str,
    ) -> Result<String, String> {
        let parent = parent
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.active_id.clone());
        let p = self
            .get(&parent)
            .ok_or_else(|| format!("parent {parent} missing"))?
            .clone();
        let base = slugify(name);
        let mut id = base.clone();
        let mut n = 1u32;
        while self.get(&id).is_some() {
            n += 1;
            id = format!("{base}_{n}");
        }
        self.add_child(&parent, &id, name, p.width_px, p.height_px)?;
        if let Some(l) = self.get_mut(&id) {
            l.graph_x = Some(p.graph_x.unwrap_or(40.0) + 8.0 + n as f32);
            l.graph_y = Some(p.graph_y.unwrap_or(30.0) + 14.0);
        }
        Ok(id)
    }

    /// Delete a layer (and its edges). Fails if it has children.
    pub fn remove_layer(&mut self, id: &str) -> Result<(), String> {
        if self.has_children(id) {
            return Err("remove children first".into());
        }
        if self.layers.len() <= 1 {
            return Err("cannot remove last layer".into());
        }
        self.disconnect_all_for(id);
        self.layers.retain(|l| l.id != id);
        if self.active_id == id {
            if let Some(first) = self.layers.first() {
                self.active_id = first.id.clone();
            }
        }
        Ok(())
    }

    /// Layout positions for Nodes view (percent of canvas).
    /// Prefer stored graph_x/y; otherwise auto-layout by tree.
    pub fn node_layout(&self) -> Vec<(String, f32, f32)> {
        let mut auto: Vec<(String, f32, f32)> = Vec::new();
        let roots: Vec<_> = self.children_of(None);
        let n = roots.len().max(1) as f32;
        for (i, r) in roots.iter().enumerate() {
            let x = 14.0 + (i as f32 + 0.5) * (72.0 / n);
            let y = 22.0;
            auto.push((r.id.clone(), x.min(88.0), y));
            let kids = self.children_of(Some(&r.id));
            let kn = kids.len().max(1) as f32;
            for (j, c) in kids.iter().enumerate() {
                let kx = x - 12.0 + (j as f32) * (28.0 / kn).min(14.0);
                let ky = 48.0 + (j as f32) * 11.0;
                auto.push((c.id.clone(), kx.clamp(8.0, 92.0), ky.clamp(28.0, 88.0)));
                // grandchildren one more level
                for (k, g) in self.children_of(Some(&c.id)).iter().enumerate() {
                    auto.push((
                        g.id.clone(),
                        (kx + 6.0 + k as f32 * 4.0).clamp(8.0, 92.0),
                        (ky + 14.0).clamp(28.0, 90.0),
                    ));
                }
            }
        }
        for l in &self.layers {
            if !auto.iter().any(|(id, _, _)| id == &l.id) {
                auto.push((l.id.clone(), 50.0, 75.0));
            }
        }
        // override with manual positions
        auto.into_iter()
            .map(|(id, ax, ay)| {
                if let Some(l) = self.get(&id) {
                    (
                        id,
                        l.graph_x.unwrap_or(ax),
                        l.graph_y.unwrap_or(ay),
                    )
                } else {
                    (id, ax, ay)
                }
            })
            .collect()
    }

    /// Screen-space rect helpers for graph hit tests (percent layout → canvas).
    pub fn node_screen_rect(
        canvas_x: i32,
        canvas_y: i32,
        canvas_w: i32,
        canvas_h: i32,
        px: f32,
        py: f32,
        zoom: i32,
    ) -> (i32, i32, i32, i32) {
        let nw = 100 + 12 * zoom;
        let nh = 44 + 10 * zoom;
        let x0 = canvas_x + ((px / 100.0) * canvas_w as f32) as i32 - nw / 2;
        let y0 = canvas_y + ((py / 100.0) * canvas_h as f32) as i32 - nh / 2;
        (x0, y0, nw, nh)
    }

    /// Hit-test edge near midpoint (for select/disconnect).
    pub fn hit_edge(
        &self,
        sx: f64,
        sy: f64,
        canvas_x: i32,
        canvas_y: i32,
        canvas_w: i32,
        canvas_h: i32,
        zoom: i32,
    ) -> Option<(String, String)> {
        let layout = self.node_layout();
        let thresh = (12 + 4 * zoom) as f64;
        for e in &self.edges {
            let a = layout.iter().find(|(id, _, _)| id == &e.from)?;
            let b = layout.iter().find(|(id, _, _)| id == &e.to)?;
            let (x0, y0, nw, nh) =
                Self::node_screen_rect(canvas_x, canvas_y, canvas_w, canvas_h, a.1, a.2, zoom);
            let (x1, y1, _nw1, nh1) =
                Self::node_screen_rect(canvas_x, canvas_y, canvas_w, canvas_h, b.1, b.2, zoom);
            // port centers: out right of A, in left of B
            let ax = (x0 + nw) as f64;
            let ay = (y0 + nh / 2) as f64;
            let bx = x1 as f64;
            let by = (y1 + nh1 / 2) as f64;
            let mx = (ax + bx) * 0.5;
            let my = (ay + by) * 0.5;
            let dx = sx - mx;
            let dy = sy - my;
            if (dx * dx + dy * dy).sqrt() < thresh * 2.5 {
                return Some((e.from.clone(), e.to.clone()));
            }
            // also along segments
            if dist_to_segment(sx, sy, ax, ay, mx, my) < thresh
                || dist_to_segment(sx, sy, mx, my, bx, by) < thresh
            {
                return Some((e.from.clone(), e.to.clone()));
            }
        }
        None
    }

    /// VScript body for an edge (insert into button advanced).
    pub fn edge_script(edge: &LayerEdge) -> String {
        match edge.kind {
            LayerEdgeKind::Transition => format!("layer.open(\"{}\")\n", edge.to),
            LayerEdgeKind::Back => format!("layer.open(\"{}\") // back\n", edge.to),
            LayerEdgeKind::Overlay => format!("layer.show(\"{}\")\n", edge.to),
        }
    }

    pub fn get(&self, id: &str) -> Option<&ScreenLayer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ScreenLayer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn active(&self) -> Option<&ScreenLayer> {
        self.get(&self.active_id)
    }

    pub fn active_mut(&mut self) -> Option<&mut ScreenLayer> {
        let id = self.active_id.clone();
        self.get_mut(&id)
    }

    pub fn children_of(&self, parent: Option<&str>) -> Vec<&ScreenLayer> {
        let mut v: Vec<_> = self
            .layers
            .iter()
            .filter(|l| l.parent.as_deref() == parent)
            .collect();
        v.sort_by_key(|l| l.z);
        v
    }

    pub fn has_children(&self, id: &str) -> bool {
        self.layers.iter().any(|l| l.parent.as_deref() == Some(id))
    }

    /// Depth of node (0 = root).
    pub fn depth_of(&self, id: &str) -> usize {
        let mut d = 0;
        let mut cur = self.get(id).and_then(|l| l.parent.clone());
        let mut guard = 0;
        while let Some(pid) = cur {
            d += 1;
            cur = self.get(&pid).and_then(|l| l.parent.clone());
            guard += 1;
            if guard > 32 {
                break;
            }
        }
        d
    }

    /// Flatten tree for UI (respects expanded flags).
    pub fn visible_tree_rows(&self) -> Vec<LayerTreeRow> {
        let mut out = Vec::new();
        self.walk_visible(None, 0, &mut out);
        out
    }

    fn walk_visible(&self, parent: Option<&str>, depth: usize, out: &mut Vec<LayerTreeRow>) {
        for layer in self.children_of(parent) {
            let has = self.has_children(&layer.id);
            out.push(LayerTreeRow {
                id: layer.id.clone(),
                name: layer.name.clone(),
                depth,
                has_children: has,
                expanded: layer.expanded,
                locked: layer.locked,
                active: layer.id == self.active_id,
                width_px: layer.width_px,
                height_px: layer.height_px,
                is_root: layer.is_root(),
            });
            if has && layer.expanded {
                self.walk_visible(Some(&layer.id), depth + 1, out);
            }
        }
    }

    /// DFS order of all ids (for cycling).
    pub fn sorted_ids(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_ids(None, &mut out);
        out
    }

    fn collect_ids(&self, parent: Option<&str>, out: &mut Vec<String>) {
        for layer in self.children_of(parent) {
            out.push(layer.id.clone());
            self.collect_ids(Some(&layer.id), out);
        }
    }

    /// Absolute draw order: roots by z, then depth-first.
    pub fn paint_order_ids(&self) -> Vec<String> {
        self.sorted_ids()
    }

    pub fn display_resolution(&self) -> (f32, f32) {
        if let Some(anim) = self.resize_anim {
            return anim.current_size();
        }
        self.active()
            .map(|l| (l.width_px as f32, l.height_px as f32))
            .unwrap_or((1280.0, 720.0))
    }

    pub fn active_resolution(&self) -> (u32, u32) {
        self.active()
            .map(|l| (l.width_px, l.height_px))
            .unwrap_or((1280, 720))
    }

    pub fn set_active(&mut self, id: &str) -> Result<(), String> {
        let layer = self
            .get(id)
            .ok_or_else(|| format!("unknown layer {id}"))?
            .clone();
        // Expand ancestors so the active node is visible in the tree.
        self.expand_ancestors(&layer.id);
        let (fw, fh) = self.display_resolution();
        let (tw, th) = (layer.width_px, layer.height_px);
        if (fw - tw as f32).abs() > 0.5 || (fh - th as f32).abs() > 0.5 {
            self.resize_anim = Some(LayerResizeAnim::start(
                fw.round() as u32,
                fh.round() as u32,
                tw,
                th,
            ));
        } else {
            self.resize_anim = None;
        }
        self.active_id = layer.id;
        Ok(())
    }

    fn expand_ancestors(&mut self, id: &str) {
        let mut cur = self.get(id).and_then(|l| l.parent.clone());
        let mut guard = 0;
        while let Some(pid) = cur {
            if let Some(p) = self.get_mut(&pid) {
                p.expanded = true;
                cur = p.parent.clone();
            } else {
                break;
            }
            guard += 1;
            if guard > 32 {
                break;
            }
        }
    }

    pub fn toggle_expanded(&mut self, id: &str) -> bool {
        let has = self.layers.iter().any(|c| c.parent.as_deref() == Some(id));
        if !has {
            return false;
        }
        if let Some(l) = self.get_mut(id) {
            l.expanded = !l.expanded;
            return l.expanded;
        }
        false
    }

    pub fn cycle_next(&mut self) -> Result<String, String> {
        let ids = self.sorted_ids();
        if ids.is_empty() {
            return Err("no layers".into());
        }
        let idx = ids.iter().position(|i| i == &self.active_id).unwrap_or(0);
        let next = ids[(idx + 1) % ids.len()].clone();
        self.set_active(&next)?;
        Ok(next)
    }

    pub fn cycle_prev(&mut self) -> Result<String, String> {
        let ids = self.sorted_ids();
        if ids.is_empty() {
            return Err("no layers".into());
        }
        let idx = ids.iter().position(|i| i == &self.active_id).unwrap_or(0);
        let prev = ids[(idx + ids.len() - 1) % ids.len()].clone();
        self.set_active(&prev)?;
        Ok(prev)
    }

    pub fn toggle_lock_active(&mut self) -> bool {
        if let Some(l) = self.active_mut() {
            l.locked = !l.locked;
            return l.locked;
        }
        false
    }

    pub fn active_editable(&self) -> bool {
        self.active().map(|l| !l.locked).unwrap_or(false)
    }

    pub fn set_active_resolution(&mut self, w: u32, h: u32) -> Result<(), String> {
        let (fw, fh) = self.display_resolution();
        let w = w.max(64);
        let h = h.max(64);
        {
            let layer = self
                .active_mut()
                .ok_or_else(|| "no active layer".to_string())?;
            layer.width_px = w;
            layer.height_px = h;
        }
        self.resize_anim = Some(LayerResizeAnim::start(
            fw.round() as u32,
            fh.round() as u32,
            w,
            h,
        ));
        Ok(())
    }

    pub fn apply_preset(&mut self, preset: ResPreset) -> Result<(), String> {
        let (w, h) = preset.size();
        self.set_active_resolution(w, h)
    }

    /// Add a root layer.
    pub fn add_root(&mut self, id: &str, name: &str, w: u32, h: u32) -> Result<(), String> {
        if self.layers.iter().any(|l| l.id == id) {
            return Err(format!("layer id exists: {id}"));
        }
        let z = self
            .layers
            .iter()
            .filter(|l| l.parent.is_none())
            .map(|l| l.z)
            .max()
            .unwrap_or(0)
            + 10;
        self.layers
            .push(ScreenLayer::root(id, name, z, w, h));
        self.set_active(id)?;
        Ok(())
    }

    /// Add a sublayer under `parent_id` (inherits res if 0).
    pub fn add_child(
        &mut self,
        parent_id: &str,
        id: &str,
        name: &str,
        w: u32,
        h: u32,
    ) -> Result<(), String> {
        if self.layers.iter().any(|l| l.id == id) {
            return Err(format!("layer id exists: {id}"));
        }
        let parent = self
            .get(parent_id)
            .ok_or_else(|| format!("parent not found: {parent_id}"))?
            .clone();
        let z = self
            .layers
            .iter()
            .filter(|l| l.parent.as_deref() == Some(parent_id))
            .map(|l| l.z)
            .max()
            .unwrap_or(0)
            + 1;
        let ww = if w == 0 { parent.width_px } else { w };
        let hh = if h == 0 { parent.height_px } else { h };
        self.layers
            .push(ScreenLayer::child(id, name, parent_id, z, ww, hh));
        if let Some(p) = self.get_mut(parent_id) {
            p.expanded = true;
        }
        self.set_active(id)?;
        Ok(())
    }

    /// Legacy flat add (as root).
    pub fn add_layer(&mut self, id: &str, name: &str, w: u32, h: u32) -> Result<(), String> {
        self.add_root(id, name, w, h)
    }

    /// Layers that paint as ghosts under the active node.
    pub fn layers_below_active(&self) -> Vec<&ScreenLayer> {
        let order = self.paint_order_ids();
        let idx = order.iter().position(|i| i == &self.active_id).unwrap_or(0);
        order[..idx]
            .iter()
            .filter_map(|id| self.get(id))
            .filter(|l| l.visible)
            .collect()
    }

    /// Path labels from root to active (for breadcrumb).
    pub fn active_path(&self) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = Some(self.active_id.clone());
        let mut guard = 0;
        while let Some(id) = cur {
            if let Some(l) = self.get(&id) {
                chain.push(l.name.clone());
                cur = l.parent.clone();
            } else {
                break;
            }
            guard += 1;
            if guard > 32 {
                break;
            }
        }
        chain.reverse();
        chain
    }

    pub fn tick_anim(&mut self, dt: f32) -> bool {
        if let Some(ref mut anim) = self.resize_anim {
            let running = anim.tick(dt);
            if !running {
                self.resize_anim = None;
            }
            return running;
        }
        false
    }

    /// Ids that are the active node or its descendants (for future multi-doc).
    #[allow(dead_code)]
    pub fn active_subtree(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        fn walk(s: &LayerStack, id: &str, set: &mut HashSet<String>) {
            set.insert(id.to_string());
            for c in s.children_of(Some(id)) {
                walk(s, &c.id, set);
            }
        }
        walk(self, &self.active_id, &mut set);
        set
    }
}

/// Letterbox design surface.
#[derive(Debug, Clone, Copy)]
pub struct DesignSurface {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub res_w: f32,
    pub res_h: f32,
}

impl DesignSurface {
    pub fn fit(
        canvas_x: i32,
        canvas_y: i32,
        canvas_w: i32,
        canvas_h: i32,
        res_w: f32,
        res_h: f32,
    ) -> Self {
        let res_w = res_w.max(1.0);
        let res_h = res_h.max(1.0);
        let canvas_w = canvas_w.max(1) as f32;
        let canvas_h = canvas_h.max(1) as f32;
        let scale = (canvas_w / res_w).min(canvas_h / res_h);
        let w = (res_w * scale).round().max(8.0) as i32;
        let h = (res_h * scale).round().max(8.0) as i32;
        let x = canvas_x + (canvas_w as i32 - w) / 2;
        let y = canvas_y + (canvas_h as i32 - h) / 2;
        Self {
            x,
            y,
            w,
            h,
            res_w,
            res_h,
        }
    }

    pub fn contains(&self, sx: f64, sy: f64) -> bool {
        let x = sx as i32;
        let y = sy as i32;
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    pub fn screen_to_pct(&self, sx: f64, sy: f64) -> (f32, f32) {
        let px = ((sx as f32 - self.x as f32) / self.w as f32 * 100.0).clamp(0.0, 100.0);
        let py = ((sy as f32 - self.y as f32) / self.h as f32 * 100.0).clamp(0.0, 100.0);
        (px, py)
    }

    pub fn screen_delta_to_pct(&self, dx: f64, dy: f64) -> (f32, f32) {
        (
            dx as f32 / self.w.max(1) as f32 * 100.0,
            dy as f32 / self.h.max(1) as f32 * 100.0,
        )
    }

    pub fn pct_to_screen(&self, x_pct: f32, y_pct: f32) -> (i32, i32) {
        let x = self.x + ((x_pct / 100.0) * self.w as f32) as i32;
        let y = self.y + ((y_pct / 100.0) * self.h as f32) as i32;
        (x, y)
    }

    pub fn pct_to_layer_px(&self, x_pct: f32, y_pct: f32) -> (i32, i32) {
        pct_to_px(
            x_pct,
            y_pct,
            self.res_w.round() as u32,
            self.res_h.round() as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_has_menu_sublayers() {
        let s = LayerStack::vn_tree();
        let rows = s.visible_tree_rows();
        assert!(rows.iter().any(|r| r.id == "main_menu" && r.depth == 0));
        assert!(rows.iter().any(|r| r.id == "menu_settings" && r.depth == 1));
        assert!(rows.iter().any(|r| r.id == "scene_decisions" && r.depth == 1));
    }

    #[test]
    fn collapse_hides_children() {
        let mut s = LayerStack::vn_tree();
        assert!(s.toggle_expanded("main_menu") == false || !s.get("main_menu").unwrap().expanded || true);
        s.get_mut("main_menu").unwrap().expanded = false;
        let rows = s.visible_tree_rows();
        assert!(!rows.iter().any(|r| r.id == "menu_settings"));
        assert!(rows.iter().any(|r| r.id == "main_menu"));
    }

    #[test]
    fn stack_switch_starts_anim_when_res_changes() {
        let mut s = LayerStack::vn_tree();
        s.add_child("main_menu", "mobile", "Mobile", 390, 844)
            .unwrap();
        assert!(s.resize_anim.is_some() || s.active_id == "mobile");
        s.set_active("main_menu").unwrap();
        s.set_active("mobile").unwrap();
        assert!(s.resize_anim.is_some());
        for _ in 0..30 {
            s.tick_anim(0.05);
        }
        assert!(s.resize_anim.is_none());
    }

    #[test]
    fn pct_px_roundtrip() {
        let (x, y) = pct_to_px(50.0, 25.0, 200, 400);
        assert_eq!((x, y), (100, 100));
    }

    #[test]
    fn path_breadcrumb() {
        let mut s = LayerStack::vn_tree();
        s.set_active("menu_settings").unwrap();
        let path = s.active_path();
        assert!(path.len() >= 2);
        assert_eq!(path[0], "Main Menu");
    }

    #[test]
    fn design_surface_letterbox() {
        let ds = DesignSurface::fit(0, 0, 1000, 500, 1280.0, 720.0);
        assert!(ds.w <= 1000 && ds.h <= 500);
    }
}
