//! Casino menu buttons driven by **velvet-style** (`.vcss`) + procedural chrome.

use crate::render::{outline, panel, text};
use crate::title_font::{draw_font_text, title_font, ui_font};
use crate::ui::theme::{Theme, WH, WW};
use velvet_script_layers::{ScreenBlueprint, ScreenButtonSpec};
use velvet_story::pack_rgb;
use velvet_style::{resolve, Color, ComputedStyle, StyleQuery, Stylesheet};

/// Layout for the main menu button column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonColumnLayout {
    /// Left edge of the column.
    pub x: i32,
    /// Top of the first button.
    pub y0: i32,
    /// Button width.
    pub w: i32,
    /// Button height (may be overridden by `.vcss`).
    pub h: i32,
    /// Vertical gap between buttons.
    pub gap: i32,
}

/// Logical rectangle for one title-menu button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonRect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl ButtonRect {
    /// Whether a logical point lies inside this rectangle.
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// Pointer state used to derive VCSS pseudo states.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuInteraction {
    /// Hovered button index.
    pub hovered: Option<usize>,
    /// Currently pressed button index.
    pub pressed: Option<usize>,
}

impl Default for ButtonColumnLayout {
    fn default() -> Self {
        Self {
            x: 52,
            y0: 244,
            w: 374,
            h: 56,
            gap: 8,
        }
    }
}

impl ButtonColumnLayout {
    /// Resolve menu geometry from the screen's VCSS class.
    pub fn from_style(sheet: &Stylesheet, screen: &ScreenBlueprint) -> Self {
        let class = primary_class(&screen.class, "screen");
        let style = resolve(sheet, &StyleQuery::class(class).with_element("screen"));
        Self {
            x: style.number("menu-x", 52.0).round() as i32,
            y0: style.number("menu-y", 244.0).round() as i32,
            w: style.number("menu-width", 374.0).round() as i32,
            h: style.number("menu-height", 56.0).round() as i32,
            gap: style.number("menu-gap", 8.0).round() as i32,
        }
    }
}

/// Resolve button rectangles from the VS2 screen and VCSS layout.
pub fn button_rects(
    sheet: &Stylesheet,
    screen: &ScreenBlueprint,
    layout: &ButtonColumnLayout,
) -> Vec<ButtonRect> {
    let base = resolve(sheet, &button_query(screen, None, false, false, false));
    let h = base.number("height", layout.h as f32).round() as i32;
    let gap = base.number("gap", layout.gap as f32).round() as i32;
    screen
        .buttons
        .iter()
        .enumerate()
        .map(|(index, _)| ButtonRect {
            x: layout.x,
            y: layout.y0 + index as i32 * (h + gap),
            w: layout.w,
            h,
        })
        .collect()
}

/// Hit-test a logical title-menu point.
pub fn hit_test_button(
    sheet: &Stylesheet,
    screen: &ScreenBlueprint,
    layout: &ButtonColumnLayout,
    x: i32,
    y: i32,
) -> Option<usize> {
    button_rects(sheet, screen, layout)
        .iter()
        .position(|rect| rect.contains(x, y))
}

/// Draw the VS2-authored button column using contextual VCSS rules.
pub fn paint_button_column(
    pixels: &mut [u32],
    theme: &Theme,
    sheet: &Stylesheet,
    screen: &ScreenBlueprint,
    layout: &ButtonColumnLayout,
    selected: usize,
    interaction: MenuInteraction,
) {
    let base = resolve(sheet, &button_query(screen, None, false, false, false));
    let h = base.number("height", layout.h as f32) as i32;
    let gap = base.number("gap", layout.gap as f32) as i32;
    let w = layout.w;

    for (index, item) in screen.buttons.iter().enumerate() {
        let y = layout.y0 + index as i32 * (h + gap);
        let selected_now = index == selected;
        let hovered = interaction.hovered == Some(index);
        let pressed = interaction.pressed == Some(index);
        let style = resolve(
            sheet,
            &button_query(screen, Some(item), selected_now, hovered, pressed),
        );
        paint_one_button(
            pixels,
            theme,
            layout.x,
            y,
            w,
            h,
            item,
            selected_now,
            hovered,
            pressed,
            &style,
        );
    }
}

