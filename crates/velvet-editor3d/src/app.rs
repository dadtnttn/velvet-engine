//! The editor itself: menus, the docked panels (scene tree, file system, 3D viewport, scripts,
//! inspector, output), dialogs and the edit history.

use std::collections::HashSet;
use std::f32::consts::{FRAC_PI_2, PI};
use std::path::{Path, PathBuf};
use std::time::Instant;

use egui::{
    Align, Align2, Color32, DragValue, FontId, Key, KeyboardShortcut, Layout, Modifiers,
    PointerButton, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui,
};
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use velvet_math::{Mat4, Vec3};
use velvet_render::Camera3D;
use velvet_scene3d::{
    check_scene_assets, load_scene_from_path, local_matrix, validate_scene, LightDoc, MeshKind3D,
    Scene3D, SceneNode3D, TransformDoc,
};

use crate::code::{self, Lang};
use crate::files::{self, FileTree};
use crate::gizmo::{self, AxisBubble, Frame, Handle, Tool, View};
use crate::scene_ops as ops;
use crate::theme;
use crate::viewport::{self, Aabb, OrbitCamera, Viewport};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Scene,
    Files,
    Viewport,
    Script(PathBuf),
    Image(PathBuf),
    Inspector,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selection {
    None,
    Node(String),
    Light(usize),
    Camera,
    Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Ok,
    Warn,
    Error,
}

impl Level {
    fn color(self) -> Color32 {
        match self {
            Level::Info => theme::TEXT_DIM,
            Level::Ok => theme::OK,
            Level::Warn => theme::WARN,
            Level::Error => theme::ERROR,
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Level::Info => "•",
            Level::Ok => "✔",
            Level::Warn => "⚠",
            Level::Error => "✖",
        }
    }
    fn index(self) -> usize {
        self as usize
    }
}

struct LogLine {
    at: f32,
    level: Level,
    text: String,
}

struct Script {
    path: PathBuf,
    text: String,
    saved: String,
    lang: Lang,
    cursor: (usize, usize),
}

impl Script {
    fn dirty(&self) -> bool {
        self.text != self.saved
    }
}

/// A picture open in its own tab.
struct Picture {
    path: PathBuf,
    size: (u32, u32),
    bytes: u64,
    kind: String,
    pending: Option<egui::ColorImage>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
    /// `None` fits the tab.
    zoom: Option<f32>,
    pan: egui::Vec2,
}

const PICTURE_KINDS: &[&str] = &[".png", ".jpg", ".jpeg", ".avif", ".svg"];

fn is_picture(path: &Path) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    PICTURE_KINDS.iter().any(|k| name.ends_with(k))
}

/// The pixel size an SVG is drawn at: its viewBox's shape, 512 on the long side.
fn svg_raster_size(svg: &str) -> (u32, u32) {
    let shape = svg
        .find("viewBox=\"")
        .map(|i| &svg[i + 9..])
        .and_then(|rest| rest.split('"').next())
        .map(|v| {
            v.split([' ', ','])
                .filter_map(|n| n.trim().parse::<f32>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|n| n.len() == 4 && n[2] > 0.0 && n[3] > 0.0)
        .map(|n| n[2] / n[3])
        .unwrap_or(1.0);
    if shape >= 1.0 {
        (512, (512.0 / shape).round().max(1.0) as u32)
    } else {
        ((512.0 * shape).round().max(1.0) as u32, 512)
    }
}

fn load_picture(path: &Path) -> Picture {
    let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
    let lower = path.to_string_lossy().to_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("").to_uppercase();
    let read: Result<(u32, u32, Vec<u8>), String> = if lower.ends_with(".svg") {
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|svg| {
                let (w, h) = svg_raster_size(&svg);
                velvet_image::rasterize_simple_svg(&svg, w, h)
                    .map(|img| (img.width, img.height, img.pixels))
                    .map_err(|e| e.to_string())
            })
    } else {
        velvet_avif::open_image(path)
            .map(|img| {
                let rgba = img.to_rgba8();
                (rgba.width(), rgba.height(), rgba.into_raw())
            })
            .map_err(|e| e.to_string())
    };
    let mut pic = Picture {
        path: path.to_path_buf(),
        size: (0, 0),
        bytes,
        kind: ext,
        pending: None,
        texture: None,
        error: None,
        zoom: None,
        pan: egui::Vec2::ZERO,
    };
    match read {
        Ok((w, h, pixels)) => {
            pic.size = (w, h);
            // The GPU takes at most 8192 a side; big pictures are shown at 4096.
            let (w, h, pixels) = if w.max(h) > 4096 {
                let k = 4096.0 / w.max(h) as f32;
                let (nw, nh) = (
                    ((w as f32 * k) as u32).max(1),
                    ((h as f32 * k) as u32).max(1),
                );
                match image::RgbaImage::from_raw(w, h, pixels) {
                    Some(img) => {
                        let small = image::imageops::resize(
                            &img,
                            nw,
                            nh,
                            image::imageops::FilterType::Triangle,
                        );
                        (nw, nh, small.into_raw())
                    }
                    None => (0, 0, Vec::new()),
                }
            } else {
                (w, h, pixels)
            };
            if w > 0 && pixels.len() == (w * h * 4) as usize {
                pic.pending = Some(egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &pixels,
                ));
            } else {
                pic.error = Some("la imagen no tiene píxeles".into());
            }
        }
        Err(e) => pic.error = Some(e),
    }
    pic
}

fn human_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", (n as f64 / 1024.0).max(1.0))
    }
}

enum Dialog {
    OpenScene {
        filter: String,
    },
    SaveAs {
        path: String,
    },
    AddModel {
        filter: String,
        at: Option<Vec3>,
        parent: Option<String>,
    },
    Discard(Pending),
    CloseScript(PathBuf),
    OpenScript {
        filter: String,
    },
    Shortcuts,
    About,
}

/// What the script picker lists.
const SCRIPT_KINDS: &[&str] = &[
    ".rs", ".vel", ".vs3", ".vcss", ".md", ".json", ".toml", ".wgsl", ".js", ".mjs", ".ts", ".py",
    ".txt", ".ron", ".yaml", ".yml", ".html", ".css",
];

#[derive(Clone)]
enum Pending {
    New,
    Open(PathBuf),
    Quit,
}

/// A scene node dragged in the tree.
struct DragNode(String);
/// A file dragged from the file system.
struct DragFile(PathBuf);

struct GizmoDrag {
    handle: Handle,
    from: Pos2,
    frame: Frame,
    start: Option<TransformDoc>,
    start_pos: Vec3,
    start_target: Vec3,
    parent_inv: Mat4,
    hit0: Option<Vec3>,
    last_angle: f32,
    turned: f32,
}

enum TreeAct {
    Select(Selection),
    Toggle(String),
    Visible(String),
    StartRename(String),
    Rename(String, String),
    EndRename,
    Duplicate(String),
    Delete(String),
    Focus(Selection),
    AddChild(String, MeshKind3D),
    AddModelDialog(Option<String>),
    ToRoot(String),
    Reparent(String, Option<String>),
    DropFile(PathBuf, Option<String>),
    DeleteLight(usize),
}

enum FileAct {
    Toggle(PathBuf),
    Select(PathBuf),
    Open(PathBuf),
    OpenText(PathBuf),
    AddModel(PathBuf),
    CopyPath(PathBuf),
    Reveal(PathBuf),
}

struct Row {
    path: String,
    depth: usize,
    icon: &'static str,
    color: Color32,
    label: String,
    has_children: bool,
    open: bool,
    hidden: bool,
}

const KIND_NAMES: [&str; 6] = ["Vacío", "Cubo", "Esfera", "Plano", "Terreno", "Modelo 3D"];

fn kind_index(m: &MeshKind3D) -> usize {
    match m {
        MeshKind3D::Empty => 0,
        MeshKind3D::Cube => 1,
        MeshKind3D::Sphere { .. } => 2,
        MeshKind3D::Plane => 3,
        MeshKind3D::Heightfield { .. } => 4,
        MeshKind3D::Gltf { .. } => 5,
    }
}

fn mesh_of_kind(k: usize) -> MeshKind3D {
    match k {
        1 => MeshKind3D::Cube,
        2 => MeshKind3D::Sphere { segments: 32 },
        3 => MeshKind3D::Plane,
        4 => hills(),
        5 => MeshKind3D::Gltf {
            path: String::new(),
        },
        _ => MeshKind3D::Empty,
    }
}

/// A small terrain with gentle hills.
fn hills() -> MeshKind3D {
    let res = 33u32;
    let heights = (0..res * res)
        .map(|i| {
            let x = (i % res) as f32 / (res - 1) as f32 * std::f32::consts::TAU;
            let z = (i / res) as f32 / (res - 1) as f32 * std::f32::consts::TAU;
            (x.sin() * z.cos() * 0.6 + (x * 2.3).cos() * 0.25) * 0.8
        })
        .collect();
    MeshKind3D::Heightfield {
        resolution: res,
        size: 16.0,
        heights,
    }
}

fn kind_color(m: &MeshKind3D) -> Color32 {
    match m {
        MeshKind3D::Gltf { .. } => Color32::from_rgb(0xc4, 0x9c, 0xff),
        MeshKind3D::Cube | MeshKind3D::Sphere { .. } | MeshKind3D::Plane => {
            Color32::from_rgb(0x8c, 0xc8, 0xff)
        }
        MeshKind3D::Heightfield { .. } => Color32::from_rgb(0x8b, 0xd9, 0x4d),
        MeshKind3D::Empty => theme::TEXT_DIM,
    }
}

fn file_color(name: &str, is_dir: bool) -> Color32 {
    if is_dir {
        return Color32::from_rgb(0xe8, 0xc0, 0x6a);
    }
    let n = name.to_ascii_lowercase();
    if n.ends_with(".scene3d.json") {
        return theme::ACCENT;
    }
    match n.rsplit('.').next().unwrap_or("") {
        "glb" | "gltf" => Color32::from_rgb(0xc4, 0x9c, 0xff),
        "rs" | "vel" | "vs3" | "wgsl" | "js" | "mjs" | "ts" | "py" => {
            Color32::from_rgb(0x66, 0xb8, 0xff)
        }
        "png" | "jpg" | "jpeg" | "webp" | "avif" => Color32::from_rgb(0x8b, 0xd9, 0x4d),
        "ogg" | "wav" | "mp3" => Color32::from_rgb(0xff, 0xa0, 0x5c),
        "md" | "txt" | "json" | "toml" => Color32::from_rgb(0xb4, 0xbc, 0xcc),
        _ => theme::TEXT_DIM,
    }
}

fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

/// 1234567 → «1.234.567».
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}

fn default_dock() -> DockState<Tab> {
    let mut dock = DockState::new(vec![Tab::Viewport]);
    let surface = dock.main_surface_mut();
    let [center, _inspector] = surface.split_right(NodeIndex::root(), 0.78, vec![Tab::Inspector]);
    // For a split to the left, `fraction` is the new (left) node's share.
    let [center, left] = surface.split_left(center, 0.25, vec![Tab::Scene]);
    let [_scene, _files] = surface.split_below(left, 0.56, vec![Tab::Files]);
    let [_view, _output] = surface.split_below(center, 0.78, vec![Tab::Output]);
    dock
}

// ------------------------------------------------------------------ small widgets
fn heading(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .family(theme::bold())
            .size(16.0)
            .color(Color32::WHITE),
    );
    ui.add_space(6.0);
}

fn dim(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(theme::TEXT_DIM));
}

fn section(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::CollapsingHeader::new(
        RichText::new(title)
            .family(theme::bold())
            .size(13.0)
            .color(theme::TEXT),
    )
    .id_salt(("section", title))
    .default_open(true)
    .show(ui, |ui| {
        ui.add_space(2.0);
        add(ui);
        ui.add_space(4.0);
    });
}

fn grid(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([10.0, 5.0])
        .min_col_width(74.0)
        .show(ui, add);
}

fn vec3_edit(ui: &mut Ui, v: &mut [f32; 3], speed: f64, suffix: &str, decimals: usize) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let w = ((ui.available_width() - 8.0) / 3.0).clamp(52.0, 96.0);
        for (i, val) in v.iter_mut().enumerate() {
            let r = ui.add_sized(
                [w, 20.0],
                DragValue::new(val)
                    .speed(speed)
                    .max_decimals(decimals)
                    .suffix(suffix),
            );
            let bar = Rect::from_min_size(
                r.rect.left_top() + egui::vec2(1.0, 4.0),
                egui::vec2(2.5, r.rect.height() - 8.0),
            );
            ui.painter().rect_filled(bar, 1.0, theme::AXES[i]);
            changed |= r.changed();
        }
    });
    changed
}

fn rgb_edit(ui: &mut Ui, c: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        let mut shown = *c;
        if ui.color_edit_button_rgb(&mut shown).changed() {
            *c = shown;
        }
        ui.spacing_mut().item_spacing.x = 3.0;
        for (i, v) in c.iter_mut().enumerate() {
            ui.add(
                DragValue::new(v)
                    .speed(0.01)
                    .range(0.0..=8.0)
                    .max_decimals(2)
                    .prefix(["r ", "g ", "b "][i]),
            );
        }
    });
}

fn shadow_text(p: &egui::Painter, pos: Pos2, text: &str, size: f32, color: Color32) {
    let font = FontId::proportional(size);
    p.text(
        pos + egui::vec2(1.0, 1.0),
        Align2::LEFT_TOP,
        text,
        font.clone(),
        Color32::from_black_alpha(170),
    );
    p.text(pos, Align2::LEFT_TOP, text, font, color);
}

fn logo(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 5.0, theme::ACCENT);
    let c = rect.center();
    let s = Stroke::new(2.4_f32, Color32::WHITE);
    p.line_segment([c + egui::vec2(-5.5, -5.0), c + egui::vec2(0.0, 5.5)], s);
    p.line_segment([c + egui::vec2(5.5, -5.0), c + egui::vec2(0.0, 5.5)], s);
}

