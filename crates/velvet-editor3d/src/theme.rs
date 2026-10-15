//! The editor's look: a dark slate workspace with Velvet's violet as the accent, close to what
//! people expect from Godot or Blender, and the dock's tabs dressed to match.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Visuals};

pub const ACCENT: Color32 = Color32::from_rgb(0x9a, 0x5c, 0xff);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x55, 0x37, 0x8f);
pub const BG_DARK: Color32 = Color32::from_rgb(0x16, 0x18, 0x1e);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x20, 0x24, 0x2c);
pub const BG_WIDGET: Color32 = Color32::from_rgb(0x2a, 0x2f, 0x3a);
pub const BG_HOVER: Color32 = Color32::from_rgb(0x35, 0x3b, 0x48);
pub const LINE: Color32 = Color32::from_rgb(0x2f, 0x34, 0x40);
pub const TEXT: Color32 = Color32::from_rgb(0xd7, 0xda, 0xe2);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x90, 0x9d);
pub const AXIS_X: Color32 = Color32::from_rgb(0xf2, 0x4b, 0x5c);
pub const AXIS_Y: Color32 = Color32::from_rgb(0x8b, 0xd9, 0x4d);
pub const AXIS_Z: Color32 = Color32::from_rgb(0x4b, 0x9b, 0xf2);
pub const SELECT: Color32 = Color32::from_rgb(0xff, 0xa3, 0x34);
pub const OK: Color32 = Color32::from_rgb(0x6f, 0xd6, 0x8b);
pub const WARN: Color32 = Color32::from_rgb(0xf2, 0xc1, 0x4b);
pub const ERROR: Color32 = Color32::from_rgb(0xf2, 0x5f, 0x5f);

pub const AXES: [Color32; 3] = [AXIS_X, AXIS_Y, AXIS_Z];

/// Applies the editor's visuals and text sizes to `ctx`.
pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = Visuals::dark();
    v.panel_fill = BG_PANEL;
    v.window_fill = BG_PANEL;
    v.extreme_bg_color = BG_DARK;
    v.faint_bg_color = Color32::from_rgb(0x25, 0x29, 0x33);
    v.code_bg_color = BG_DARK;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = CornerRadius::same(5);
    v.window_stroke = Stroke::new(1.0_f32, LINE);
    v.override_text_color = None;
    {
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = BG_PANEL;
        w.noninteractive.weak_bg_fill = BG_PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
        w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        w.inactive.bg_fill = BG_WIDGET;
        w.inactive.weak_bg_fill = BG_WIDGET;
        w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        w.hovered.bg_fill = BG_HOVER;
        w.hovered.weak_bg_fill = BG_HOVER;
        w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_DIM);
        w.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        w.active.bg_fill = ACCENT_DIM;
        w.active.weak_bg_fill = ACCENT_DIM;
        w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
        w.open.bg_fill = BG_HOVER;
        w.open.weak_bg_fill = BG_HOVER;
        for wv in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            wv.corner_radius = CornerRadius::same(3);
        }
    }
    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(7.0, 3.0);
    style.spacing.interact_size.y = 20.0;
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
    ]
    .into();
    ctx.set_style(style);
}

/// The bold face (Segoe UI Bold when present), for headings.
pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

fn load_font(fonts: &mut egui::FontDefinitions, name: &str, files: &[&str]) -> bool {
    let dir = std::path::Path::new("C:/Windows/Fonts");
    for file in files {
        if let Ok(bytes) = std::fs::read(dir.join(file)) {
            fonts.font_data.insert(
                name.to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            return true;
        }
    }
    false
}

/// Windows' own faces when present (Segoe UI, Cascadia Mono, Segoe UI Symbol for the icons),
/// egui's bundled fonts behind them.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let ui = load_font(&mut fonts, "ui", &["segoeui.ttf"]);
    let has_bold = load_font(&mut fonts, "ui-bold", &["segoeuib.ttf", "seguisb.ttf"]);
    let mono = load_font(&mut fonts, "mono", &["CascadiaMono.ttf", "consola.ttf"]);
    let symbols = load_font(&mut fonts, "symbols", &["seguisym.ttf"]);
    let base = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
        if ui {
            list.insert(0, "ui".into());
        }
        if symbols {
            list.push("symbols".into());
        }
    }
    if let Some(list) = fonts.families.get_mut(&FontFamily::Monospace) {
        if mono {
            list.insert(0, "mono".into());
        }
        if symbols {
            list.push("symbols".into());
        }
    }
    let mut bold_list = Vec::new();
    if has_bold {
        bold_list.push("ui-bold".to_owned());
    } else if ui {
        bold_list.push("ui".to_owned());
    }
    bold_list.extend(base);
    if symbols {
        bold_list.push("symbols".into());
    }
    fonts.families.insert(bold(), bold_list);
    ctx.set_fonts(fonts);
}

/// The dock's tabs and separators in the same colours.
pub fn dock_style(style: &egui::Style) -> egui_dock::Style {
    let mut d = egui_dock::Style::from_egui(style);
    d.tab_bar.bg_fill = BG_DARK;
    d.tab_bar.height = 26.0;
    d.tab_bar.hline_color = ACCENT_DIM;
    d.tab.active.bg_fill = BG_PANEL;
    d.tab.active.text_color = Color32::WHITE;
    d.tab.focused.bg_fill = BG_PANEL;
    d.tab.focused.text_color = Color32::WHITE;
    d.tab.inactive.bg_fill = BG_DARK;
    d.tab.inactive.text_color = TEXT_DIM;
    d.tab.hovered.bg_fill = BG_WIDGET;
    d.tab.hovered.text_color = TEXT;
    d.tab.hline_below_active_tab_name = true;
    d.tab.tab_body.bg_fill = BG_PANEL;
    d.tab.tab_body.stroke = Stroke::NONE;
    d.tab.tab_body.inner_margin = egui::Margin::same(6);
    d.separator.width = 3.0;
    d.separator.color_idle = BG_DARK;
    d.separator.color_hovered = ACCENT_DIM;
    d.separator.color_dragged = ACCENT;
    d.main_surface_border_stroke = Stroke::NONE;
    d
}