fn button_query(
    screen: &ScreenBlueprint,
    item: Option<&ScreenButtonSpec>,
    selected: bool,
    hovered: bool,
    pressed: bool,
) -> StyleQuery {
    let mut query = StyleQuery::class("menu-item")
        .with_element("button")
        .with_ancestor_class(primary_class(&screen.class, "screen"));
    if let Some(item) = item {
        query = query.with_id(item.id.clone());
        for class in item.class.split_whitespace() {
            if class != "button" && class != "menu-item" {
                query = query.with_class(class.to_string());
            }
        }
        if !item.enabled {
            query = query.with_state("disabled");
        }
    }
    if selected {
        query = query.with_state("selected").with_state("focus");
    }
    if hovered {
        query = query.with_state("hover");
    }
    if pressed {
        query = query.with_state("active");
    }
    query
}

fn primary_class<'a>(classes: &'a str, fallback: &'a str) -> &'a str {
    classes.split_whitespace().next().unwrap_or(fallback)
}

#[allow(clippy::too_many_arguments)]
fn paint_one_button(
    pixels: &mut [u32],
    _theme: &Theme,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    item: &ScreenButtonSpec,
    selected: bool,
    hovered: bool,
    pressed: bool,
    style: &ComputedStyle,
) {
    let scale = style
        .number("scale", if pressed { 0.985 } else { 1.0 })
        .clamp(0.94, 1.0);
    let inset_x = ((w as f32 * (1.0 - scale)) * 0.5).round() as i32;
    let inset_y = ((h as f32 * (1.0 - scale)) * 0.5).round() as i32;
    let x = x + inset_x;
    let y = y + inset_y;
    let w = (w - inset_x * 2).max(8);
    let h = (h - inset_y * 2).max(8);
    let opacity = style.number("opacity", 1.0).clamp(0.0, 1.0);

    let shadow = style.color("shadow-color", Color::rgb(0, 0, 0));
    let shadow_alpha = style.number("shadow-opacity", 0.45).clamp(0.0, 1.0) * opacity;
    let shadow_x = style.number("shadow-offset-x", 3.0).round() as i32;
    let shadow_y = style.number("shadow-offset-y", 4.0).round() as i32;
    paint_deco_fill(
        pixels,
        x + shadow_x,
        y + shadow_y,
        w,
        h,
        shadow.rgb_tuple(),
        shadow_alpha,
        6,
    );

    let bg = style.background();
    let fg = style.color_text();
    let border = style.border_color();
    let glow = style.color("glow", Color::rgba(0, 0, 0, 0.0));
    let glow_strength = style.number("glow-strength", if selected { 0.85 } else { 0.0 });

    if (selected || hovered) && glow_strength > 0.05 {
        paint_selected_fill(pixels, x, y, w, h, bg, glow, glow_strength);
    } else {
        let near_black = mix_rgb(bg.rgb_tuple(), (3, 3, 9), 0.28);
        paint_deco_fill(
            pixels,
            x,
            y,
            w,
            h,
            near_black,
            bg.a.clamp(0.58, 1.0) * opacity,
            6,
        );
        if hovered {
            paint_deco_fill(
                pixels,
                x + 2,
                y + 2,
                w - 4,
                h - 4,
                glow.rgb_tuple(),
                0.055 * opacity,
                4,
            );
        }
    }

    let radius = style.number("border-radius", 4.0).clamp(0.0, 24.0);
    let border_width = style
        .number("border-width", if selected { 2.0 } else { 1.0 })
        .round()
        .clamp(1.0, 3.0) as i32;
    paint_ornate_gold_border(
        pixels,
        x,
        y,
        w,
        h,
        border.rgb_tuple(),
        selected,
        radius.max(3.0) as i32,
        border_width,
    );

    let icon_size =
        (style.number("icon-size", (h - 18) as f32).round() as i32 + 4).clamp(28, (h - 12).max(28));
    let padding = style.number("padding-x", 14.0).round() as i32;
    let icon_lane = (padding * 2 + icon_size).max(76);
    let icon_x = x + (icon_lane - icon_size) / 2;
    let icon_y = y + (h - icon_size) / 2;
    let fallback_icon = if item.icon.is_empty() {
        "diamond"
    } else {
        item.icon.as_str()
    };
    let icon_name = style.keyword("icon", fallback_icon);
    paint_menu_icon(
        pixels,
        icon_x,
        icon_y,
        icon_size,
        icon_name,
        border.rgb_tuple(),
        selected,
    );

    let divider_x = x + icon_lane;
    paint_deco_divider(
        pixels,
        divider_x,
        y + 10,
        h - 20,
        border.rgb_tuple(),
        if selected { 0.60 } else { 0.28 } * opacity,
    );

    let label_x = divider_x + 22;
    let label_color = fg.rgb_tuple();
    let description_color = style
        .color("description-color", Color::rgb(181, 138, 123))
        .rgb_tuple();
    let hotkey_color = style
        .color("hotkey-color", Color::rgb(201, 184, 150))
        .rgb_tuple();

    // Primary label + authored explanatory copy. The previous menu discarded
    // these VS2 fields, making every option feel like a generic button.
    paint_menu_label(
        pixels,
        label_x,
        y + 23,
        &item.label,
        18.0,
        label_color,
        opacity,
        2,
    );
    if !item.description.is_empty() {
        paint_ui_text(
            pixels,
            label_x,
            y + 43,
            &item.description,
            10.2,
            description_color,
            opacity * 0.88,
            1,
        );
    }

    if !item.hotkey.is_empty() {
        let pill_w = 50;
        let pill_h = 20;
        let pill_x = x + w - pill_w - 12;
        let pill_y = y + 10;
        paint_deco_fill(
            pixels,
            pill_x,
            pill_y,
            pill_w,
            pill_h,
            (8, 5, 15),
            0.94 * opacity,
            4,
        );
        paint_deco_outline(
            pixels,
            pill_x,
            pill_y,
            pill_w,
            pill_h,
            4,
            border.rgb_tuple(),
            if selected { 0.86 } else { 0.46 },
        );
        let key_width = measure_ui_text(&item.hotkey, 9.5);
        paint_ui_text(
            pixels,
            pill_x + (pill_w - key_width) / 2,
            pill_y + 14,
            &item.hotkey,
            9.5,
            hotkey_color,
            opacity,
            1,
        );
    }

    if selected {
        paint_selection_rail(pixels, x - 7, y + 8, h - 16, glow.rgb_tuple());
    }
}