fn workspace_button(ui: &mut Ui, text: &str, on: bool) -> egui::Response {
    let t = RichText::new(text)
        .size(13.0)
        .color(if on { Color32::WHITE } else { theme::TEXT_DIM });
    ui.add(
        egui::Button::new(t)
            .fill(if on {
                theme::ACCENT_DIM
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(4.0)
            .min_size(egui::vec2(78.0, 22.0)),
    )
}

fn draw_eye(p: &egui::Painter, c: Pos2, open: bool, col: Color32) {
    let half = |sign: f32| -> Vec<Pos2> {
        (0..=12)
            .map(|k| {
                let x = -6.0 + k as f32;
                let y = 3.6 * (1.0 - (x / 6.0).powi(2)).max(0.0).sqrt();
                c + egui::vec2(x, y * sign)
            })
            .collect()
    };
    p.add(egui::Shape::line(half(-1.0), Stroke::new(1.2_f32, col)));
    p.add(egui::Shape::line(half(1.0), Stroke::new(1.2_f32, col)));
    if open {
        p.circle_filled(c, 1.9, col);
    } else {
        p.line_segment(
            [c + egui::vec2(-6.0, 4.5), c + egui::vec2(6.0, -4.5)],
            Stroke::new(1.3_f32, col),
        );
    }
}

struct RowOut {
    resp: egui::Response,
    toggled: bool,
    eye: bool,
    label_rect: Rect,
}

/// One row of a tree: arrow, icon, label and an optional eye toggle at the right.
#[allow(clippy::too_many_arguments)]
fn tree_row(
    ui: &mut Ui,
    id: egui::Id,
    depth: usize,
    arrow: Option<bool>,
    icon: &str,
    icon_color: Color32,
    label: &str,
    selected: bool,
    dimmed: bool,
    eye: Option<bool>,
) -> RowOut {
    let h = 22.0;
    let width = ui.available_width().max(60.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, h), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let bg = rect.shrink2(egui::vec2(2.0, 1.0));
    if selected {
        painter.rect_filled(bg, 3.0, theme::ACCENT_DIM);
    } else if resp.hovered() {
        painter.rect_filled(bg, 3.0, theme::BG_HOVER);
    }
    let x0 = rect.left() + 6.0 + depth as f32 * 14.0;
    let cy = rect.center().y;
    let mut toggled = false;
    if let Some(open) = arrow {
        let ar = Rect::from_center_size(egui::pos2(x0 + 6.0, cy), egui::vec2(14.0, h));
        let a = ui.interact(ar, id.with("arrow"), Sense::click());
        toggled = a.clicked();
        let c = ar.center();
        let col = if a.hovered() {
            Color32::WHITE
        } else {
            theme::TEXT_DIM
        };
        let pts = if open {
            vec![
                c + egui::vec2(-4.0, -2.0),
                c + egui::vec2(4.0, -2.0),
                c + egui::vec2(0.0, 3.0),
            ]
        } else {
            vec![
                c + egui::vec2(-2.0, -4.0),
                c + egui::vec2(3.0, 0.0),
                c + egui::vec2(-2.0, 4.0),
            ]
        };
        painter.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
    }
    painter.text(
        egui::pos2(x0 + 22.0, cy),
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(13.0),
        icon_color,
    );
    let label_rect = Rect::from_min_max(
        egui::pos2(x0 + 32.0, rect.top()),
        egui::pos2((rect.right() - 24.0).max(x0 + 60.0), rect.bottom()),
    );
    let text_col = if selected {
        Color32::WHITE
    } else if dimmed {
        theme::TEXT_DIM.gamma_multiply(0.75)
    } else {
        theme::TEXT
    };
    painter.text(
        egui::pos2(label_rect.left(), cy),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        text_col,
    );
    let mut eye_clicked = false;
    if let Some(visible) = eye {
        let er =
            Rect::from_center_size(egui::pos2(rect.right() - 13.0, cy), egui::vec2(18.0, 18.0));
        let e = ui.interact(er, id.with("eye"), Sense::click());
        eye_clicked = e.clicked();
        let col = if e.hovered() {
            Color32::WHITE
        } else if visible {
            theme::TEXT_DIM
        } else {
            theme::TEXT_DIM.gamma_multiply(0.5)
        };
        draw_eye(&painter, er.center(), visible, col);
    }
    RowOut {
        resp,
        toggled,
        eye: eye_clicked,
        label_rect,
    }
}

/// Outlines a tree row a node or a file is about to be dropped on, and reports the drop.
fn drop_target(ui: &Ui, resp: &egui::Response, parent: Option<&str>, acts: &mut Vec<TreeAct>) {
    let hovering = resp
        .dnd_hover_payload::<DragNode>()
        .is_some_and(|n| Some(n.0.as_str()) != parent)
        || resp.dnd_hover_payload::<DragFile>().is_some();
    if hovering {
        ui.painter().rect_stroke(
            resp.rect.shrink(1.0),
            3.0,
            Stroke::new(1.5_f32, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(n) = resp.dnd_release_payload::<DragNode>() {
        acts.push(TreeAct::Reparent(n.0.clone(), parent.map(str::to_string)));
    }
    if let Some(f) = resp.dnd_release_payload::<DragFile>() {
        acts.push(TreeAct::DropFile(f.0.clone(), parent.map(str::to_string)));
    }
}

fn walk_rows(
    nodes: &[SceneNode3D],
    parent: Option<&str>,
    depth: usize,
    filter: &str,
    collapsed: &HashSet<String>,
    out: &mut Vec<Row>,
) {
    fn matches(n: &SceneNode3D, f: &str) -> bool {
        n.id.to_lowercase().contains(f) || n.children.iter().any(|c| matches(c, f))
    }
    for n in nodes {
        if !filter.is_empty() && !matches(n, filter) {
            continue;
        }
        let path = ops::join(parent, &n.id);
        let open = !filter.is_empty() || !collapsed.contains(&path);
        let (icon, _) = ops::kind_label(&n.mesh);
        out.push(Row {
            path: path.clone(),
            depth,
            icon,
            color: kind_color(&n.mesh),
            label: n.id.clone(),
            has_children: !n.children.is_empty(),
            open,
            hidden: n.disabled,
        });
        if open {
            walk_rows(&n.children, Some(&path), depth + 1, filter, collapsed, out);
        }
    }
}

// ------------------------------------------------------------------ state
pub struct State {
    pub root: PathBuf,
    pub scene: Scene3D,
    pub scene_path: Option<PathBuf>,
    committed: Scene3D,
    saved: Scene3D,
    undo: Vec<Scene3D>,
    redo: Vec<Scene3D>,
    pub selection: Selection,
    pub tool: Tool,
    pub snap: bool,
    snap_move: f32,
    snap_rot: f32,
    snap_scale: f32,
    pub cam: OrbitCamera,
    pub use_scene_camera: bool,
    pub show_grid: bool,
    pub show_bounds: bool,
    pub show_stats: bool,
    gdrag: Option<GizmoDrag>,
    hot: Option<Handle>,
    icon_hits: Vec<(Pos2, Selection)>,
    axis_bubbles: Vec<AxisBubble>,
    ctx_point: Option<Vec3>,
    flying: bool,
    pub viewport: Option<Viewport>,
    viewport_visible: bool,
    viewport_px: (u32, u32),
    files: FileTree,
    file_filter: String,
    file_search: Option<(String, Vec<PathBuf>)>,
    expanded: HashSet<PathBuf>,
    selected_file: Option<PathBuf>,
    tree_filter: String,
    collapsed: HashSet<String>,
    renaming: Option<(String, String, bool)>,
    name_edit: Option<(String, String)>,
    scripts: Vec<Script>,
    pictures: Vec<Picture>,
    checker: Option<egui::TextureHandle>,
    focused_script: Option<PathBuf>,
    log: Vec<LogLine>,
    log_filter: [bool; 4],
    started: Instant,
    dialog: Option<Dialog>,
    models: Option<Vec<String>>,
    scenes: Option<Vec<String>>,
    script_files: Option<Vec<String>>,
    script_button: Option<Rect>,
    open_tabs: Vec<Tab>,
    focus_tab: Option<Tab>,
    close_tabs: Vec<Tab>,
    reset_layout: bool,
    last_view: Option<(Camera3D, Rect)>,
    row_rects: std::collections::HashMap<String, Rect>,
    script_rect: Option<Rect>,
    pub gpu_name: String,
    pub quit: bool,
    repaint: bool,
}

impl State {
    fn new(root: PathBuf) -> Self {
        let scene = ops::default_scene();
        let cam = OrbitCamera::looking(v3(scene.camera.position), v3(scene.camera.target), 50.0);
        Self {
            root,
            committed: scene.clone(),
            saved: scene.clone(),
            scene,
            scene_path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            selection: Selection::None,
            tool: Tool::Move,
            snap: false,
            snap_move: 0.25,
            snap_rot: 15.0,
            snap_scale: 0.1,
            cam,
            use_scene_camera: false,
            show_grid: true,
            show_bounds: false,
            show_stats: true,
            gdrag: None,
            hot: None,
            icon_hits: Vec::new(),
            axis_bubbles: Vec::new(),
            ctx_point: None,
            flying: false,
            viewport: None,
            viewport_visible: false,
            viewport_px: (960, 540),
            files: FileTree::default(),
            file_filter: String::new(),
            file_search: None,
            expanded: HashSet::new(),
            selected_file: None,
            tree_filter: String::new(),
            collapsed: HashSet::new(),
            renaming: None,
            name_edit: None,
            scripts: Vec::new(),
            pictures: Vec::new(),
            checker: None,
            focused_script: None,
            log: Vec::new(),
            log_filter: [true; 4],
            started: Instant::now(),
            dialog: None,
            models: None,
            scenes: None,
            script_files: None,
            script_button: None,
            open_tabs: Vec::new(),
            focus_tab: None,
            close_tabs: Vec::new(),
            reset_layout: false,
            last_view: None,
            row_rects: std::collections::HashMap::new(),
            script_rect: None,
            gpu_name: String::new(),
            quit: false,
            repaint: false,
        }
    }

    pub fn log(&mut self, level: Level, text: impl Into<String>) {
        let text = text.into();
        eprintln!("[editor] {text}");
        self.log.push(LogLine {
            at: self.started.elapsed().as_secs_f32(),
            level,
            text,
        });
        if self.log.len() > 2000 {
            self.log.drain(..500);
        }
    }

    pub fn dirty(&self) -> bool {
        self.scene != self.saved
    }

    pub fn title(&self) -> String {
        let project = self
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        format!(
            "{}{} — {} — Velvet Editor",
            if self.dirty() { "● " } else { "" },
            self.scene.name,
            project
        )
    }

    // -------------------------------------------------------------- scene files
    fn set_scene(&mut self, scene: Scene3D, path: Option<PathBuf>) {
        let c = &scene.camera;
        self.cam = OrbitCamera::looking(v3(c.position), v3(c.target), c.fov_deg.clamp(25.0, 80.0));
        self.committed = scene.clone();
        self.saved = scene.clone();
        self.scene = scene;
        self.scene_path = path;
        self.undo.clear();
        self.redo.clear();
        self.selection = Selection::None;
        self.gdrag = None;
        self.name_edit = None;
        self.renaming = None;
        self.collapsed.clear();
    }

    pub fn open_scene(&mut self, path: &Path) {
        match load_scene_from_path(path) {
            Ok(scene) => {
                let rel = files::rel(&self.root, path);
                let report = check_scene_assets(&self.root, &scene);
                self.set_scene(scene, Some(path.to_path_buf()));
                let n = ops::count(&self.scene.nodes);
                self.log(Level::Ok, format!("Escena abierta: {rel} ({n} nodos)"));
                if !report.is_ok() {
                    for line in report.to_text().lines() {
                        self.log(Level::Warn, line.to_string());
                    }
                }
            }
            Err(e) => self.log(
                Level::Error,
                format!("No se pudo abrir {}: {e}", path.display()),
            ),
        }
    }

    fn request(&mut self, then: Pending) {
        if self.dirty() {
            self.dialog = Some(Dialog::Discard(then));
        } else {
            self.run_pending(then);
        }
    }

    fn run_pending(&mut self, then: Pending) {
        match then {
            Pending::New => {
                self.set_scene(ops::default_scene(), None);
                self.log(Level::Info, "Escena nueva");
            }
            Pending::Open(p) => self.open_scene(&p),
            Pending::Quit => self.quit = true,
        }
    }

    pub fn request_quit(&mut self) {
        self.request(Pending::Quit);
    }

    fn save(&mut self) -> bool {
        match self.scene_path.clone() {
            Some(p) => self.save_to(&p),
            None => {
                self.open_save_as();
                false
            }
        }
    }

    fn open_save_as(&mut self) {
        let path = match &self.scene_path {
            Some(p) => files::rel(&self.root, p),
            None => format!("scenes/{}.scene3d.json", self.scene.name),
        };
        self.dialog = Some(Dialog::SaveAs { path });
    }

    fn save_to(&mut self, path: &Path) -> bool {
        if let Err(e) = validate_scene(&self.scene) {
            self.log(Level::Warn, format!("Aviso al validar: {e}"));
        }
        let json = match serde_json::to_string_pretty(&self.scene) {
            Ok(j) => j + "\n",
            Err(e) => {
                self.log(Level::Error, format!("No se pudo convertir a JSON: {e}"));
                return false;
            }
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(path, json) {
            Ok(()) => {
                self.saved = self.scene.clone();
                self.scene_path = Some(path.to_path_buf());
                self.files.refresh();
                self.scenes = None;
                self.file_search = None;
                let rel = files::rel(&self.root, path);
                self.log(Level::Ok, format!("Guardada: {rel}"));
                true
            }
            Err(e) => {
                self.log(
                    Level::Error,
                    format!("No se pudo guardar {}: {e}", path.display()),
                );
                false
            }
        }
    }

    fn check_assets(&mut self) {
        let report = check_scene_assets(&self.root, &self.scene);
        if report.is_ok() {
            self.log(
                Level::Ok,
                format!("Recursos correctos: {} modelos comprobados", report.checked),
            );
        } else {
            for line in report.to_text().lines() {
                self.log(Level::Warn, line.to_string());
            }
        }
        match validate_scene(&self.scene) {
            Ok(()) => self.log(Level::Ok, "La escena es válida"),
            Err(e) => self.log(Level::Error, format!("La escena no es válida: {e}")),
        }
    }

    // -------------------------------------------------------------- history
    fn commit(&mut self) {
        if self.scene != self.committed {
            let prev = std::mem::replace(&mut self.committed, self.scene.clone());
            self.undo.push(prev);
            if self.undo.len() > 300 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    fn undo(&mut self) {
        self.commit();
        if let Some(prev) = self.undo.pop() {
            let now = std::mem::replace(&mut self.scene, prev);
            self.redo.push(now);
            self.committed = self.scene.clone();
            self.fix_selection();
            self.log(Level::Info, "Deshecho");
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let now = std::mem::replace(&mut self.scene, next);
            self.undo.push(now);
            self.committed = self.scene.clone();
            self.fix_selection();
            self.log(Level::Info, "Rehecho");
        }
    }

    fn fix_selection(&mut self) {
        let keep = match &self.selection {
            Selection::Node(p) => ops::node(&self.scene.nodes, p).is_some(),
            Selection::Light(i) => *i < self.scene.lights.len(),
            _ => true,
        };
        if !keep {
            self.selection = Selection::None;
        }
    }

    // -------------------------------------------------------------- lookups
    fn models_list(&mut self) -> &Vec<String> {
        if self.models.is_none() {
            self.models = Some(files::find(&self.root, &[".glb", ".gltf"], 5000));
        }
        self.models.get_or_insert_with(Vec::new)
    }

    fn script_files_list(&mut self) -> &Vec<String> {
        if self.script_files.is_none() {
            self.script_files = Some(files::find(&self.root, SCRIPT_KINDS, 8000));
        }
        self.script_files.get_or_insert_with(Vec::new)
    }

    fn open_script_picker(&mut self) {
        self.dialog = Some(Dialog::OpenScript {
            filter: String::new(),
        });
    }

    fn scenes_list(&mut self) -> &Vec<String> {
        if self.scenes.is_none() {
            self.scenes = Some(files::find(&self.root, &[".scene3d.json"], 2000));
        }
        self.scenes.get_or_insert_with(Vec::new)
    }

    /// A node's world matrix, straight from the document.
    fn world_of(&self, path: &str) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        let mut list = &self.scene.nodes;
        for part in path.split('/') {
            match list.iter().find(|n| n.id == part) {
                Some(n) => {
                    m = m * local_matrix(&n.transform);
                    list = &n.children;
                }
                None => break,
            }
        }
        m
    }

    fn parent_world(&self, path: &str) -> Mat4 {
        ops::parent_of(path).map_or(Mat4::IDENTITY, |p| self.world_of(p))
    }

    fn render_camera(&self) -> Camera3D {
        if self.use_scene_camera {
            viewport::camera_from_doc(&self.scene)
        } else {
            self.cam.camera()
        }
    }

    /// Where new things land: on whatever is under the camera's target.
    fn drop_point(&self) -> Vec3 {
        let t = self.cam.target;
        self.viewport
            .as_ref()
            .and_then(|v| v.surface_point(t + Vec3::Y * 200.0, Vec3::new(0.0, -1.0, 0.0)))
            .unwrap_or(Vec3::new(t.x, 0.0, t.z))
    }

    // -------------------------------------------------------------- edits
    fn add_primitive(&mut self, kind: MeshKind3D, parent: Option<String>, at: Option<Vec3>) {
        let (name, lift, scale) = match &kind {
            MeshKind3D::Cube => ("cubo", 0.5, [1.0; 3]),
            MeshKind3D::Sphere { .. } => ("esfera", 0.5, [0.5; 3]),
            MeshKind3D::Plane => ("plano", 0.0, [4.0, 1.0, 4.0]),
            MeshKind3D::Heightfield { .. } => ("terreno", 0.0, [1.0; 3]),
            MeshKind3D::Empty => ("nodo", 0.0, [1.0; 3]),
            MeshKind3D::Gltf { .. } => ("modelo", 0.0, [1.0; 3]),
        };
        let tint = matches!(kind, MeshKind3D::Cube | MeshKind3D::Sphere { .. });
        let mut node = ops::new_node(name, kind);
        if parent.is_none() {
            let p = at.unwrap_or_else(|| self.drop_point());
            node.transform.position = [p.x, p.y + lift, p.z];
        }
        node.transform.scale = scale;
        if tint {
            node.material.base_color = [0.62, 0.45, 0.95, 1.0];
        }
        match ops::add_node(&mut self.scene, parent.as_deref(), node) {
            Some(p) => {
                self.log(Level::Info, format!("Añadido «{p}»"));
                self.selection = Selection::Node(p);
            }
            None => self.log(Level::Warn, "No se pudo añadir el nodo"),
        }
    }

    fn add_model(&mut self, rel: &str, parent: Option<String>, at: Option<Vec3>) {
        let stem = Path::new(rel)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "modelo".into());
        let mut node = ops::new_node(
            &stem,
            MeshKind3D::Gltf {
                path: rel.to_string(),
            },
        );
        if parent.is_none() {
            let p = at.unwrap_or_else(|| self.drop_point());
            node.transform.position = p.to_array();
        }
        if let Some(p) = ops::add_node(&mut self.scene, parent.as_deref(), node) {
            self.log(Level::Info, format!("Añadido el modelo «{p}» ({rel})"));
            self.selection = Selection::Node(p);
        }
    }

    fn add_point_light(&mut self, at: Option<Vec3>) {
        let p = at.unwrap_or_else(|| self.drop_point()) + Vec3::Y * 2.5;
        self.scene.lights.push(LightDoc::Point {
            position: p.to_array(),
            color: [1.0, 0.78, 0.5],
            intensity: 2.5,
            radius: 7.0,
        });
        self.selection = Selection::Light(self.scene.lights.len() - 1);
        self.log(Level::Info, "Añadida una luz puntual");
    }

    fn add_sun(&mut self) {
        if self
            .scene
            .lights
            .iter()
            .any(|l| matches!(l, LightDoc::Directional { .. }))
        {
            self.log(
                Level::Warn,
                "Ya hay una luz direccional: el visor usa solo la primera.",
            );
        }
        self.scene.lights.push(LightDoc::Directional {
            direction: [-0.48, 0.82, 0.31],
            key_color: [1.05, 0.82, 0.62],
            ambient_color: [0.16, 0.12, 0.28],
            shadow_extent: 14.0,
        });
        self.selection = Selection::Light(self.scene.lights.len() - 1);
    }

    fn drop_file(&mut self, path: &Path, parent: Option<String>, at: Option<Vec3>) {
        let name = path.to_string_lossy().to_lowercase();
        if name.ends_with(".glb") || name.ends_with(".gltf") {
            let rel = files::rel(&self.root, path);
            self.add_model(&rel, parent, at);
        } else if name.ends_with(".scene3d.json") {
            self.request(Pending::Open(path.to_path_buf()));
        } else {
            self.log(
                Level::Info,
                "Solo se pueden soltar modelos .glb o escenas .scene3d.json",
            );
        }
    }

    fn delete_selected(&mut self) {
        match self.selection.clone() {
            Selection::Node(p) => {
                if ops::remove_node(&mut self.scene, &p).is_some() {
                    self.log(Level::Info, format!("Borrado «{p}»"));
                    self.selection = Selection::None;
                }
            }
            Selection::Light(i) if i < self.scene.lights.len() => {
                self.scene.lights.remove(i);
                self.selection = Selection::None;
                self.log(Level::Info, "Luz borrada");
            }
            _ => {}
        }
    }

    fn duplicate_selected(&mut self) {
        match self.selection.clone() {
            Selection::Node(p) => {
                if let Some(np) = ops::duplicate(&mut self.scene, &p) {
                    self.log(Level::Info, format!("Duplicado «{p}» → «{np}»"));
                    self.selection = Selection::Node(np);
                }
            }
            Selection::Light(i) => {
                if let Some(mut l) = self.scene.lights.get(i).cloned() {
                    if let LightDoc::Point { position, .. } = &mut l {
                        position[0] += 1.0;
                    }
                    self.scene.lights.push(l);
                    self.selection = Selection::Light(self.scene.lights.len() - 1);
                }
            }
            _ => {}
        }
    }

    fn focus_selected(&mut self) {
        let one = Vec3::new(1.0, 1.0, 1.0);
        let around = |c: Vec3| Aabb {
            min: c - one,
            max: c + one,
        };
        let b = match &self.selection {
            Selection::Node(p) => self
                .viewport
                .as_ref()
                .and_then(|v| v.bounds.get(p).copied()),
            Selection::Light(i) => match self.scene.lights.get(*i) {
                Some(LightDoc::Point { position, .. }) => Some(around(v3(*position))),
                _ => None,
            },
            Selection::Camera => Some(around(v3(self.scene.camera.position))),
            _ => self.viewport.as_ref().map(|v| v.scene_bounds()),
        };
        if let Some(b) = b {
            self.cam.frame(&b);
            self.use_scene_camera = false;
        }
    }

    fn start_rename(&mut self) {
        if let Selection::Node(p) = &self.selection {
            self.renaming = Some((p.clone(), ops::leaf(p).to_string(), false));
        }
    }

    fn rename(&mut self, path: &str, wanted: &str) {
        let wanted = wanted.trim();
        if wanted.is_empty() || ops::leaf(path) == wanted {
            return;
        }
        if let Some(np) = ops::rename(&mut self.scene, path, wanted) {
            if self.selection == Selection::Node(path.to_string()) {
                self.selection = Selection::Node(np.clone());
            }
            self.log(Level::Info, format!("Renombrado «{path}» → «{np}»"));
        }
    }

    fn reparent(&mut self, path: &str, to: Option<&str>) {
        if let Some(np) = ops::reparent(&mut self.scene, path, to) {
            self.log(
                Level::Info,
                format!(
                    "«{}» ahora cuelga de {}",
                    ops::leaf(&np),
                    to.map_or("la raíz".to_string(), |t| format!("«{t}»"))
                ),
            );
            self.selection = Selection::Node(np);
        }
    }

    // -------------------------------------------------------------- scripts and files
    pub fn open_script(&mut self, path: &Path) {
        if !self.scripts.iter().any(|s| s.path == path) {
            if std::fs::metadata(path).is_ok_and(|m| m.len() > 3 * 1024 * 1024) {
                self.log(
                    Level::Warn,
                    format!(
                        "{} es demasiado grande para el editor de texto",
                        path.display()
                    ),
                );
                return;
            }
            match std::fs::read_to_string(path) {
                Ok(text) => self.scripts.push(Script {
                    path: path.to_path_buf(),
                    saved: text.clone(),
                    text,
                    lang: Lang::of(path),
                    cursor: (1, 1),
                }),
                Err(e) => {
                    self.log(
                        Level::Error,
                        format!("No se pudo abrir {}: {e}", path.display()),
                    );
                    return;
                }
            }
        }
        self.open_tabs.push(Tab::Script(path.to_path_buf()));
    }

    pub fn open_picture(&mut self, path: &Path) {
        if !self.pictures.iter().any(|p| p.path == path) {
            let pic = load_picture(path);
            let rel = files::rel(&self.root, path);
            match &pic.error {
                Some(e) => self.log(
                    Level::Warn,
                    format!("No se pudo abrir la imagen {rel}: {e}"),
                ),
                None => self.log(
                    Level::Info,
                    format!("Imagen: {rel} ({} × {} px)", pic.size.0, pic.size.1),
                ),
            }
            self.pictures.push(pic);
        }
        self.open_tabs.push(Tab::Image(path.to_path_buf()));
    }

    fn image_tab(&mut self, ui: &mut Ui, path: &Path) {
        let Some(i) = self.pictures.iter().position(|p| p.path == path) else {
            dim(ui, "Esta imagen ya no está abierta.");
            return;
        };
        let rel = files::rel(&self.root, path);
        let checker = self
            .checker
            .get_or_insert_with(|| {
                let (a, b) = (Color32::from_gray(58), Color32::from_gray(44));
                let img = egui::ColorImage {
                    size: [2, 2],
                    pixels: vec![a, b, b, a],
                };
                ui.ctx().load_texture(
                    "editor-checker",
                    img,
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Nearest,
                        wrap_mode: egui::TextureWrapMode::Repeat,
                        mipmap_mode: None,
                    },
                )
            })
            .id();
        let pic = &mut self.pictures[i];
        if let Some(img) = pic.pending.take() {
            pic.texture = Some(ui.ctx().load_texture(
                rel.clone(),
                img,
                egui::TextureOptions::LINEAR,
            ));
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("🖼").color(file_color(&rel, false)));
            ui.label(
                RichText::new(&rel)
                    .monospace()
                    .size(12.0)
                    .color(theme::TEXT),
            );
            if pic.error.is_none() {
                ui.label(
                    RichText::new(format!(
                        "{} × {} px · {} · {}",
                        pic.size.0,
                        pic.size.1,
                        pic.kind,
                        human_bytes(pic.bytes)
                    ))
                    .size(12.0)
                    .color(theme::TEXT_DIM),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("100 %").clicked() {
                    pic.zoom = Some(1.0);
                    pic.pan = egui::Vec2::ZERO;
                }
                if ui.button("Ajustar").clicked() {
                    pic.zoom = None;
                    pic.pan = egui::Vec2::ZERO;
                }
            });
        });
        ui.add_space(3.0);
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        if rect.width() < 8.0 || rect.height() < 8.0 {
            return;
        }
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let p = ui.painter_at(rect);
        // The checkerboard: one 2×2 texture repeated, 16 px a square.
        let uv = Rect::from_min_max(Pos2::ZERO, (rect.size() / 32.0).to_pos2());
        p.image(checker, rect, uv, Color32::WHITE);
        if let Some(err) = &pic.error {
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("No se puede mostrar: {err}"),
                FontId::proportional(14.0),
                theme::ERROR,
            );
            return;
        }
        let Some(tex) = pic.texture.as_ref() else {
            return;
        };
        // 100 % = one picture pixel per screen pixel.
        let natural = tex.size_vec2() / ui.ctx().pixels_per_point();
        let fit = ((rect.width() - 24.0) / natural.x)
            .min((rect.height() - 24.0) / natural.y)
            .clamp(0.01, 8.0);
        let zoom = pic.zoom.unwrap_or(fit);
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let new = (zoom * (1.0 + scroll * 0.0015)).clamp(0.02, 32.0);
                // Keep the spot under the pointer where it is.
                if let Some(m) = resp.hover_pos() {
                    let centre = rect.center() + pic.pan;
                    let spot = (m - centre) / zoom;
                    pic.pan = m - rect.center() - spot * new;
                }
                pic.zoom = Some(new);
            }
        }
        if resp.dragged() {
            pic.pan += resp.drag_delta();
            pic.zoom.get_or_insert(zoom);
        }
        if resp.double_clicked() {
            if pic.zoom.is_some() {
                pic.zoom = None;
                pic.pan = egui::Vec2::ZERO;
            } else {
                pic.zoom = Some(1.0);
            }
        }
        let zoom = pic.zoom.unwrap_or(fit);
        let shown = Rect::from_center_size(rect.center() + pic.pan, natural * zoom);
        p.image(
            tex.id(),
            shown,
            Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        p.rect_stroke(
            shown,
            0.0,
            Stroke::new(1.0_f32, theme::LINE),
            egui::StrokeKind::Outside,
        );
        shadow_text(
            &p,
            rect.left_bottom() + egui::vec2(10.0, -24.0),
            &format!(
                "{:.0} % · rueda: zoom · arrastrar: mover · doble clic: ajustar/100 %",
                zoom * 100.0
            ),
            12.0,
            theme::TEXT_DIM,
        );
    }

    fn save_script(&mut self, path: &Path) {
        let Some(i) = self.scripts.iter().position(|s| s.path == path) else {
            return;
        };
        let rel = files::rel(&self.root, path);
        match std::fs::write(path, &self.scripts[i].text) {
            Ok(()) => {
                self.scripts[i].saved = self.scripts[i].text.clone();
                self.log(Level::Ok, format!("Guardado: {rel}"));
                if self.scene_path.as_deref() == Some(path) {
                    self.log(
                        Level::Info,
                        "Es la escena abierta: Escena → Recargar desde el disco para verla",
                    );
                }
            }
            Err(e) => self.log(Level::Error, format!("No se pudo guardar {rel}: {e}")),
        }
    }

    fn open_file(&mut self, path: &Path) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name.ends_with(".scene3d.json") {
            self.request(Pending::Open(path.to_path_buf()));
        } else if name.ends_with(".glb") || name.ends_with(".gltf") {
            let rel = files::rel(&self.root, path);
            self.add_model(&rel, None, None);
        } else if is_picture(path) {
            self.open_picture(path);
        } else if Lang::is_text(path) {
            self.open_script(path);
        } else {
            self.log(Level::Info, format!("Sin vista previa para «{name}»"));
        }
    }

    // -------------------------------------------------------------- per frame
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        let cmd_shift = |k| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, k);
        if ctx.input_mut(|i| i.consume_shortcut(&cmd_shift(Key::S))) {
            self.open_save_as();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::S))) {
            match self.focused_script.clone() {
                Some(p) => self.save_script(&p),
                None => {
                    self.save();
                }
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::O))) {
            self.dialog = Some(Dialog::OpenScene {
                filter: String::new(),
            });
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::P))) {
            self.open_script_picker();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::N))) {
            self.request(Pending::New);
        }
        if self.dialog.is_some() || ctx.wants_keyboard_input() {
            return;
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&cmd_shift(Key::Z)) || i.consume_shortcut(&cmd(Key::Y))
        }) {
            self.redo();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::Z))) {
            self.undo();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::D))) {
            self.duplicate_selected();
        }
        if self.flying {
            return;
        }
        let pressed = |k| ctx.input(|i| i.key_pressed(k) && i.modifiers.is_none());
        for (key, tool) in [
            (Key::Q, Tool::Select),
            (Key::W, Tool::Move),
            (Key::E, Tool::Rotate),
            (Key::R, Tool::Scale),
        ] {
            if pressed(key) {
                self.tool = tool;
            }
        }
        if pressed(Key::F) {
            self.focus_selected();
        }
        if pressed(Key::Delete) {
            self.delete_selected();
        }
        if pressed(Key::F2) {
            self.start_rename();
        }
    }

    fn end_frame(&mut self, ctx: &egui::Context) {
        let busy = self.gdrag.is_some()
            || ctx.input(|i| i.pointer.any_down())
            || ctx.wants_keyboard_input();
        if !busy {
            self.commit();
        }
    }

    // -------------------------------------------------------------- menus
    fn menu_bar(&mut self, ctx: &egui::Context, viewport_focused: bool) {
        egui::TopBottomPanel::top("menu")
            .frame(
                egui::Frame::default()
                    .fill(theme::BG_DARK)
                    .inner_margin(egui::Margin::symmetric(8, 5)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    logo(ui);
                    ui.add_space(4.0);
                    ui.menu_button("Escena", |ui| self.scene_menu(ui));
                    ui.menu_button("Editar", |ui| self.edit_menu(ui));
                    ui.menu_button("Añadir", |ui| self.add_menu(ui, None, None));
                    ui.menu_button("Ver", |ui| self.view_menu(ui));
                    ui.menu_button("Ayuda", |ui| {
                        if ui.button("Atajos de teclado").clicked() {
                            self.dialog = Some(Dialog::Shortcuts);
                            ui.close_menu();
                        }
                        if ui.button("Acerca de Velvet Editor").clicked() {
                            self.dialog = Some(Dialog::About);
                            ui.close_menu();
                        }
                    });
                    ui.add_space(20.0);
                    let script_on = self.focused_script.is_some();
                    if workspace_button(ui, "🎬  3D", viewport_focused).clicked() {
                        self.focus_tab = Some(Tab::Viewport);
                    }
                    let script_button = workspace_button(ui, "📜  Script", script_on);
                    self.script_button = Some(script_button.rect);
                    if script_button
                        .on_hover_text("El editor de scripts (Ctrl+P busca uno)")
                        .clicked()
                    {
                        match self.scripts.last() {
                            Some(s) => self.focus_tab = Some(Tab::Script(s.path.clone())),
                            None => self.open_script_picker(),
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("Velvet Engine {}", env!("CARGO_PKG_VERSION")))
                                .color(theme::TEXT_DIM)
                                .size(12.0),
                        );
                        ui.add_space(12.0);
                        let name = format!(
                            "{}{}",
                            self.scene.name,
                            if self.dirty() { "  ●" } else { "" }
                        );
                        ui.label(RichText::new(name).color(theme::TEXT).family(theme::bold()));
                        ui.label(RichText::new("Escena").color(theme::TEXT_DIM).size(12.0));
                    });
                });
            });
    }

    fn scene_menu(&mut self, ui: &mut Ui) {
        if ui
            .add(egui::Button::new("Nueva escena").shortcut_text("Ctrl+N"))
            .clicked()
        {
            self.request(Pending::New);
            ui.close_menu();
        }
        if ui
            .add(egui::Button::new("Abrir escena…").shortcut_text("Ctrl+O"))
            .clicked()
        {
            self.dialog = Some(Dialog::OpenScene {
                filter: String::new(),
            });
            ui.close_menu();
        }
        if ui
            .add(egui::Button::new("Abrir script…").shortcut_text("Ctrl+P"))
            .clicked()
        {
            self.open_script_picker();
            ui.close_menu();
        }
        ui.menu_button("Escenas del proyecto", |ui| {
            let list = self.scenes_list().clone();
            if list.is_empty() {
                dim(ui, "No hay escenas .scene3d.json");
            }
            ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                for rel in list.iter().take(80) {
                    if ui.button(rel).clicked() {
                        let p = self.root.join(rel);
                        self.request(Pending::Open(p));
                        ui.close_menu();
                    }
                }
            });
        });
        ui.separator();
        if ui
            .add(egui::Button::new("Guardar").shortcut_text("Ctrl+S"))
            .clicked()
        {
            self.save();
            ui.close_menu();
        }
        if ui
            .add(egui::Button::new("Guardar como…").shortcut_text("Ctrl+Mayús+S"))
            .clicked()
        {
            self.open_save_as();
            ui.close_menu();
        }
        ui.separator();
        if ui
            .add_enabled(
                self.scene_path.is_some(),
                egui::Button::new("Recargar desde el disco"),
            )
            .clicked()
        {
            if let Some(p) = self.scene_path.clone() {
                self.request(Pending::Open(p));
            }
            ui.close_menu();
        }
        if ui.button("Recargar modelos").clicked() {
            if let Some(v) = self.viewport.as_mut() {
                v.reload_models();
            }
            self.models = None;
            self.log(Level::Info, "Los modelos se vuelven a leer del disco");
            ui.close_menu();
        }
        if ui.button("Comprobar recursos").clicked() {
            self.check_assets();
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Salir").clicked() {
            self.request_quit();
            ui.close_menu();
        }
    }

    fn edit_menu(&mut self, ui: &mut Ui) {
        let can_undo = !self.undo.is_empty() || self.scene != self.committed;
        if ui
            .add_enabled(
                can_undo,
                egui::Button::new("Deshacer").shortcut_text("Ctrl+Z"),
            )
            .clicked()
        {
            self.undo();
            ui.close_menu();
        }
        if ui
            .add_enabled(
                !self.redo.is_empty(),
                egui::Button::new("Rehacer").shortcut_text("Ctrl+Y"),
            )
            .clicked()
        {
            self.redo();
            ui.close_menu();
        }
        ui.separator();
        let any = matches!(self.selection, Selection::Node(_) | Selection::Light(_));
        let node = matches!(self.selection, Selection::Node(_));
        if ui
            .add_enabled(any, egui::Button::new("Duplicar").shortcut_text("Ctrl+D"))
            .clicked()
        {
            self.duplicate_selected();
            ui.close_menu();
        }
        if ui
            .add_enabled(node, egui::Button::new("Renombrar").shortcut_text("F2"))
            .clicked()
        {
            self.start_rename();
            ui.close_menu();
        }
        if ui
            .add_enabled(any, egui::Button::new("Borrar").shortcut_text("Supr"))
            .clicked()
        {
            self.delete_selected();
            ui.close_menu();
        }
        ui.separator();
        if ui
            .add(egui::Button::new("Enfocar la selección").shortcut_text("F"))
            .clicked()
        {
            self.focus_selected();
            ui.close_menu();
        }
        ui.separator();
        for t in [Tool::Select, Tool::Move, Tool::Rotate, Tool::Scale] {
            if ui
                .add(
                    egui::Button::new(t.label())
                        .shortcut_text(t.key())
                        .selected(self.tool == t),
                )
                .clicked()
            {
                self.tool = t;
                ui.close_menu();
            }
        }
        ui.checkbox(&mut self.snap, "Ajuste a la rejilla");
    }

    fn view_menu(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.show_grid, "Rejilla");
        ui.checkbox(&mut self.show_bounds, "Cajas de todos los nodos");
        ui.checkbox(&mut self.show_stats, "Estadísticas del visor");
        ui.checkbox(&mut self.use_scene_camera, "Ver por la cámara de la escena");
        ui.separator();
        if ui.button("Encuadrar toda la escena").clicked() {
            if let Some(b) = self.viewport.as_ref().map(|v| v.scene_bounds()) {
                self.cam.frame(&b);
                self.use_scene_camera = false;
            }
            ui.close_menu();
        }
        if ui
            .button("Restablecer la distribución de paneles")
            .clicked()
        {
            self.reset_layout = true;
            ui.close_menu();
        }
    }

    fn add_menu(&mut self, ui: &mut Ui, parent: Option<String>, at: Option<Vec3>) {
        let items: [(&str, MeshKind3D); 4] = [
            ("◇  Nodo vacío", MeshKind3D::Empty),
            ("■  Cubo", MeshKind3D::Cube),
            ("●  Esfera", MeshKind3D::Sphere { segments: 32 }),
            ("▬  Plano", MeshKind3D::Plane),
        ];
        for (label, kind) in items {
            if ui.button(label).clicked() {
                self.add_primitive(kind, parent.clone(), at);
                ui.close_menu();
            }
        }
        if ui.button("⛰  Terreno").clicked() {
            self.add_primitive(hills(), parent.clone(), at);
            ui.close_menu();
        }
        ui.separator();
        if ui.button("◈  Modelo 3D…").clicked() {
            self.dialog = Some(Dialog::AddModel {
                filter: String::new(),
                at,
                parent: parent.clone(),
            });
            ui.close_menu();
        }
        if parent.is_none() {
            ui.separator();
            if ui.button("💡  Luz puntual").clicked() {
                self.add_point_light(at);
                ui.close_menu();
            }
            if ui.button("☀  Luz direccional").clicked() {
                self.add_sun();
                ui.close_menu();
            }
        }
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::default()
                    .fill(theme::BG_DARK)
                    .inner_margin(egui::Margin::symmetric(10, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(l) = self.log.last() {
                        ui.label(RichText::new(l.level.icon()).color(l.level.color()));
                        ui.label(RichText::new(&l.text).color(theme::TEXT_DIM).size(12.0));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let small = |t: String| RichText::new(t).color(theme::TEXT_DIM).size(12.0);
                        if !self.gpu_name.is_empty() {
                            ui.label(small(self.gpu_name.clone()));
                            ui.separator();
                        }
                        if let Some(v) = &self.viewport {
                            ui.label(small(format!(
                                "{}×{} · CPU {:.1} ms · {} dibujos · {} triángulos",
                                v.size.0,
                                v.size.1,
                                v.frame_ms,
                                v.draws,
                                thousands(v.triangles)
                            )));
                            ui.separator();
                        }
                        ui.label(small(format!(
                            "{} nodos · {} luces",
                            ops::count(&self.scene.nodes),
                            self.scene.lights.len()
                        )));
                    });
                });
            });
    }

    // -------------------------------------------------------------- scene tree
    fn scene_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let parent = match &self.selection {
                Selection::Node(p) => Some(p.clone()),
                _ => None,
            };
            ui.menu_button(RichText::new(" + ").strong(), |ui| {
                if parent.is_some() {
                    ui.label(
                        RichText::new("Como hijo de la selección")
                            .size(11.5)
                            .color(theme::TEXT_DIM),
                    );
                }
                self.add_menu(ui, parent.clone(), None);
            })
            .response
            .on_hover_text("Añadir un nodo (hijo del seleccionado)");
            ui.add(
                TextEdit::singleline(&mut self.tree_filter)
                    .hint_text("🔍 Filtrar nodos")
                    .desired_width(f32::INFINITY),
            );
        });
        ui.add_space(3.0);
        let mut rows = Vec::new();
        let filter = self.tree_filter.trim().to_lowercase();
        walk_rows(
            &self.scene.nodes,
            None,
            0,
            &filter,
            &self.collapsed,
            &mut rows,
        );
        let mut acts: Vec<TreeAct> = Vec::new();
        ScrollArea::vertical()
            .id_salt("scene-tree")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let r = tree_row(
                    ui,
                    ui.make_persistent_id("tree-root"),
                    0,
                    None,
                    "🎬",
                    theme::ACCENT,
                    &self.scene.name,
                    self.selection == Selection::None,
                    false,
                    None,
                );
                if r.resp.clicked() {
                    acts.push(TreeAct::Select(Selection::None));
                }
                drop_target(ui, &r.resp, None, &mut acts);

                let r = tree_row(
                    ui,
                    ui.make_persistent_id("tree-camera"),
                    1,
                    None,
                    "🎥",
                    Color32::from_rgb(0xff, 0xc8, 0x6b),
                    "Cámara",
                    self.selection == Selection::Camera,
                    false,
                    None,
                );
                if r.resp.clicked() {
                    acts.push(TreeAct::Select(Selection::Camera));
                }
                if r.resp.double_clicked() {
                    acts.push(TreeAct::Focus(Selection::Camera));
                }
                for (i, l) in self.scene.lights.iter().enumerate() {
                    let (icon, name) = match l {
                        LightDoc::Directional { .. } => ("☀", "Luz direccional"),
                        LightDoc::Point { .. } => ("💡", "Luz puntual"),
                    };
                    let r = tree_row(
                        ui,
                        ui.make_persistent_id(("tree-light", i)),
                        1,
                        None,
                        icon,
                        Color32::from_rgb(0xff, 0xe0, 0x8a),
                        &format!("{name} {}", i + 1),
                        self.selection == Selection::Light(i),
                        false,
                        None,
                    );
                    if r.resp.clicked() {
                        acts.push(TreeAct::Select(Selection::Light(i)));
                    }
                    if r.resp.double_clicked() {
                        acts.push(TreeAct::Focus(Selection::Light(i)));
                    }
                    r.resp.context_menu(|ui| {
                        if ui
                            .button(RichText::new("Borrar la luz").color(theme::ERROR))
                            .clicked()
                        {
                            acts.push(TreeAct::DeleteLight(i));
                            ui.close_menu();
                        }
                    });
                }
                let r = tree_row(
                    ui,
                    ui.make_persistent_id("tree-env"),
                    1,
                    None,
                    "🌫",
                    Color32::from_rgb(0x9a, 0xc4, 0xff),
                    "Entorno (niebla)",
                    self.selection == Selection::Environment,
                    false,
                    None,
                );
                if r.resp.clicked() {
                    acts.push(TreeAct::Select(Selection::Environment));
                }
                ui.add_space(4.0);
                let line = ui.available_rect_before_wrap();
                ui.painter().hline(
                    line.left() + 8.0..=line.right() - 8.0,
                    line.top(),
                    Stroke::new(1.0_f32, theme::LINE),
                );
                ui.add_space(4.0);

                for row in &rows {
                    let id = ui.make_persistent_id(("node", &row.path));
                    let selected = matches!(&self.selection, Selection::Node(p) if *p == row.path);
                    let renaming = matches!(&self.renaming, Some((p, _, _)) if *p == row.path);
                    let r = tree_row(
                        ui,
                        id,
                        row.depth + 1,
                        row.has_children.then_some(row.open),
                        row.icon,
                        row.color,
                        if renaming { "" } else { &row.label },
                        selected,
                        row.hidden,
                        Some(!row.hidden),
                    );
                    self.row_rects.insert(row.path.clone(), r.resp.rect);
                    if r.toggled {
                        acts.push(TreeAct::Toggle(row.path.clone()));
                    }
                    if r.eye {
                        acts.push(TreeAct::Visible(row.path.clone()));
                    }
                    if r.resp.clicked() && !r.toggled && !r.eye {
                        acts.push(TreeAct::Select(Selection::Node(row.path.clone())));
                    }
                    if r.resp.double_clicked() {
                        acts.push(TreeAct::StartRename(row.path.clone()));
                    }
                    if r.resp.drag_started() {
                        egui::DragAndDrop::set_payload(ui.ctx(), DragNode(row.path.clone()));
                    }
                    drop_target(ui, &r.resp, Some(&row.path), &mut acts);
                    if renaming {
                        if let Some((_, buf, focused)) = self.renaming.as_mut() {
                            let te = ui.put(
                                r.label_rect.shrink2(egui::vec2(0.0, 1.0)),
                                TextEdit::singleline(buf).margin(egui::Margin::symmetric(4, 1)),
                            );
                            if !*focused {
                                te.request_focus();
                                *focused = true;
                            }
                            if te.lost_focus() {
                                if !ui.input(|i| i.key_pressed(Key::Escape)) {
                                    acts.push(TreeAct::Rename(row.path.clone(), buf.clone()));
                                }
                                acts.push(TreeAct::EndRename);
                            }
                        }
                    }
                    r.resp.context_menu(|ui| {
                        ui.menu_button("Añadir hijo", |ui| {
                            let items: [(&str, MeshKind3D); 4] = [
                                ("◇  Nodo vacío", MeshKind3D::Empty),
                                ("■  Cubo", MeshKind3D::Cube),
                                ("●  Esfera", MeshKind3D::Sphere { segments: 32 }),
                                ("▬  Plano", MeshKind3D::Plane),
                            ];
                            for (label, kind) in items {
                                if ui.button(label).clicked() {
                                    acts.push(TreeAct::AddChild(row.path.clone(), kind));
                                    ui.close_menu();
                                }
                            }
                            if ui.button("◈  Modelo 3D…").clicked() {
                                acts.push(TreeAct::AddModelDialog(Some(row.path.clone())));
                                ui.close_menu();
                            }
                        });
                        if ui
                            .add(egui::Button::new("Renombrar").shortcut_text("F2"))
                            .clicked()
                        {
                            acts.push(TreeAct::StartRename(row.path.clone()));
                            ui.close_menu();
                        }
                        if ui
                            .add(egui::Button::new("Duplicar").shortcut_text("Ctrl+D"))
                            .clicked()
                        {
                            acts.push(TreeAct::Duplicate(row.path.clone()));
                            ui.close_menu();
                        }
                        if ops::parent_of(&row.path).is_some()
                            && ui.button("Sacar a la raíz").clicked()
                        {
                            acts.push(TreeAct::ToRoot(row.path.clone()));
                            ui.close_menu();
                        }
                        if ui
                            .add(egui::Button::new("Enfocar").shortcut_text("F"))
                            .clicked()
                        {
                            acts.push(TreeAct::Focus(Selection::Node(row.path.clone())));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Borrar").color(theme::ERROR))
                                    .shortcut_text("Supr"),
                            )
                            .clicked()
                        {
                            acts.push(TreeAct::Delete(row.path.clone()));
                            ui.close_menu();
                        }
                    });
                }
                if rows.is_empty() && !filter.is_empty() {
                    ui.add_space(6.0);
                    dim(ui, "  Ningún nodo coincide.");
                }
                // The empty space below: deselect, and drop here to hang things from the root.
                let rest = ui.available_size_before_wrap();
                let (_, resp) =
                    ui.allocate_exact_size(egui::vec2(rest.x, rest.y.max(48.0)), Sense::click());
                if resp.clicked() {
                    acts.push(TreeAct::Select(Selection::None));
                }
                drop_target(ui, &resp, None, &mut acts);
            });
        for a in acts {
            self.apply_tree(a);
        }
    }

    fn apply_tree(&mut self, a: TreeAct) {
        match a {
            TreeAct::Select(s) => self.selection = s,
            TreeAct::Toggle(p) => {
                if !self.collapsed.remove(&p) {
                    self.collapsed.insert(p);
                }
            }
            TreeAct::Visible(p) => {
                if let Some(n) = ops::node_mut(&mut self.scene.nodes, &p) {
                    n.disabled = !n.disabled;
                }
            }
            TreeAct::StartRename(p) => {
                self.renaming = Some((p.clone(), ops::leaf(&p).to_string(), false));
            }
            TreeAct::Rename(p, name) => self.rename(&p, &name),
            TreeAct::EndRename => self.renaming = None,
            TreeAct::Duplicate(p) => {
                self.selection = Selection::Node(p);
                self.duplicate_selected();
            }
            TreeAct::Delete(p) => {
                self.selection = Selection::Node(p);
                self.delete_selected();
            }
            TreeAct::Focus(s) => {
                self.selection = s;
                self.focus_selected();
            }
            TreeAct::AddChild(p, kind) => self.add_primitive(kind, Some(p), None),
            TreeAct::AddModelDialog(parent) => {
                self.dialog = Some(Dialog::AddModel {
                    filter: String::new(),
                    at: None,
                    parent,
                })
            }
            TreeAct::ToRoot(p) => {
                if let Some(np) = ops::move_to_root(&mut self.scene, &p) {
                    self.selection = Selection::Node(np);
                }
            }
            TreeAct::Reparent(p, to) => self.reparent(&p, to.as_deref()),
            TreeAct::DropFile(f, parent) => self.drop_file(&f, parent, None),
            TreeAct::DeleteLight(i) => {
                self.selection = Selection::Light(i);
                self.delete_selected();
            }
        }
    }

    // -------------------------------------------------------------- file system
    fn files_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("⟳")
                .on_hover_text("Volver a leer las carpetas")
                .clicked()
            {
                self.files.refresh();
                self.models = None;
                self.scenes = None;
                self.file_search = None;
            }
            ui.add(
                TextEdit::singleline(&mut self.file_filter)
                    .hint_text("🔍 Buscar archivos")
                    .desired_width(f32::INFINITY),
            );
        });
        ui.add_space(3.0);
        let mut acts = Vec::new();
        let filter = self.file_filter.trim().to_lowercase();
        if filter.chars().count() >= 2 {
            let stale = self
                .file_search
                .as_ref()
                .map_or(true, |(f, _)| *f != filter);
            if stale {
                let found = files::search(&self.root, &filter, 300);
                self.file_search = Some((filter.clone(), found));
            }
        }
        ScrollArea::vertical()
            .id_salt("file-tree")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if filter.chars().count() >= 2 {
                    let found = self
                        .file_search
                        .as_ref()
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    if found.is_empty() {
                        dim(ui, "  Nada coincide.");
                    }
                    for p in found {
                        let rel = files::rel(&self.root, &p);
                        let is_dir = p.is_dir();
                        let name = p
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.file_row(ui, &p, &rel, &name, is_dir, 0, &mut acts);
                    }
                } else {
                    let name = self
                        .root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    tree_row(
                        ui,
                        ui.make_persistent_id("files-root"),
                        0,
                        None,
                        "🗀",
                        file_color("", true),
                        &format!("res://  ({name})"),
                        false,
                        false,
                        None,
                    );
                    let root = self.root.clone();
                    self.dir_rows(ui, &root, 1, &mut acts);
                }
            });
        for a in acts {
            match a {
                FileAct::Toggle(p) => {
                    if !self.expanded.remove(&p) {
                        self.expanded.insert(p);
                    }
                }
                FileAct::Select(p) => self.selected_file = Some(p),
                FileAct::Open(p) => self.open_file(&p),
                FileAct::OpenText(p) => self.open_script(&p),
                FileAct::AddModel(p) => self.drop_file(&p, None, None),
                FileAct::CopyPath(p) => {
                    let rel = files::rel(&self.root, &p);
                    ui.ctx().copy_text(rel.clone());
                    self.log(Level::Info, format!("Ruta copiada: {rel}"));
                }
                FileAct::Reveal(p) => {
                    let _ = std::process::Command::new("explorer")
                        .arg(format!("/select,{}", p.display()))
                        .spawn();
                }
            }
        }
    }

    fn dir_rows(&mut self, ui: &mut Ui, dir: &Path, depth: usize, acts: &mut Vec<FileAct>) {
        for e in self.files.list(dir) {
            let rel = e.name.clone();
            self.file_row(ui, &e.path, &rel, &e.name, e.is_dir, depth, acts);
            if e.is_dir && self.expanded.contains(&e.path) {
                self.dir_rows(ui, &e.path, depth + 1, acts);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn file_row(
        &mut self,
        ui: &mut Ui,
        path: &Path,
        label: &str,
        name: &str,
        is_dir: bool,
        depth: usize,
        acts: &mut Vec<FileAct>,
    ) {
        let open = self.expanded.contains(path);
        let id = ui.make_persistent_id(("file", path));
        let selected = self.selected_file.as_deref() == Some(path);
        let r = tree_row(
            ui,
            id,
            depth,
            is_dir.then_some(open),
            files::icon(name, is_dir),
            file_color(name, is_dir),
            label,
            selected,
            false,
            None,
        );
        if r.toggled || (is_dir && r.resp.clicked()) {
            acts.push(FileAct::Toggle(path.to_path_buf()));
        } else if r.resp.clicked() {
            acts.push(FileAct::Select(path.to_path_buf()));
        }
        if !is_dir && r.resp.double_clicked() {
            acts.push(FileAct::Open(path.to_path_buf()));
        }
        if !is_dir && r.resp.drag_started() {
            egui::DragAndDrop::set_payload(ui.ctx(), DragFile(path.to_path_buf()));
        }
        let lower = name.to_ascii_lowercase();
        r.resp.context_menu(|ui| {
            if !is_dir && ui.button("Abrir").clicked() {
                acts.push(FileAct::Open(path.to_path_buf()));
                ui.close_menu();
            }
            if (lower.ends_with(".glb") || lower.ends_with(".gltf"))
                && ui.button("Añadir a la escena").clicked()
            {
                acts.push(FileAct::AddModel(path.to_path_buf()));
                ui.close_menu();
            }
            if lower.ends_with(".json") && ui.button("Abrir como texto").clicked() {
                acts.push(FileAct::OpenText(path.to_path_buf()));
                ui.close_menu();
            }
            if ui.button("Copiar la ruta").clicked() {
                acts.push(FileAct::CopyPath(path.to_path_buf()));
                ui.close_menu();
            }
            if ui.button("Mostrar en el Explorador").clicked() {
                acts.push(FileAct::Reveal(path.to_path_buf()));
                ui.close_menu();
            }
        });
    }

    // -------------------------------------------------------------- output
    fn output_tab(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            for (lvl, name) in [
                (Level::Info, "Mensajes"),
                (Level::Ok, "Hecho"),
                (Level::Warn, "Avisos"),
                (Level::Error, "Errores"),
            ] {
                let count = self.log.iter().filter(|l| l.level == lvl).count();
                ui.toggle_value(
                    &mut self.log_filter[lvl.index()],
                    RichText::new(format!("{} {name} {count}", lvl.icon()))
                        .color(lvl.color())
                        .size(12.0),
                );
            }
            ui.separator();
            if ui.button("Limpiar").clicked() {
                self.log.clear();
            }
        });
        ui.add_space(2.0);
        ScrollArea::vertical()
            .id_salt("output")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for l in &self.log {
                    if !self.log_filter[l.level.index()] {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:>6.1} s", l.at))
                                .monospace()
                                .size(11.5)
                                .color(theme::TEXT_DIM),
                        );
                        ui.label(RichText::new(l.level.icon()).color(l.level.color()));
                        let col = if l.level == Level::Info {
                            theme::TEXT
                        } else {
                            l.level.color()
                        };
                        ui.add(
                            egui::Label::new(RichText::new(&l.text).color(col).size(12.5)).wrap(),
                        );
                    });
                }
            });
    }

    // -------------------------------------------------------------- scripts
    fn script_tab(&mut self, ui: &mut Ui, path: &Path) {
        let Some(i) = self.scripts.iter().position(|s| s.path == path) else {
            dim(ui, "Este archivo ya no está abierto.");
            return;
        };
        let rel = files::rel(&self.root, path);
        let mut save = false;
        let mut reload = false;
        {
            let s = &self.scripts[i];
            ui.horizontal(|ui| {
                ui.label(RichText::new(files::icon(&rel, false)).color(file_color(&rel, false)));
                ui.label(
                    RichText::new(&rel)
                        .monospace()
                        .size(12.0)
                        .color(theme::TEXT),
                );
                if s.dirty() {
                    ui.label(RichText::new("● sin guardar").color(theme::WARN).size(12.0));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    save = ui
                        .add_enabled(s.dirty(), egui::Button::new("Guardar"))
                        .on_hover_text("Ctrl+S")
                        .clicked();
                    reload = ui
                        .button("Recargar")
                        .on_hover_text("Volver a leerlo del disco")
                        .clicked();
                    ui.label(
                        RichText::new(format!(
                            "{} · Línea {}, col. {}",
                            s.lang.name(),
                            s.cursor.0,
                            s.cursor.1
                        ))
                        .color(theme::TEXT_DIM)
                        .size(12.0),
                    );
                });
            });
        }
        ui.add_space(3.0);
        let font = FontId::monospace(13.0);
        let lang = self.scripts[i].lang;
        let mut layouter = |ui: &Ui, text: &str, _wrap: f32| {
            let mut job = code::highlight(text, lang, &font);
            job.wrap.max_width = f32::INFINITY;
            ui.fonts(|f| f.layout_job(job))
        };
        let area = ui.available_rect_before_wrap();
        self.script_rect = Some(area);
        ui.painter().rect_filled(area, 4.0, theme::BG_DARK);
        let mut cursor = None;
        let script = &mut self.scripts[i];
        ScrollArea::both()
            .id_salt(("script-scroll", path))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (gutter, _) = ui.allocate_exact_size(egui::vec2(50.0, 1.0), Sense::hover());
                    let out = TextEdit::multiline(&mut script.text)
                        .id_salt(("script", path))
                        .code_editor()
                        .frame(false)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .desired_rows(40)
                        .margin(egui::Margin::symmetric(8, 6))
                        .layouter(&mut layouter)
                        .show(ui);
                    let painter = ui.painter();
                    let clip = painter.clip_rect();
                    let numbers = FontId::monospace(12.0);
                    let mut line = 1;
                    let mut starts = true;
                    for row in &out.galley.rows {
                        let y = out.galley_pos.y + row.rect.min.y;
                        if starts && y + row.rect.height() >= clip.top() && y <= clip.bottom() {
                            painter.text(
                                egui::pos2(gutter.right() - 6.0, y + 1.0),
                                Align2::RIGHT_TOP,
                                line.to_string(),
                                numbers.clone(),
                                theme::TEXT_DIM.gamma_multiply(0.75),
                            );
                        }
                        if starts {
                            line += 1;
                        }
                        starts = row.ends_with_newline;
                    }
                    if let Some(range) = out.cursor_range {
                        cursor = Some(range.primary.ccursor.index);
                    }
                });
            });
        if let Some(ci) = cursor {
            let (mut line, mut col) = (1, 1);
            for (k, ch) in script.text.chars().enumerate() {
                if k == ci {
                    break;
                }
                if ch == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
            }
            script.cursor = (line, col);
        }
        if save {
            self.save_script(path);
        }
        if reload {
            if let Ok(t) = std::fs::read_to_string(path) {
                self.scripts[i].saved = t.clone();
                self.scripts[i].text = t;
                self.log(Level::Info, format!("Recargado: {rel}"));
            }
        }
    }

    // -------------------------------------------------------------- inspector
    fn inspector_tab(&mut self, ui: &mut Ui) {
        ScrollArea::vertical()
            .id_salt("inspector")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(2.0);
                match self.selection.clone() {
                    Selection::Node(p) => self.inspect_node(ui, &p),
                    Selection::Light(i) => self.inspect_light(ui, i),
                    Selection::Camera => self.inspect_camera(ui),
                    Selection::Environment => self.inspect_environment(ui),
                    Selection::None => self.inspect_scene(ui),
                }
            });
    }

    fn inspect_node(&mut self, ui: &mut Ui, path: &str) {
        if self.models.is_none() {
            self.models = Some(files::find(&self.root, &[".glb", ".gltf"], 5000));
        }
        let world = self.world_of(path);
        let bounds = self
            .viewport
            .as_ref()
            .and_then(|v| v.bounds.get(path).copied());
        let root = self.root.clone();
        let mut rename_to: Option<String> = None;
        let mut add_child: Option<MeshKind3D> = None;
        let clip_info = ops::node(&self.scene.nodes, path)
            .and_then(|n| match &n.mesh {
                MeshKind3D::Gltf { path: m } => Some(m.clone()),
                _ => None,
            })
            .and_then(|m| {
                let clips = self.viewport.as_ref()?.clips(&m)?;
                Some((m, clips))
            });
        let mut clip_choice: Option<Option<usize>> = None;
        let models: &[String] = self.models.as_deref().unwrap_or(&[]);
        let Some(node) = ops::node_mut(&mut self.scene.nodes, path) else {
            dim(ui, "Ese nodo ya no existe.");
            return;
        };
        let (icon, kind) = ops::kind_label(&node.mesh);
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(18.0).color(kind_color(&node.mesh)));
            let edit = self
                .name_edit
                .get_or_insert_with(|| (path.to_string(), node.id.clone()));
            if edit.0 != path {
                *edit = (path.to_string(), node.id.clone());
            }
            let r = ui.add(
                TextEdit::singleline(&mut edit.1)
                    .font(FontId::proportional(15.0))
                    .desired_width((ui.available_width() - 76.0).max(80.0)),
            );
            if r.lost_focus() && edit.1.trim() != node.id {
                rename_to = Some(edit.1.clone());
            }
            ui.label(RichText::new(kind).color(theme::TEXT_DIM));
        });
        ui.label(
            RichText::new(path)
                .monospace()
                .size(11.0)
                .color(theme::TEXT_DIM),
        );
        ui.add_space(6.0);

        section(ui, "Transformación", |ui| {
            grid(ui, "tf", |ui| {
                dim(ui, "Posición");
                vec3_edit(ui, &mut node.transform.position, 0.02, "", 3);
                ui.end_row();
                dim(ui, "Rotación");
                vec3_edit(ui, &mut node.transform.rotation_deg, 0.5, "°", 1);
                ui.end_row();
                dim(ui, "Escala");
                vec3_edit(ui, &mut node.transform.scale, 0.01, "", 3);
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.small_button("Restablecer").clicked() {
                    node.transform = TransformDoc::default();
                }
                if ui
                    .small_button("Escala uniforme")
                    .on_hover_text("Copia X en Y y Z")
                    .clicked()
                {
                    let x = node.transform.scale[0];
                    node.transform.scale = [x, x, x];
                }
            });
        });

        section(ui, "Malla", |ui| {
            grid(ui, "mesh", |ui| {
                dim(ui, "Tipo");
                let current = kind_index(&node.mesh);
                egui::ComboBox::from_id_salt("mesh-kind")
                    .selected_text(KIND_NAMES[current])
                    .show_ui(ui, |ui| {
                        for (k, name) in KIND_NAMES.iter().enumerate() {
                            if ui.selectable_label(k == current, *name).clicked() && k != current {
                                node.mesh = mesh_of_kind(k);
                            }
                        }
                    });
                ui.end_row();
                match &mut node.mesh {
                    MeshKind3D::Sphere { segments } => {
                        dim(ui, "Segmentos");
                        ui.add(DragValue::new(segments).range(3..=64));
                        ui.end_row();
                    }
                    MeshKind3D::Heightfield {
                        resolution,
                        size,
                        heights,
                    } => {
                        dim(ui, "Muestras");
                        ui.label(format!(
                            "{r} × {r} ({} alturas)",
                            heights.len(),
                            r = resolution
                        ));
                        ui.end_row();
                        dim(ui, "Tamaño");
                        ui.add(
                            DragValue::new(size)
                                .speed(0.1)
                                .range(0.5..=4000.0)
                                .suffix(" m"),
                        );
                        ui.end_row();
                    }
                    MeshKind3D::Gltf { path: model } => {
                        dim(ui, "Archivo");
                        ui.add(
                            TextEdit::singleline(model)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                        ui.end_row();
                        dim(ui, "");
                        ui.horizontal(|ui| {
                            if root.join(model.as_str()).is_file() {
                                ui.label(RichText::new("✔ encontrado").color(theme::OK).size(12.0));
                            } else {
                                ui.label(
                                    RichText::new("✖ no existe").color(theme::ERROR).size(12.0),
                                );
                            }
                            egui::ComboBox::from_id_salt("model-pick")
                                .selected_text("Cambiar…")
                                .width(110.0)
                                .height(360.0)
                                .show_ui(ui, |ui| {
                                    ui.set_min_width(360.0);
                                    for m in models {
                                        if ui
                                            .selectable_label(
                                                m.as_str() == model.as_str(),
                                                m.as_str(),
                                            )
                                            .clicked()
                                        {
                                            *model = m.clone();
                                        }
                                    }
                                });
                        });
                        ui.end_row();
                    }
                    _ => {}
                }
            });
        });

        if let Some((_, (names, current))) = &clip_info {
            section(ui, "Animación", |ui| {
                grid(ui, "anim", |ui| {
                    dim(ui, "Clip");
                    let shown = current
                        .and_then(|c| names.get(c))
                        .map_or("(postura de enlace)".to_string(), |n| n.clone());
                    egui::ComboBox::from_id_salt("clip-pick")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(current.is_none(), "(postura de enlace)")
                                .clicked()
                            {
                                clip_choice = Some(None);
                            }
                            for (k, n) in names.iter().enumerate() {
                                if ui
                                    .selectable_label(*current == Some(k), n.as_str())
                                    .clicked()
                                {
                                    clip_choice = Some(Some(k));
                                }
                            }
                        });
                    ui.end_row();
                    dim(ui, "Vista previa");
                    if let Some(v) = self.viewport.as_mut() {
                        let text = if v.playing {
                            "⏸ Pausar"
                        } else {
                            "▶ Reproducir"
                        };
                        ui.toggle_value(&mut v.playing, text);
                    }
                    ui.end_row();
                });
                ui.label(
                    RichText::new("Solo para mirarla: la escena no guarda la animación.")
                        .size(11.5)
                        .color(theme::TEXT_DIM),
                );
            });
        }

        section(ui, "Material", |ui| {
            grid(ui, "mat", |ui| {
                dim(ui, "Color");
                // Only on a real change: the picker's round trip nudges the value (0.75 → 0.75000006)
                // and would put a step in the history every time a node is shown.
                let mut color = node.material.base_color;
                if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                    node.material.base_color = color;
                }
                ui.end_row();
                dim(ui, "Oclusión");
                ui.add(egui::Slider::new(
                    &mut node.material.ambient_occlusion,
                    0.0..=1.0,
                ));
                ui.end_row();
            });
            if matches!(node.mesh, MeshKind3D::Gltf { .. }) {
                ui.label(
                    RichText::new(
                        "En los modelos, el color tiñe sus texturas (blanco = original).",
                    )
                    .size(11.5)
                    .color(theme::TEXT_DIM),
                );
            }
        });

        section(ui, "Render y física", |ui| {
            let mut visible = !node.disabled;
            if ui.checkbox(&mut visible, "Visible").changed() {
                node.disabled = !visible;
            }
            ui.checkbox(&mut node.cast_shadow, "Proyecta sombra");
            ui.checkbox(&mut node.collider, "Colisionador (para el juego)");
        });

        section(ui, "Información", |ui| {
            grid(ui, "info", |ui| {
                dim(ui, "Posición global");
                ui.label(format!(
                    "{:.2}, {:.2}, {:.2}",
                    world.w_axis[0], world.w_axis[1], world.w_axis[2]
                ));
                ui.end_row();
                if let Some(b) = bounds {
                    let s = b.max - b.min;
                    dim(ui, "Tamaño");
                    ui.label(format!("{:.2} × {:.2} × {:.2} m", s.x, s.y, s.z));
                    ui.end_row();
                }
                dim(ui, "Hijos");
                ui.label(node.children.len().to_string());
                ui.end_row();
            });
            ui.menu_button("+ Añadir hijo", |ui| {
                let items: [(&str, MeshKind3D); 4] = [
                    ("◇  Nodo vacío", MeshKind3D::Empty),
                    ("■  Cubo", MeshKind3D::Cube),
                    ("●  Esfera", MeshKind3D::Sphere { segments: 32 }),
                    ("▬  Plano", MeshKind3D::Plane),
                ];
                for (label, kind) in items {
                    if ui.button(label).clicked() {
                        add_child = Some(kind);
                        ui.close_menu();
                    }
                }
            });
        });

        if let (Some(choice), Some((model, _))) = (clip_choice, &clip_info) {
            if let Some(v) = self.viewport.as_mut() {
                v.set_clip(model, choice);
            }
        }
        if let Some(name) = rename_to {
            self.rename(path, &name);
        }
        if let Some(kind) = add_child {
            self.add_primitive(kind, Some(path.to_string()), None);
        }
    }

    fn inspect_light(&mut self, ui: &mut Ui, i: usize) {
        let mut delete = false;
        let Some(light) = self.scene.lights.get_mut(i) else {
            dim(ui, "Esa luz ya no existe.");
            return;
        };
        match light {
            LightDoc::Directional {
                direction,
                key_color,
                ambient_color,
                shadow_extent,
            } => {
                ui.label(
                    RichText::new(format!("☀  Luz direccional {}", i + 1))
                        .family(theme::bold())
                        .size(15.0),
                );
                ui.add_space(6.0);
                section(ui, "Sol", |ui| {
                    let v = v3(*direction).normalize_or_zero();
                    let mut az = v.x.atan2(v.z).to_degrees();
                    let mut el = v.y.clamp(-1.0, 1.0).asin().to_degrees();
                    grid(ui, "sun", |ui| {
                        dim(ui, "Azimut");
                        let a = ui.add(egui::Slider::new(&mut az, -180.0..=180.0).suffix("°"));
                        ui.end_row();
                        dim(ui, "Elevación");
                        let b = ui.add(egui::Slider::new(&mut el, -10.0..=90.0).suffix("°"));
                        ui.end_row();
                        if a.changed() || b.changed() {
                            let (a, e) = (az.to_radians(), el.to_radians());
                            *direction = [e.cos() * a.sin(), e.sin(), e.cos() * a.cos()];
                        }
                        dim(ui, "Dirección");
                        vec3_edit(ui, direction, 0.01, "", 3);
                        ui.end_row();
                        dim(ui, "Color");
                        rgb_edit(ui, key_color);
                        ui.end_row();
                        dim(ui, "Ambiente");
                        rgb_edit(ui, ambient_color);
                        ui.end_row();
                        dim(ui, "Alcance sombras");
                        ui.add(
                            DragValue::new(shadow_extent)
                                .speed(0.2)
                                .range(1.0..=400.0)
                                .suffix(" m"),
                        );
                        ui.end_row();
                    });
                });
            }
            LightDoc::Point {
                position,
                color,
                intensity,
                radius,
            } => {
                ui.label(
                    RichText::new(format!("💡  Luz puntual {}", i + 1))
                        .family(theme::bold())
                        .size(15.0),
                );
                ui.add_space(6.0);
                section(ui, "Luz", |ui| {
                    grid(ui, "point", |ui| {
                        dim(ui, "Posición");
                        vec3_edit(ui, position, 0.02, "", 2);
                        ui.end_row();
                        dim(ui, "Color");
                        rgb_edit(ui, color);
                        ui.end_row();
                        dim(ui, "Intensidad");
                        ui.add(DragValue::new(intensity).speed(0.05).range(0.0..=100.0));
                        ui.end_row();
                        dim(ui, "Radio");
                        ui.add(
                            DragValue::new(radius)
                                .speed(0.05)
                                .range(0.1..=200.0)
                                .suffix(" m"),
                        );
                        ui.end_row();
                    });
                });
            }
        }
        ui.add_space(6.0);
        if ui
            .button(RichText::new("Borrar la luz").color(theme::ERROR))
            .clicked()
        {
            delete = true;
        }
        if delete {
            self.selection = Selection::Light(i);
            self.delete_selected();
        }
    }

    fn inspect_camera(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new("🎥  Cámara de la escena")
                .family(theme::bold())
                .size(15.0),
        );
        dim(ui, "Desde donde arranca el juego al cargar esta escena.");
        ui.add_space(6.0);
        let c = &mut self.scene.camera;
        section(ui, "Vista", |ui| {
            grid(ui, "cam", |ui| {
                dim(ui, "Posición");
                vec3_edit(ui, &mut c.position, 0.02, "", 2);
                ui.end_row();
                dim(ui, "Objetivo");
                vec3_edit(ui, &mut c.target, 0.02, "", 2);
                ui.end_row();
                dim(ui, "Campo visual");
                ui.add(egui::Slider::new(&mut c.fov_deg, 10.0..=120.0).suffix("°"));
                ui.end_row();
                dim(ui, "Ortográfica");
                let mut ortho = c.orthographic_height.is_some();
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut ortho, "").changed() {
                        c.orthographic_height = if ortho { Some(10.0) } else { None };
                    }
                    if let Some(h) = c.orthographic_height.as_mut() {
                        ui.add(
                            DragValue::new(h)
                                .speed(0.1)
                                .range(0.5..=500.0)
                                .prefix("alto "),
                        );
                    }
                });
                ui.end_row();
                dim(ui, "Cerca / lejos");
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut c.near).speed(0.01).range(0.001..=10.0));
                    ui.add(DragValue::new(&mut c.far).speed(1.0).range(1.0..=10000.0));
                });
                ui.end_row();
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.use_scene_camera, "👁 Ver por esta cámara");
            if ui.button("Copiar la vista del editor").clicked() {
                let cam = self.cam.camera();
                c.position = cam.position.to_array();
                c.target = cam.target.to_array();
                c.fov_deg = self.cam.fov_deg;
            }
        });
    }

    fn inspect_environment(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("🌫  Entorno").family(theme::bold()).size(15.0));
        ui.add_space(6.0);
        let env = &mut self.scene.environment;
        section(ui, "Niebla", |ui| {
            grid(ui, "fog", |ui| {
                dim(ui, "Color");
                rgb_edit(ui, &mut env.fog_color);
                ui.end_row();
                dim(ui, "Empieza");
                ui.add(
                    DragValue::new(&mut env.fog_start)
                        .speed(0.2)
                        .range(0.0..=5000.0)
                        .suffix(" m"),
                );
                ui.end_row();
                dim(ui, "Termina");
                ui.add(
                    DragValue::new(&mut env.fog_end)
                        .speed(0.2)
                        .range(0.0..=5000.0)
                        .suffix(" m"),
                );
                ui.end_row();
                dim(ui, "Densidad");
                ui.add(egui::Slider::new(&mut env.fog_density, 0.0..=1.0));
                ui.end_row();
            });
        });
    }

    fn inspect_scene(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("🎬  Escena").family(theme::bold()).size(15.0));
        ui.add_space(6.0);
        let file = self
            .scene_path
            .as_ref()
            .map_or("(sin guardar)".to_string(), |p| files::rel(&self.root, p));
        let models = {
            let mut paths = Vec::new();
            ops::all_paths(&self.scene.nodes, None, &mut paths);
            paths
                .iter()
                .filter(|p| {
                    matches!(
                        ops::node(&self.scene.nodes, p).map(|n| &n.mesh),
                        Some(MeshKind3D::Gltf { .. })
                    )
                })
                .count()
        };
        let (nodes, lights) = (ops::count(&self.scene.nodes), self.scene.lights.len());
        section(ui, "Documento", |ui| {
            grid(ui, "doc", |ui| {
                dim(ui, "Nombre");
                ui.add(TextEdit::singleline(&mut self.scene.name).desired_width(f32::INFINITY));
                ui.end_row();
                dim(ui, "Archivo");
                ui.label(RichText::new(&file).monospace().size(12.0));
                ui.end_row();
                dim(ui, "Contenido");
                ui.label(format!("{nodes} nodos · {models} modelos · {lights} luces"));
                ui.end_row();
                dim(ui, "Proyecto");
                ui.label(RichText::new(self.root.display().to_string()).size(12.0));
                ui.end_row();
            });
        });
        let mut check = false;
        let mut save = false;
        ui.horizontal(|ui| {
            check = ui.button("Comprobar recursos").clicked();
            save = ui.button("Guardar").clicked();
        });
        ui.add_space(10.0);
        section(ui, "Cómo se usa", |ui| {
            for line in [
                "Clic: seleccionar · W/E/R: mover, rotar, escalar",
                "Clic derecho y arrastrar: girar la vista · rueda: acercar",
                "Clic central: desplazar · clic derecho + WASD: volar",
                "F: enfocar la selección · Supr: borrar · Ctrl+D: duplicar",
                "Arrastra un .glb del Sistema de archivos al visor para añadirlo",
            ] {
                ui.label(RichText::new(line).size(12.0).color(theme::TEXT_DIM));
            }
        });
        if check {
            self.check_assets();
        }
        if save {
            self.save();
        }
    }

    // -------------------------------------------------------------- viewport
    fn effective_tool(&self) -> Tool {
        match self.selection {
            Selection::Node(_) => self.tool,
            Selection::Light(_) | Selection::Camera if self.tool != Tool::Select => Tool::Move,
            _ => self.tool,
        }
    }

    fn gizmo_frame(&self, view: &View, tool: Tool) -> Option<Frame> {
        if tool == Tool::Select {
            return None;
        }
        let xyz = [Vec3::X, Vec3::Y, Vec3::Z];
        let (center, axes) = match &self.selection {
            Selection::Node(p) => {
                let node = ops::node(&self.scene.nodes, p)?;
                if node.disabled {
                    return None;
                }
                let world = self.world_of(p);
                let center = Vec3::new(world.w_axis[0], world.w_axis[1], world.w_axis[2]);
                let axes = match tool {
                    Tool::Rotate => {
                        gizmo::euler_axes(&self.parent_world(p), node.transform.rotation_deg)
                    }
                    Tool::Scale => gizmo::local_axes(&world),
                    _ => xyz,
                };
                (center, axes)
            }
            Selection::Light(i) => match self.scene.lights.get(*i)? {
                LightDoc::Point { position, .. } => (v3(*position), xyz),
                _ => return None,
            },
            Selection::Camera if !self.use_scene_camera => (v3(self.scene.camera.position), xyz),
            _ => return None,
        };
        Some(Frame {
            center,
            axes,
            arm: view.arm(center),
        })
    }

    fn begin_gizmo(&mut self, view: &View, frame: Frame, handle: Handle, from: Pos2) {
        let (start, parent_inv) = match &self.selection {
            Selection::Node(p) => (
                ops::node(&self.scene.nodes, p).map(|n| n.transform.clone()),
                self.parent_world(p).inverse().unwrap_or(Mat4::IDENTITY),
            ),
            _ => (None, Mat4::IDENTITY),
        };
        let hit0 = match handle {
            Handle::Plane(i) => gizmo::plane_hit(view, from, frame.center, frame.axes[i]),
            _ => None,
        };
        self.gdrag = Some(GizmoDrag {
            handle,
            from,
            frame,
            start,
            start_pos: frame.center,
            start_target: v3(self.scene.camera.target),
            parent_inv,
            hit0,
            last_angle: gizmo::screen_angle(view, frame.center, from).unwrap_or(0.0),
            turned: 0.0,
        });
    }

    fn update_gizmo(&mut self, view: &View, to: Pos2) {
        let (snap, sm, sr, ss) = (self.snap, self.snap_move, self.snap_rot, self.snap_scale);
        let Some(d) = self.gdrag.as_mut() else {
            return;
        };
        let f = d.frame;
        let mut pos = None;
        let mut rot = None;
        let mut scale = None;
        match d.handle {
            Handle::Axis(i) => {
                if let Some(w) = gizmo::axis_drag(view, d.start_pos, f.axes[i], f.arm, d.from, to) {
                    let w = if snap { gizmo::snap(w, sm) } else { w };
                    pos = Some(d.start_pos + f.axes[i] * w);
                }
            }
            Handle::Plane(i) => {
                if let (Some(h0), Some(h)) =
                    (d.hit0, gizmo::plane_hit(view, to, d.start_pos, f.axes[i]))
                {
                    let mut m = h - h0;
                    if snap {
                        m = Vec3::new(
                            gizmo::snap(m.x, sm),
                            gizmo::snap(m.y, sm),
                            gizmo::snap(m.z, sm),
                        );
                    }
                    pos = Some(d.start_pos + m);
                }
            }
            Handle::Ring(i) => {
                if let Some(a) = gizmo::screen_angle(view, d.start_pos, to) {
                    d.turned += gizmo::wrap_angle(a - d.last_angle);
                    d.last_angle = a;
                    let sign = if view.facing(d.start_pos, f.axes[i]) {
                        1.0
                    } else {
                        -1.0
                    };
                    let mut deg = d.turned.to_degrees() * sign;
                    if snap {
                        deg = gizmo::snap(deg, sr);
                    }
                    rot = Some((i, deg));
                }
            }
            Handle::ScaleAxis(i) => {
                if let (Some(w), Some(t)) = (
                    gizmo::axis_drag(view, d.start_pos, f.axes[i], f.arm, d.from, to),
                    d.start.as_ref(),
                ) {
                    let mut k = 1.0 + w / f.arm;
                    if snap {
                        k = 1.0 + gizmo::snap(k - 1.0, ss);
                    }
                    let mut s = t.scale;
                    s[i] = (t.scale[i] * k).max(0.001);
                    scale = Some(s);
                }
            }
            Handle::ScaleAll => {
                if let Some(t) = d.start.as_ref() {
                    let dd = (to.x - d.from.x) - (to.y - d.from.y);
                    let mut k = (1.0 + dd / 150.0).max(0.01);
                    if snap {
                        k = (1.0 + gizmo::snap(k - 1.0, ss)).max(0.01);
                    }
                    scale = Some(t.scale.map(|v| (v * k).max(0.001)));
                }
            }
        }
        let start = d.start.clone();
        let (parent_inv, start_pos, start_target) = (d.parent_inv, d.start_pos, d.start_target);
        match self.selection.clone() {
            Selection::Node(p) => {
                if let Some(n) = ops::node_mut(&mut self.scene.nodes, &p) {
                    if let Some(w) = pos {
                        n.transform.position = parent_inv.transform_point3(w).to_array();
                    }
                    if let (Some((i, deg)), Some(t)) = (rot, start.as_ref()) {
                        n.transform.rotation_deg = t.rotation_deg;
                        n.transform.rotation_deg[i] = t.rotation_deg[i] + deg;
                    }
                    if let Some(s) = scale {
                        n.transform.scale = s;
                    }
                }
            }
            Selection::Light(i) => {
                if let (Some(w), Some(LightDoc::Point { position, .. })) =
                    (pos, self.scene.lights.get_mut(i))
                {
                    *position = w.to_array();
                }
            }
            Selection::Camera => {
                if let Some(w) = pos {
                    let delta = w - start_pos;
                    self.scene.camera.position = w.to_array();
                    self.scene.camera.target = (start_target + delta).to_array();
                }
            }
            _ => {}
        }
    }

    fn look_along(&mut self, axis: usize, positive: bool) {
        self.use_scene_camera = false;
        let (yaw, pitch) = match (axis, positive) {
            (0, true) => (FRAC_PI_2, 0.0),
            (0, false) => (-FRAC_PI_2, 0.0),
            (1, true) => (self.cam.yaw, 1.52),
            (1, false) => (self.cam.yaw, -1.52),
            (2, true) => (0.0, 0.0),
            _ => (PI, 0.0),
        };
        self.cam.yaw = yaw;
        self.cam.pitch = pitch;
    }

    fn navigate(&mut self, ui: &Ui, resp: &egui::Response, rect: Rect, alt: bool, shift: bool) {
        let d = resp.drag_delta();
        let orbiting = (resp.dragged_by(PointerButton::Secondary) && !shift)
            || (resp.dragged_by(PointerButton::Middle) && shift)
            || (resp.dragged_by(PointerButton::Primary) && alt && self.gdrag.is_none());
        let panning = (resp.dragged_by(PointerButton::Middle) && !shift)
            || (resp.dragged_by(PointerButton::Secondary) && shift);
        let scroll = if resp.hovered() {
            ui.input(|i| i.smooth_scroll_delta.y)
        } else {
            0.0
        };
        if (orbiting || panning || scroll != 0.0) && self.use_scene_camera {
            // Moving the view leaves the scene camera, starting from where it looks.
            let c = &self.scene.camera;
            self.cam =
                OrbitCamera::looking(v3(c.position), v3(c.target), c.fov_deg.clamp(25.0, 80.0));
            self.use_scene_camera = false;
        }
        if orbiting {
            self.cam.orbit(d.x, d.y);
        }
        if panning {
            self.cam.pan(d.x, d.y, rect.height());
        }
        if scroll != 0.0 {
            self.cam.zoom(scroll);
        }
        let right_held =
            resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.secondary_down());
        self.flying = right_held;
        if right_held {
            let (dt, k) = ui.input(|i| {
                (
                    i.stable_dt.min(0.05),
                    [
                        i.key_down(Key::W),
                        i.key_down(Key::S),
                        i.key_down(Key::A),
                        i.key_down(Key::D),
                        i.key_down(Key::Q),
                        i.key_down(Key::E),
                        i.modifiers.shift,
                    ],
                )
            });
            let axis = |a: bool, b: bool| a as i32 as f32 - b as i32 as f32;
            let (f, r, u) = (axis(k[0], k[1]), axis(k[3], k[2]), axis(k[5], k[4]));
            if f != 0.0 || r != 0.0 || u != 0.0 {
                let speed = self.cam.distance.clamp(2.0, 60.0) * if k[6] { 2.5 } else { 0.9 } * dt;
                self.cam.fly(f * speed, r * speed, u * speed);
                self.repaint = true;
            }
        }
    }

    fn pick(&mut self, view: &View, p: Pos2) {
        if let Some((_, sel)) = self
            .icon_hits
            .iter()
            .find(|(q, _)| (*q - p).length() < 12.0)
        {
            self.selection = sel.clone();
            return;
        }
        let (o, d) = view.ray(p);
        self.selection = match self.viewport.as_ref().and_then(|v| v.pick(o, d)) {
            Some((id, _)) => Selection::Node(id),
            None => Selection::None,
        };
    }

    fn viewport_toolbar(&mut self, ui: &mut Ui) {
        let compact = ui.available_width() < 760.0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            for (t, icon) in [
                (Tool::Select, "↖"),
                (Tool::Move, "✥"),
                (Tool::Rotate, "⟳"),
                (Tool::Scale, "⤡"),
            ] {
                let on = self.tool == t;
                let label = if compact {
                    icon.to_string()
                } else {
                    format!("{icon} {}", t.label())
                };
                let text = RichText::new(label).size(12.5).color(if on {
                    Color32::WHITE
                } else {
                    theme::TEXT
                });
                let r = ui.add(
                    egui::Button::new(text)
                        .fill(if on {
                            theme::ACCENT_DIM
                        } else {
                            theme::BG_WIDGET
                        })
                        .min_size(egui::vec2(0.0, 22.0)),
                );
                if r.on_hover_text(format!("{} ({})", t.label(), t.key()))
                    .clicked()
                {
                    self.tool = t;
                }
            }
            ui.separator();
            ui.toggle_value(&mut self.snap, RichText::new("⊞ Ajuste").size(12.5))
                .on_hover_text("Mover en pasos fijos y girar de 15 en 15°");
            if self.snap {
                ui.add(
                    DragValue::new(&mut self.snap_move)
                        .speed(0.01)
                        .range(0.01..=10.0)
                        .suffix(" m"),
                );
                ui.add(
                    DragValue::new(&mut self.snap_rot)
                        .speed(0.5)
                        .range(1.0..=90.0)
                        .suffix("°"),
                );
            }
            ui.separator();
            egui::ComboBox::from_id_salt("view-camera")
                .selected_text(if self.use_scene_camera {
                    "Cámara de la escena"
                } else {
                    "Perspectiva"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.use_scene_camera, false, "Perspectiva (editor)");
                    ui.selectable_value(&mut self.use_scene_camera, true, "Cámara de la escena");
                });
            if let Some(v) = self.viewport.as_mut().filter(|v| v.has_rigs()) {
                ui.separator();
                let text = if v.playing {
                    "⏸ Animar"
                } else {
                    "▶ Animar"
                };
                ui.toggle_value(&mut v.playing, RichText::new(text).size(12.5))
                    .on_hover_text("Reproduce las animaciones de los personajes");
            }
            ui.separator();
            ui.toggle_value(&mut self.show_grid, RichText::new("Rejilla").size(12.5));
            ui.toggle_value(&mut self.show_bounds, RichText::new("Cajas").size(12.5));
            ui.toggle_value(&mut self.show_stats, RichText::new("Info").size(12.5));
        });
        ui.add_space(3.0);
    }

    fn viewport_menu(&mut self, ui: &mut Ui, at: Option<Vec3>) {
        if matches!(self.selection, Selection::Node(_) | Selection::Light(_)) {
            if ui
                .add(egui::Button::new("Duplicar").shortcut_text("Ctrl+D"))
                .clicked()
            {
                self.duplicate_selected();
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("Enfocar").shortcut_text("F"))
                .clicked()
            {
                self.focus_selected();
                ui.close_menu();
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("Borrar").color(theme::ERROR))
                        .shortcut_text("Supr"),
                )
                .clicked()
            {
                self.delete_selected();
                ui.close_menu();
            }
            ui.separator();
        }
        ui.menu_button("Añadir aquí", |ui| self.add_menu(ui, None, at));
        if ui.button("Encuadrar toda la escena").clicked() {
            if let Some(b) = self.viewport.as_ref().map(|v| v.scene_bounds()) {
                self.cam.frame(&b);
            }
            ui.close_menu();
        }
    }

    fn viewport_tab(&mut self, ui: &mut Ui) {
        self.viewport_visible = true;
        self.viewport_toolbar(ui);
        // Only what is visible: content that overflows would widen the panel past its clip.
        let mut rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        rect.set_width(rect.width().max(32.0));
        rect.set_height(rect.height().max(32.0));
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let ppp = ui.ctx().pixels_per_point();
        self.viewport_px = (
            (rect.width() * ppp).round().max(16.0) as u32,
            (rect.height() * ppp).round().max(16.0) as u32,
        );
        let painter = ui.painter_at(rect);
        match self.viewport.as_ref().and_then(|v| v.tex_id) {
            Some(id) => {
                painter.image(
                    id,
                    rect,
                    Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect, 0.0, theme::BG_DARK);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Preparando el visor…",
                    FontId::proportional(14.0),
                    theme::TEXT_DIM,
                );
                self.repaint = true;
            }
        }
        let hover = resp.hover_pos();
        let (alt, shift) = ui.input(|i| (i.modifiers.alt, i.modifiers.shift));

        // The view-axes bubbles (laid out last frame) turn the view.
        let mut consumed = false;
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                if let Some(b) = self
                    .axis_bubbles
                    .iter()
                    .find(|b| (b.pos - p).length() < 11.0)
                {
                    let (axis, positive) = (b.axis, b.positive);
                    self.look_along(axis, positive);
                    consumed = true;
                }
            }
        }
        self.navigate(ui, &resp, rect, alt, shift);
        let view = View::new(self.render_camera(), rect);
        self.last_view = Some((view.cam, rect));
        let tool = self.effective_tool();

        // Transform handles.
        let frame = self.gizmo_frame(&view, tool);
        if self.gdrag.is_none() {
            self.hot = match (frame, hover) {
                (Some(f), Some(p)) if !alt => gizmo::hit(&view, &f, tool, p),
                _ => None,
            };
        }
        if resp.drag_started_by(PointerButton::Primary) && !alt {
            let origin = ui.input(|i| i.pointer.press_origin()).or(hover);
            if let (Some(f), Some(o)) = (frame, origin) {
                if let Some(h) = gizmo::hit(&view, &f, tool, o) {
                    self.begin_gizmo(&view, f, h, o);
                }
            }
        }
        if self.gdrag.is_some() {
            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                self.update_gizmo(&view, p);
            }
            if resp.drag_stopped() || !ui.input(|i| i.pointer.primary_down()) {
                self.gdrag = None;
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if self.hot.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }

        // Picking.
        if resp.clicked() && !consumed && self.hot.is_none() {
            if let Some(p) = resp.interact_pointer_pos() {
                self.pick(&view, p);
            }
        }

        // Right click: a menu for the spot under the pointer.
        if resp.secondary_clicked() {
            self.ctx_point = resp.interact_pointer_pos().and_then(|p| {
                let (o, d) = view.ray(p);
                self.viewport.as_ref()?.surface_point(o, d)
            });
        }
        let ctx_point = self.ctx_point;
        resp.context_menu(|ui| self.viewport_menu(ui, ctx_point));

        // Models dropped from the file system land where they are let go.
        if resp.dnd_hover_payload::<DragFile>().is_some() {
            painter.rect_stroke(
                rect.shrink(2.0),
                4.0,
                Stroke::new(2.0_f32, theme::ACCENT),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(f) = resp.dnd_release_payload::<DragFile>() {
            let at = hover.or(resp.interact_pointer_pos()).and_then(|p| {
                let (o, d) = view.ray(p);
                self.viewport.as_ref()?.surface_point(o, d)
            });
            self.drop_file(&f.0, None, at);
        }

        self.draw_overlays(&painter, &view, rect, hover, tool);
    }

    fn draw_overlays(
        &mut self,
        p: &egui::Painter,
        view: &View,
        rect: Rect,
        hover: Option<Pos2>,
        tool: Tool,
    ) {
        let mut icons = Vec::new();
        if let Some(vp) = &self.viewport {
            if self.show_bounds {
                for b in vp.bounds.values() {
                    gizmo::draw_box(
                        p,
                        view,
                        b,
                        Stroke::new(1.0_f32, Color32::from_white_alpha(38)),
                    );
                }
            }
            if let Selection::Node(path) = &self.selection {
                if let Some(b) = vp.bounds.get(path) {
                    gizmo::draw_box(p, view, b, Stroke::new(1.6_f32, theme::SELECT));
                }
                if let Some(n) = ops::node(&self.scene.nodes, path) {
                    if matches!(n.mesh, MeshKind3D::Empty) {
                        let w = self.world_of(path);
                        let at = Vec3::new(w.w_axis[0], w.w_axis[1], w.w_axis[2]);
                        gizmo::draw_origin(p, view, at, gizmo::local_axes(&w), view.arm(at) * 0.3);
                    }
                }
            }
        }
        for (i, l) in self.scene.lights.iter().enumerate() {
            if let LightDoc::Point {
                position,
                color,
                radius,
                ..
            } = l
            {
                let sel = self.selection == Selection::Light(i);
                if sel {
                    gizmo::draw_sphere(
                        p,
                        view,
                        v3(*position),
                        *radius,
                        theme::SELECT.gamma_multiply(0.45),
                    );
                }
                if let Some(q) = gizmo::draw_light(p, view, v3(*position), *color, sel) {
                    icons.push((q, Selection::Light(i)));
                }
            }
        }
        if !self.use_scene_camera {
            let cam = viewport::camera_from_doc(&self.scene);
            let aspect = rect.width() / rect.height().max(1.0);
            if let Some(q) =
                gizmo::draw_camera(p, view, &cam, aspect, self.selection == Selection::Camera)
            {
                icons.push((q, Selection::Camera));
            }
        }
        if let Some(f) = self.gizmo_frame(view, tool) {
            let hot = self.gdrag.as_ref().map(|d| d.handle).or(self.hot);
            gizmo::draw(p, view, &f, tool, hot);
        }
        self.axis_bubbles = gizmo::draw_view_axes(
            p,
            &view.cam,
            rect.right_top() + egui::vec2(-60.0, 60.0),
            hover,
        );

        let mode = if self.use_scene_camera {
            "Cámara de la escena"
        } else {
            "Perspectiva"
        };
        shadow_text(
            p,
            rect.left_top() + egui::vec2(10.0, 8.0),
            &format!("{mode} · {}", tool.label()),
            12.5,
            theme::TEXT,
        );
        if self.gdrag.is_some() {
            if let Selection::Node(path) = &self.selection {
                if let Some(n) = ops::node(&self.scene.nodes, path) {
                    let t = &n.transform;
                    let text = match tool {
                        Tool::Rotate => format!(
                            "Rotación  {:.1}°  {:.1}°  {:.1}°",
                            t.rotation_deg[0], t.rotation_deg[1], t.rotation_deg[2]
                        ),
                        Tool::Scale => format!(
                            "Escala  {:.3}  {:.3}  {:.3}",
                            t.scale[0], t.scale[1], t.scale[2]
                        ),
                        _ => format!(
                            "Posición  {:.2}  {:.2}  {:.2}",
                            t.position[0], t.position[1], t.position[2]
                        ),
                    };
                    shadow_text(
                        p,
                        rect.left_top() + egui::vec2(10.0, 28.0),
                        &text,
                        12.5,
                        theme::SELECT,
                    );
                }
            }
        }
        if self.show_stats {
            if let Some(vp) = &self.viewport {
                let text = format!(
                    "{} dibujos · {} triángulos · {}×{} px",
                    vp.draws,
                    thousands(vp.triangles),
                    vp.size.0,
                    vp.size.1
                );
                shadow_text(
                    p,
                    rect.left_bottom() + egui::vec2(10.0, -24.0),
                    &text,
                    12.0,
                    theme::TEXT_DIM,
                );
            }
        }
        self.icon_hits = icons;
    }

    // -------------------------------------------------------------- dialogs
    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        match dialog {
            Dialog::OpenScene { mut filter } => {
                let list = self.scenes_list().clone();
                let mut chosen = None;
                let mut cancel = false;
                let r = egui::Modal::new(egui::Id::new("dlg-open")).show(ctx, |ui| {
                    ui.set_width(540.0);
                    heading(ui, "Abrir escena");
                    ui.add(
                        TextEdit::singleline(&mut filter)
                            .hint_text("🔍 Buscar")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(4.0);
                    let f = filter.to_lowercase();
                    ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        let mut any = false;
                        for rel in list.iter().filter(|r| r.to_lowercase().contains(&f)) {
                            any = true;
                            if ui
                                .selectable_label(
                                    false,
                                    RichText::new(format!("🎬  {rel}")).size(13.0),
                                )
                                .clicked()
                            {
                                chosen = Some(rel.clone());
                            }
                        }
                        if !any {
                            dim(ui, "No hay escenas que coincidan.");
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            cancel = true;
                        }
                    });
                });
                if let Some(rel) = chosen {
                    let p = self.root.join(rel);
                    self.request(Pending::Open(p));
                } else if !cancel && !r.should_close() {
                    self.dialog = Some(Dialog::OpenScene { filter });
                }
            }
            Dialog::SaveAs { mut path } => {
                let mut act = None;
                let r = egui::Modal::new(egui::Id::new("dlg-save")).show(ctx, |ui| {
                    ui.set_width(480.0);
                    heading(ui, "Guardar escena como");
                    dim(ui, "Ruta dentro del proyecto:");
                    ui.add(
                        TextEdit::singleline(&mut path)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                    let ok = path.ends_with(".scene3d.json");
                    if !ok {
                        ui.label(
                            RichText::new("Debe terminar en .scene3d.json")
                                .color(theme::WARN)
                                .size(12.0),
                        );
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(ok, egui::Button::new("Guardar")).clicked() {
                            act = Some(true);
                        }
                        if ui.button("Cancelar").clicked() {
                            act = Some(false);
                        }
                    });
                });
                match act {
                    Some(true) => {
                        let full = self.root.join(&path);
                        self.save_to(&full);
                    }
                    Some(false) => {}
                    None => {
                        if !r.should_close() {
                            self.dialog = Some(Dialog::SaveAs { path });
                        }
                    }
                }
            }
            Dialog::AddModel {
                mut filter,
                at,
                parent,
            } => {
                let list = self.models_list().clone();
                let mut chosen = None;
                let mut cancel = false;
                let r = egui::Modal::new(egui::Id::new("dlg-model")).show(ctx, |ui| {
                    ui.set_width(600.0);
                    heading(ui, "Añadir modelo 3D");
                    dim(ui, "Modelos .glb del proyecto. También puedes arrastrarlos del Sistema de archivos al visor.");
                    ui.add_space(4.0);
                    ui.add(TextEdit::singleline(&mut filter).hint_text("🔍 Buscar").desired_width(f32::INFINITY));
                    ui.add_space(4.0);
                    let f = filter.to_lowercase();
                    let shown: Vec<&String> = list.iter().filter(|m| m.to_lowercase().contains(&f)).take(500).collect();
                    ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for rel in &shown {
                            let (dir, name) = match rel.rfind('/') {
                                Some(k) => (&rel[..k], &rel[k + 1..]),
                                None => ("", rel.as_str()),
                            };
                            let mut job = egui::text::LayoutJob::default();
                            job.append("◈  ", 0.0, egui::TextFormat::simple(FontId::proportional(13.0), Color32::from_rgb(0xc4, 0x9c, 0xff)));
                            job.append(name, 0.0, egui::TextFormat::simple(FontId::proportional(13.0), theme::TEXT));
                            job.append(&format!("   {dir}"), 0.0, egui::TextFormat::simple(FontId::proportional(11.5), theme::TEXT_DIM));
                            if ui.selectable_label(false, job).clicked() {
                                chosen = Some((*rel).clone());
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        dim(ui, &format!("{} de {} modelos", shown.len(), list.len()));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cancelar").clicked() {
                                cancel = true;
                            }
                        });
                    });
                });
                if let Some(rel) = chosen {
                    self.add_model(&rel, parent, at);
                } else if !cancel && !r.should_close() {
                    self.dialog = Some(Dialog::AddModel { filter, at, parent });
                }
            }
            Dialog::OpenScript { mut filter } => {
                let list = self.script_files_list().clone();
                let mut chosen = None;
                let mut cancel = false;
                let r = egui::Modal::new(egui::Id::new("dlg-script")).show(ctx, |ui| {
                    ui.set_width(640.0);
                    heading(ui, "Abrir script");
                    dim(ui, "Código y textos del proyecto: Rust, Velvet Script, guiones .md, JSON, WGSL…");
                    ui.add_space(4.0);
                    let field = ui.add(
                        TextEdit::singleline(&mut filter)
                            .hint_text("🔍 Buscar · Intro abre el primero")
                            .desired_width(f32::INFINITY),
                    );
                    if !field.has_focus() && !field.lost_focus() {
                        field.request_focus();
                    }
                    ui.add_space(4.0);
                    let f = filter.to_lowercase();
                    let shown: Vec<&String> = list
                        .iter()
                        .filter(|m| m.to_lowercase().contains(&f))
                        .take(400)
                        .collect();
                    if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        chosen = shown.first().map(|s| (*s).clone());
                    }
                    ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for rel in &shown {
                            let (dir, name) = match rel.rfind('/') {
                                Some(k) => (&rel[..k], &rel[k + 1..]),
                                None => ("", rel.as_str()),
                            };
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &format!("{}  ", files::icon(name, false)),
                                0.0,
                                egui::TextFormat::simple(FontId::proportional(13.0), file_color(name, false)),
                            );
                            job.append(name, 0.0, egui::TextFormat::simple(FontId::proportional(13.0), theme::TEXT));
                            job.append(
                                &format!("   {dir}"),
                                0.0,
                                egui::TextFormat::simple(FontId::proportional(11.5), theme::TEXT_DIM),
                            );
                            if ui.selectable_label(false, job).clicked() {
                                chosen = Some((*rel).clone());
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        dim(ui, &format!("{} de {} archivos", shown.len(), list.len()));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cancelar").clicked() {
                                cancel = true;
                            }
                        });
                    });
                });
                if let Some(rel) = chosen {
                    let p = self.root.join(rel);
                    self.open_script(&p);
                } else if !cancel && !r.should_close() {
                    self.dialog = Some(Dialog::OpenScript { filter });
                }
            }
            Dialog::Discard(then) => {
                let mut choice = None;
                let name = self.scene.name.clone();
                let r = egui::Modal::new(egui::Id::new("dlg-discard")).show(ctx, |ui| {
                    ui.set_width(440.0);
                    heading(ui, "Cambios sin guardar");
                    ui.label(format!("La escena «{name}» tiene cambios sin guardar."));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Guardar").clicked() {
                            choice = Some(0);
                        }
                        if ui.button("No guardar").clicked() {
                            choice = Some(1);
                        }
                        if ui.button("Cancelar").clicked() {
                            choice = Some(2);
                        }
                    });
                });
                match choice {
                    Some(0) => {
                        if self.save() {
                            self.run_pending(then);
                        }
                    }
                    Some(1) => self.run_pending(then),
                    Some(_) => {}
                    None => {
                        if !r.should_close() {
                            self.dialog = Some(Dialog::Discard(then));
                        }
                    }
                }
            }
            Dialog::CloseScript(path) => {
                let mut choice = None;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let r = egui::Modal::new(egui::Id::new("dlg-close-script")).show(ctx, |ui| {
                    ui.set_width(440.0);
                    heading(ui, "Cerrar el archivo");
                    ui.label(format!("«{name}» tiene cambios sin guardar."));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Guardar y cerrar").clicked() {
                            choice = Some(0);
                        }
                        if ui.button("Cerrar sin guardar").clicked() {
                            choice = Some(1);
                        }
                        if ui.button("Cancelar").clicked() {
                            choice = Some(2);
                        }
                    });
                });
                match choice {
                    Some(0) | Some(1) => {
                        if choice == Some(0) {
                            self.save_script(&path);
                        }
                        self.scripts.retain(|s| s.path != path);
                        self.close_tabs.push(Tab::Script(path));
                    }
                    Some(_) => {}
                    None => {
                        if !r.should_close() {
                            self.dialog = Some(Dialog::CloseScript(path));
                        }
                    }
                }
            }
            Dialog::Shortcuts => {
                let r = egui::Modal::new(egui::Id::new("dlg-keys")).show(ctx, |ui| {
                    ui.set_width(520.0);
                    heading(ui, "Atajos de teclado");
                    egui::Grid::new("keys")
                        .num_columns(2)
                        .spacing([24.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for (k, what) in [
                                ("Q / W / E / R", "Seleccionar · mover · rotar · escalar"),
                                ("Clic", "Seleccionar en el visor"),
                                (
                                    "Clic derecho + arrastrar",
                                    "Girar la vista (Alt + clic también)",
                                ),
                                (
                                    "Clic central + arrastrar",
                                    "Desplazar la vista (o Mayús + clic derecho)",
                                ),
                                ("Rueda", "Acercar y alejar"),
                                ("Clic derecho + WASD / Q E", "Volar (Mayús: más rápido)"),
                                ("F", "Enfocar la selección"),
                                ("Ctrl+D", "Duplicar"),
                                ("Supr", "Borrar"),
                                ("F2 / doble clic", "Renombrar"),
                                ("Ctrl+Z / Ctrl+Y", "Deshacer / rehacer"),
                                ("Ctrl+S", "Guardar la escena (o el script abierto)"),
                                ("Ctrl+O / Ctrl+N", "Abrir / nueva escena"),
                                ("Ctrl+P", "Buscar y abrir un script"),
                            ] {
                                ui.label(RichText::new(k).monospace().color(theme::ACCENT));
                                ui.label(what);
                                ui.end_row();
                            }
                        });
                    ui.add_space(8.0);
                    ui.button("Cerrar").clicked()
                });
                if !r.inner && !r.should_close() {
                    self.dialog = Some(Dialog::Shortcuts);
                }
            }
            Dialog::About => {
                let r = egui::Modal::new(egui::Id::new("dlg-about")).show(ctx, |ui| {
                    ui.set_width(460.0);
                    ui.horizontal(|ui| {
                        logo(ui);
                        heading(ui, "Velvet Editor");
                    });
                    ui.label(format!("Velvet Engine {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(6.0);
                    dim(ui, "Editor de escenas 3D del motor: árbol de nodos, inspector, visor con el renderizador del propio motor, scripts y archivos del proyecto.");
                    ui.add_space(4.0);
                    dim(ui, "Hecho en Rust con egui y wgpu.");
                    ui.add_space(8.0);
                    ui.button("Cerrar").clicked()
                });
                if !r.inner && !r.should_close() {
                    self.dialog = Some(Dialog::About);
                }
            }
        }
    }

    fn dnd_preview(&self, ctx: &egui::Context) {
        let label = if let Some(n) = egui::DragAndDrop::payload::<DragNode>(ctx) {
            Some(format!("◇  {}", ops::leaf(&n.0)))
        } else {
            egui::DragAndDrop::payload::<DragFile>(ctx).map(|f| {
                format!(
                    "◈  {}",
                    f.0.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                )
            })
        };
        if let (Some(text), Some(pos)) = (label, ctx.pointer_latest_pos()) {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("dnd-preview"),
            ));
            let galley = painter.layout_no_wrap(text, FontId::proportional(13.0), Color32::WHITE);
            let r = Rect::from_min_size(
                pos + egui::vec2(14.0, 10.0),
                galley.size() + egui::vec2(14.0, 8.0),
            );
            painter.rect_filled(r, 4.0, theme::ACCENT_DIM);
            painter.galley(r.min + egui::vec2(7.0, 4.0), galley, Color32::WHITE);
        }
    }
}

impl TabViewer for State {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        match tab {
            Tab::Scene => "Escena".into(),
            Tab::Files => "Sistema de archivos".into(),
            Tab::Viewport => "Visor 3D".into(),
            Tab::Inspector => "Inspector".into(),
            Tab::Output => "Salida".into(),
            Tab::Image(p) => {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                format!("🖼 {name}").into()
            }
            Tab::Script(p) => {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let dirty = self.scripts.iter().any(|s| &s.path == p && s.dirty());
                format!("{}{name}", if dirty { "● " } else { "" }).into()
            }
        }
    }

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        match tab {
            Tab::Script(p) => egui::Id::new(("script-tab", p.clone())),
            Tab::Image(p) => egui::Id::new(("image-tab", p.clone())),
            other => egui::Id::new(format!("{other:?}")),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Tab) {
        match tab {
            Tab::Scene => self.scene_tab(ui),
            Tab::Files => self.files_tab(ui),
            Tab::Viewport => self.viewport_tab(ui),
            Tab::Inspector => self.inspector_tab(ui),
            Tab::Output => self.output_tab(ui),
            Tab::Script(p) => {
                let p = p.clone();
                self.script_tab(ui, &p);
            }
            Tab::Image(p) => {
                let p = p.clone();
                self.image_tab(ui, &p);
            }
        }
    }

    fn closeable(&mut self, tab: &mut Tab) -> bool {
        matches!(tab, Tab::Script(_) | Tab::Image(_))
    }

    fn on_close(&mut self, tab: &mut Tab) -> bool {
        if let Tab::Image(p) = tab {
            self.pictures.retain(|pic| &pic.path != p);
        }
        if let Tab::Script(p) = tab {
            if self.scripts.iter().any(|s| &s.path == p && s.dirty()) {
                self.dialog = Some(Dialog::CloseScript(p.clone()));
                return false;
            }
            self.scripts.retain(|s| &s.path != p);
        }
        true
    }

    fn clear_background(&self, tab: &Tab) -> bool {
        !matches!(tab, Tab::Viewport | Tab::Image(_))
    }

    fn scroll_bars(&self, _tab: &Tab) -> [bool; 2] {
        [false, false]
    }
}