fn measure_ui_text(value: &str, size: f32) -> i32 {
    if let Some(font) = ui_font() {
        value
            .chars()
            .map(|ch| font.rasterize(ch, size).0.advance_width)
            .sum::<f32>()
            .round() as i32
    } else {
        value.chars().count() as i32 * 6
    }
}

fn paint_selection_rail(pixels: &mut [u32], x: i32, y: i32, h: i32, glow: (u8, u8, u8)) {
    for spread in (1..=5).rev() {
        panel(
            pixels,
            WW,
            WH,
            x - spread / 2,
            y - 2,
            3 + spread,
            h + 4,
            glow,
            (6 - spread) as f32 * 0.045,
        );
    }
    panel(pixels, WW, WH, x, y, 3, h, glow, 0.95);
    paint_diamond(pixels, x + 1, y, 3, (242, 181, 113));
    paint_diamond(pixels, x + 1, y + h - 1, 3, (242, 181, 113));
}

#[allow(clippy::too_many_arguments)]
fn paint_menu_label(
    pixels: &mut [u32],
    x: i32,
    baseline: i32,
    value: &str,
    size: f32,
    color: (u8, u8, u8),
    opacity: f32,
    fallback_scale: i32,
) {
    if let Some(font) = title_font() {
        let mut pen_x = x as f32;
        for ch in value.chars() {
            let glyph = ch.to_string();
            draw_font_text(
                pixels,
                font,
                pen_x,
                baseline as f32,
                &glyph,
                size,
                color,
                opacity,
            );
            let (metrics, _) = font.rasterize(ch, size);
            pen_x += metrics.advance_width + 0.7;
        }
    } else {
        paint_ui_text(
            pixels,
            x,
            baseline,
            value,
            size,
            color,
            opacity,
            fallback_scale,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_ui_text(
    pixels: &mut [u32],
    x: i32,
    baseline: i32,
    value: &str,
    size: f32,
    color: (u8, u8, u8),
    opacity: f32,
    fallback_scale: i32,
) {
    if let Some(font) = ui_font() {
        draw_font_text(
            pixels,
            font,
            x as f32,
            baseline as f32,
            value,
            size,
            color,
            opacity,
        );
    } else {
        text(
            pixels,
            WW,
            WH,
            x,
            baseline - fallback_scale * 7,
            value,
            color,
            fallback_scale,
        );
    }
}

fn paint_selected_fill(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    bg: Color,
    glow: Color,
    strength: f32,
) {
    // Broad, restrained violet bloom behind a razor-thin art-deco plate.
    for spread in (2..=6).rev() {
        let alpha = (7 - spread) as f32 * 0.014 * strength;
        paint_deco_fill(
            pixels,
            x - spread,
            y - spread / 2,
            w + spread * 2,
            h + spread,
            glow.rgb_tuple(),
            alpha,
            7,
        );
    }
    paint_deco_fill(pixels, x, y, w, h, (4, 2, 11), 0.98, 6);
    for col in 0..w {
        let t = col as f32 / w as f32;
        let g = (1.0 - (t - 0.20).abs() * 1.45).clamp(0.0, 1.0);
        let g = g * g * strength * 0.32;
        let r = (bg.r as f32 * (1.0 - g) + glow.r as f32 * g) as u8;
        let gr = (bg.g as f32 * (1.0 - g) + glow.g as f32 * g) as u8;
        let b = (bg.b as f32 * (1.0 - g) + glow.b as f32 * g) as u8;
        for row in 0..h {
            let inset = deco_row_inset(row, h, 6);
            if col < inset || col >= w - inset {
                continue;
            }
            let v = 1.0 - ((row as f32 / h as f32) - 0.5).abs() * 1.15;
            let v = v.clamp(0.15, 1.0);
            let a = 0.82 + 0.14 * g * v;
            let px = x + col;
            let py = y + row;
            if px < 0 || py < 0 || px >= WW as i32 || py >= WH as i32 {
                continue;
            }
            let i = (py as u32 * WW + px as u32) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(r, gr, b), a);
        }
    }
    panel(
        pixels,
        WW,
        WH,
        x + 10,
        y + h / 2,
        w - 20,
        1,
        glow.rgb_tuple(),
        0.16 * strength,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_deco_fill(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
    cut: i32,
) {
    if w <= 0 || h <= 0 {
        return;
    }
    for row in 0..h {
        let inset = deco_row_inset(row, h, cut);
        panel(
            pixels,
            WW,
            WH,
            x + inset,
            y + row,
            (w - inset * 2).max(0),
            1,
            rgb,
            opacity,
        );
    }
}

fn deco_row_inset(row: i32, h: i32, cut: i32) -> i32 {
    let cut = cut.max(1).min((h / 2).max(1));
    if row < cut {
        cut - row
    } else if row >= h - cut {
        row - (h - cut - 1)
    } else {
        0
    }
}

fn paint_ornate_gold_border(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    gold: (u8, u8, u8),
    selected: bool,
    corner: i32,
    border_width: i32,
) {
    let gold_hi = if selected {
        (
            gold.0.saturating_add(20),
            gold.1.saturating_add(20),
            gold.2.saturating_add(20),
        )
    } else {
        gold
    };
    let cut = corner.clamp(5, 8);
    for inset in 0..border_width.max(1) {
        paint_deco_outline(
            pixels,
            x + inset,
            y + inset,
            w - inset * 2,
            h - inset * 2,
            cut.saturating_sub(inset).max(3),
            gold_hi,
            if selected { 0.96 } else { 0.78 },
        );
    }
    paint_deco_outline(
        pixels,
        x + 3,
        y + 3,
        w - 6,
        h - 6,
        (cut - 2).max(3),
        gold,
        if selected { 0.62 } else { 0.36 },
    );

    // Small engraved ticks preserve the period look without a noisy frame.
    let tick = 12;
    draw_line_alpha(
        pixels,
        x + cut + 4,
        y + 3,
        x + cut + tick,
        y + 3,
        gold,
        0.72,
    );
    draw_line_alpha(
        pixels,
        x + w - cut - tick,
        y + h - 4,
        x + w - cut - 4,
        y + h - 4,
        gold,
        0.72,
    );
    paint_diamond(pixels, x + cut + 1, y + cut + 1, 2, gold_hi);
    paint_diamond(pixels, x + w - cut - 2, y + h - cut - 2, 2, gold_hi);
}

#[allow(clippy::too_many_arguments)]
fn paint_deco_outline(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    cut: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    if w <= cut * 2 || h <= cut * 2 {
        return;
    }
    let right = x + w - 1;
    let bottom = y + h - 1;
    draw_line_alpha(pixels, x + cut, y, right - cut, y, rgb, opacity);
    draw_line_alpha(pixels, right - cut, y, right, y + cut, rgb, opacity);
    draw_line_alpha(pixels, right, y + cut, right, bottom - cut, rgb, opacity);
    draw_line_alpha(
        pixels,
        right,
        bottom - cut,
        right - cut,
        bottom,
        rgb,
        opacity,
    );
    draw_line_alpha(pixels, right - cut, bottom, x + cut, bottom, rgb, opacity);
    draw_line_alpha(pixels, x + cut, bottom, x, bottom - cut, rgb, opacity);
    draw_line_alpha(pixels, x, bottom - cut, x, y + cut, rgb, opacity);
    draw_line_alpha(pixels, x, y + cut, x + cut, y, rgb, opacity);
}

fn paint_deco_divider(pixels: &mut [u32], x: i32, y: i32, h: i32, rgb: (u8, u8, u8), opacity: f32) {
    if h <= 8 {
        return;
    }
    draw_line_alpha(pixels, x, y + 4, x, y + h - 4, rgb, opacity);
    draw_line_alpha(pixels, x - 2, y + 2, x, y, rgb, opacity);
    draw_line_alpha(pixels, x, y, x + 2, y + 2, rgb, opacity);
    draw_line_alpha(pixels, x - 2, y + h - 2, x, y + h, rgb, opacity);
    draw_line_alpha(pixels, x, y + h, x + 2, y + h - 2, rgb, opacity);
}

#[allow(clippy::too_many_arguments)]
fn draw_line_alpha(
    pixels: &mut [u32],
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        panel(pixels, WW, WH, x0, y0, 1, 1, rgb, opacity);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn mix_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t).round() as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t).round() as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t).round() as u8,
    )
}