// ------------------------------------------------------------------ the editor
pub struct Editor {
    dock: DockState<Tab>,
    pub s: State,
}

impl Editor {
    pub fn new(ctx: &egui::Context, root: PathBuf) -> Self {
        theme::install_fonts(ctx);
        theme::apply(ctx);
        let mut s = State::new(root);
        let shown = s.root.display().to_string();
        s.log(Level::Info, format!("Proyecto: {shown}"));
        Self {
            dock: default_dock(),
            s,
        }
    }

    /// One frame of the interface.
    pub fn ui(&mut self, ctx: &egui::Context) {
        let focused = self.dock.find_active_focused().map(|(_, t)| t.clone());
        {
            let s = &mut self.s;
            s.viewport_visible = false;
            s.focused_script = match &focused {
                Some(Tab::Script(p)) => Some(p.clone()),
                _ => None,
            };
            s.shortcuts(ctx);
            s.menu_bar(ctx, matches!(focused, Some(Tab::Viewport) | None));
            s.status_bar(ctx);
            DockArea::new(&mut self.dock)
                .style(theme::dock_style(&ctx.style()))
                .show_close_buttons(true)
                .show_leaf_collapse_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show(ctx, s);
            s.dialogs(ctx);
            s.dnd_preview(ctx);
        }
        let open = std::mem::take(&mut self.s.open_tabs);
        let focus = self.s.focus_tab.take();
        let close = std::mem::take(&mut self.s.close_tabs);
        if std::mem::take(&mut self.s.reset_layout) {
            self.dock = default_dock();
            for s in &self.s.scripts {
                self.s.open_tabs.push(Tab::Script(s.path.clone()));
            }
        }
        for tab in open {
            self.show_tab(tab);
        }
        if let Some(tab) = focus {
            self.show_tab(tab);
        }
        for tab in close {
            if let Some(loc) = self.dock.find_tab(&tab) {
                self.dock.remove_tab(loc);
            }
        }
        self.s.end_frame(ctx);
    }

    fn show_tab(&mut self, tab: Tab) {
        if self.dock.find_tab(&tab).is_none() {
            // New tabs (scripts) open next to the viewport, like Godot's script editor.
            if let Some((surface, node, _)) = self.dock.find_tab(&Tab::Viewport) {
                self.dock.set_focused_node_and_surface((surface, node));
            }
            self.dock.push_to_focused_leaf(tab.clone());
        }
        if let Some(loc) = self.dock.find_tab(&tab) {
            self.dock.set_active_tab(loc);
            self.dock.set_focused_node_and_surface((loc.0, loc.1));
        }
    }

    /// Draws the 3D view into its texture (after `ui`, before egui paints).
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        let s = &mut self.s;
        if !s.viewport_visible {
            return;
        }
        if s.viewport.is_none() {
            s.viewport = Some(Viewport::new(device, queue, 4));
            s.repaint = true;
        }
        let cam = s.render_camera();
        let px = s.viewport_px;
        let show_grid = s.show_grid;
        let mut loaded = Vec::new();
        let problems = match s.viewport.as_mut() {
            Some(vp) => {
                vp.ensure_size(device, egui_renderer, px);
                vp.render(
                    device,
                    queue,
                    &s.root,
                    &s.scene,
                    cam,
                    show_grid,
                    &mut loaded,
                )
            }
            None => Vec::new(),
        };
        if s.viewport.as_ref().is_some_and(|v| v.playing) {
            s.repaint = true;
        }
        for p in problems {
            s.log(Level::Error, p);
        }
        for m in loaded {
            s.log(
                Level::Ok,
                format!(
                    "Modelo cargado: {} — {} partes, {} triángulos, {} texturas, {:.2} × {:.2} × {:.2} m ({} ms)",
                    m.path,
                    m.parts,
                    thousands(m.triangles),
                    m.textures,
                    m.size[0],
                    m.size[1],
                    m.size[2],
                    m.millis
                ),
            );
            s.repaint = true;
        }
    }

    /// Whether the editor wants another frame right away.
    pub fn wants_repaint(&mut self) -> bool {
        std::mem::take(&mut self.s.repaint)
    }

    pub fn title(&self) -> String {
        self.s.title()
    }

    pub fn open_scene(&mut self, path: &Path) {
        self.s.open_scene(path);
    }

    pub fn open_script(&mut self, path: &Path) {
        self.s.open_script(path);
    }

    pub fn select(&mut self, path: &str) {
        if ops::node(&self.s.scene.nodes, path).is_some() {
            self.s.selection = Selection::Node(path.to_string());
        } else {
            self.s
                .log(Level::Warn, format!("No existe el nodo «{path}»"));
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.s.tool = tool;
    }

    pub fn focus_viewport(&mut self) {
        self.s.focus_tab = Some(Tab::Viewport);
    }

    pub fn frame_selection(&mut self) {
        self.s.focus_selected();
    }

    pub fn set_orbit(&mut self, yaw_deg: f32, pitch_deg: f32, distance: Option<f32>) {
        self.s.cam.yaw = yaw_deg.to_radians();
        self.s.cam.pitch = pitch_deg.to_radians().clamp(-1.52, 1.52);
        if let Some(d) = distance {
            self.s.cam.distance = d.max(0.3);
        }
    }

    pub fn request_quit(&mut self) {
        self.s.request_quit();
    }

    // -------------------------------------------------------------- probes for scripted QA
    fn probe_view(&self) -> Option<View> {
        self.s.last_view.map(|(cam, rect)| View::new(cam, rect))
    }

    /// Where a node's box centre is on screen.
    pub fn node_screen(&self, path: &str) -> Option<Pos2> {
        let view = self.probe_view()?;
        let b = self.s.viewport.as_ref()?.bounds.get(path)?;
        view.project(b.center())
    }

    /// A point on a transform handle of the selection, and the handle's centre.
    pub fn handle_screen(&self, handle: Handle) -> Option<(Pos2, Pos2)> {
        let view = self.probe_view()?;
        let tool = self.s.effective_tool();
        let f = self.s.gizmo_frame(&view, tool)?;
        let c = view.project(f.center)?;
        let p = match handle {
            Handle::Axis(i) | Handle::ScaleAxis(i) => {
                view.project(f.center + f.axes[i] * (f.arm * 0.7))?
            }
            Handle::Plane(i) => {
                let (u, v) = (f.axes[(i + 1) % 3], f.axes[(i + 2) % 3]);
                view.project(f.center + (u + v) * (f.arm * 0.36))?
            }
            Handle::ScaleAll => c,
            Handle::Ring(_) => {
                // The first spot round the screen that the ring answers to.

                (0..72).find_map(|k| {
                    let a = k as f32 / 72.0 * std::f32::consts::TAU;
                    let q = c + egui::vec2(a.cos(), a.sin()) * gizmo::ARM_PX * 0.86;
                    (0..40).find_map(|s| {
                        let q = c + (q - c) * (0.6 + s as f32 * 0.02);
                        (gizmo::hit(&view, &f, tool, q) == Some(handle)).then_some(q)
                    })
                })?
            }
        };
        Some((p, c))
    }

    pub fn tree_row_screen(&self, path: &str) -> Option<Pos2> {
        self.s.row_rects.get(path).map(|r| r.center())
    }

    pub fn script_button_screen(&self) -> Option<Pos2> {
        self.s.script_button.map(|r| r.center())
    }

    pub fn open_file(&mut self, path: &Path) {
        self.s.open_file(path);
    }

    /// For an open picture: whether it reached the screen (`None`: not open).
    pub fn picture_shown(&self, path: &Path) -> Option<bool> {
        self.s
            .pictures
            .iter()
            .find(|p| p.path == path)
            .map(|p| p.error.is_none() && p.texture.is_some())
    }

    pub fn dialog_open(&self) -> bool {
        self.s.dialog.is_some()
    }

    pub fn open_scripts(&self) -> Vec<PathBuf> {
        self.s.scripts.iter().map(|s| s.path.clone()).collect()
    }

    pub fn script_area(&self) -> Option<Rect> {
        self.s.script_rect
    }

    pub fn selected(&self) -> Option<String> {
        match &self.s.selection {
            Selection::Node(p) => Some(p.clone()),
            _ => None,
        }
    }

    pub fn transform_of(&self, path: &str) -> Option<TransformDoc> {
        ops::node(&self.s.scene.nodes, path).map(|n| n.transform.clone())
    }

    pub fn has_node(&self, path: &str) -> bool {
        ops::node(&self.s.scene.nodes, path).is_some()
    }

    pub fn save_scene_as(&mut self, path: &Path) -> bool {
        self.s.save_to(path)
    }

    pub fn play_animations(&mut self, on: bool) {
        if let Some(v) = self.s.viewport.as_mut() {
            v.playing = on;
        }
    }

    pub fn should_quit(&self) -> bool {
        self.s.quit
    }
}