fn paint_diamond(pixels: &mut [u32], cx: i32, cy: i32, size: i32, rgb: (u8, u8, u8)) {
    for dy in -size..=size {
        let span = size - dy.abs();
        panel(
            pixels,
            WW,
            WH,
            cx - span,
            cy + dy,
            span * 2 + 1,
            1,
            rgb,
            0.95,
        );
    }
}

fn paint_menu_icon(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    size: i32,
    icon: &str,
    accent: (u8, u8, u8),
    selected: bool,
) {
    let gold = accent;
    let pink = (255, 120, 220);
    let cx = x + size / 2;
    let cy = y + size / 2;
    let s = size / 2 - 2;
    let kind = icon.to_ascii_lowercase();

    match kind.as_str() {
        "play" => paint_arcana_star(pixels, cx, cy, s + 2, gold, pink, selected),
        "diamond" => paint_diamond(pixels, cx, cy, s.clamp(4, 9), gold),
        "star" => paint_arcana_star(pixels, cx, cy, s + 2, gold, pink, selected),
        "cards" => {
            outline(
                pixels,
                WW,
                WH,
                x + 6,
                y + 8,
                size / 2 + 2,
                size - 14,
                gold,
                1,
            );
            outline(
                pixels,
                WW,
                WH,
                x + size / 3,
                y + 6,
                size / 2 + 2,
                size - 14,
                gold,
                1,
            );
        }
        "chip" => {
            draw_circle_outline(pixels, cx, cy, s - 1, gold, 2);
            draw_circle_outline(pixels, cx, cy, s / 2, gold, 1);
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                draw_line_alpha(
                    pixels,
                    cx + dx * (s / 2 + 2),
                    cy + dy * (s / 2 + 2),
                    cx + dx * (s - 2),
                    cy + dy * (s - 2),
                    gold,
                    0.9,
                );
            }
            paint_diamond(pixels, cx, cy, 2, gold);
        }
        "gear" => {
            draw_circle_outline(pixels, cx, cy, s - 4, gold, 2);
            draw_circle_outline(pixels, cx, cy, 4, gold, 1);
            for (dx, dy) in [
                (0, -1),
                (1, -1),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
            ] {
                panel(
                    pixels,
                    WW,
                    WH,
                    cx + dx * (s - 2) - 2,
                    cy + dy * (s - 2) - 2,
                    5,
                    5,
                    gold,
                    0.9,
                );
            }
        }
        "power" => {
            draw_circle_outline(pixels, cx, cy + 2, s - 3, gold, 2);
            panel(pixels, WW, WH, cx - 4, cy - s, 9, s + 2, (4, 3, 10), 1.0);
            panel(pixels, WW, WH, cx - 1, cy - s, 3, s + 3, gold, 0.95);
        }
        _ => {
            draw_circle_outline(pixels, cx, cy + 1, s - 2, gold, 2);
            panel(pixels, WW, WH, cx - 3, cy - s + 2, 7, 5, (8, 6, 16), 1.0);
            panel(pixels, WW, WH, cx - 1, cy - s + 2, 3, s, gold, 0.95);
        }
    }
}

fn paint_arcana_star(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    radius: i32,
    gold: (u8, u8, u8),
    pink: (u8, u8, u8),
    selected: bool,
) {
    if selected {
        for spread in (3..=radius + 5).rev() {
            let alpha = (radius + 6 - spread) as f32 * 0.012;
            panel(
                pixels,
                WW,
                WH,
                cx - spread,
                cy - 2,
                spread * 2 + 1,
                5,
                pink,
                alpha,
            );
            panel(
                pixels,
                WW,
                WH,
                cx - 2,
                cy - spread,
                5,
                spread * 2 + 1,
                pink,
                alpha,
            );
        }
    }
    let vertical = radius.max(6);
    let horizontal = (radius * 3 / 4).max(5);
    for dy in -vertical..=vertical {
        let normalized = dy.abs() as f32 / vertical as f32;
        let half = ((1.0 - normalized) * 5.0).ceil() as i32;
        panel(
            pixels,
            WW,
            WH,
            cx - half,
            cy + dy,
            half * 2 + 1,
            1,
            if selected { pink } else { gold },
            0.92,
        );
    }
    for dx in -horizontal..=horizontal {
        let normalized = dx.abs() as f32 / horizontal as f32;
        let half = ((1.0 - normalized) * 4.0).ceil() as i32;
        panel(
            pixels,
            WW,
            WH,
            cx + dx,
            cy - half,
            1,
            half * 2 + 1,
            gold,
            0.96,
        );
    }
    paint_diamond(pixels, cx, cy, 3, (255, 229, 190));
}

fn draw_circle_outline(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    r: i32,
    rgb: (u8, u8, u8),
    thickness: i32,
) {
    let r2 = r * r;
    let r_in = (r - thickness).max(0);
    let r_in2 = r_in * r_in;
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 && d2 >= r_in2 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && px < WW as i32 && py < WH as i32 {
                    let i = (py as u32 * WW + px as u32) as usize;
                    pixels[i] = blend(pixels[i], pack_rgb(rgb.0, rgb.1, rgb.2), 0.92);
                }
            }
        }
    }
}

fn blend(dst: u32, src: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((src >> 16) & 0xFF) as f32;
    let sg = ((src >> 8) & 0xFF) as f32;
    let sb = (src & 0xFF) as f32;
    pack_rgb(
        (dr + (sr - dr) * t) as u8,
        (dg + (sg - dg) * t) as u8,
        (db + (sb - db) * t) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use velvet_script_layers::parse_screen_source;
    use velvet_style::parse_stylesheet;

    fn authored_menu() -> (Stylesheet, ScreenBlueprint) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let style_source = std::fs::read_to_string(root.join("styles/casino.vcss")).unwrap();
        let screen_source = std::fs::read_to_string(root.join("ui/main_menu.vel")).unwrap();
        let sheet = parse_stylesheet(&style_source).unwrap();
        let screen = parse_screen_source(&screen_source, Some("main_menu.vel"))
            .unwrap()
            .remove(0);
        (sheet, screen)
    }

    #[test]
    fn compact_reference_layout_is_the_safe_default() {
        assert_eq!(
            ButtonColumnLayout::default(),
            ButtonColumnLayout {
                x: 52,
                y0: 244,
                w: 374,
                h: 56,
                gap: 8,
            }
        );
    }

    #[test]
    fn art_deco_fill_preserves_clipped_corners() {
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_deco_fill(&mut pixels, 20, 20, 60, 24, (166, 101, 58), 1.0, 6);
        let at = |x: u32, y: u32| pixels[(y * WW + x) as usize];
        assert_eq!(at(20, 20), 0, "outer corner must remain clipped");
        assert_ne!(at(26, 20), 0, "engraved top edge must be filled");
        assert_ne!(at(20, 30), 0, "straight side must be filled");
    }

    #[test]
    fn vcss_controls_layout_and_pointer_hit_testing() {
        let (sheet, screen) = authored_menu();
        let layout = ButtonColumnLayout::from_style(&sheet, &screen);
        assert_eq!((layout.x, layout.y0, layout.w), (52, 244, 374));
        let rects = button_rects(&sheet, &screen, &layout);
        assert_eq!(rects.len(), 5);
        assert_eq!((rects[0].h, rects[1].y - rects[0].y), (56, 64));
        assert_eq!(hit_test_button(&sheet, &screen, &layout, 60, 250), Some(0));
        assert_eq!(hit_test_button(&sheet, &screen, &layout, 20, 250), None);
    }

    #[test]
    fn descendant_classes_and_pseudo_states_change_computed_style() {
        let (sheet, mut screen) = authored_menu();
        let start = &screen.buttons[0];
        let idle = resolve(
            &sheet,
            &button_query(&screen, Some(start), false, false, false),
        );
        let focused = resolve(
            &sheet,
            &button_query(&screen, Some(start), true, false, false),
        );
        assert_eq!(idle.number("border-width", 0.0), 1.0);
        assert_eq!(focused.number("border-width", 0.0), 1.0);
        assert_ne!(
            idle.background().rgb_tuple(),
            focused.background().rgb_tuple()
        );
        assert!(focused.number("glow-strength", 0.0) > 0.8);

        screen.buttons[0].enabled = false;
        let disabled = resolve(
            &sheet,
            &button_query(&screen, Some(&screen.buttons[0]), false, false, false),
        );
        assert_eq!(disabled.number("opacity", 1.0), 0.45);
    }
}
