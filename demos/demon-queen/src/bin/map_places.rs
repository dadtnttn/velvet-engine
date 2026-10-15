//! The castle's floor plan and the forest's chart, drawn for the corner
//! minimap and for the full map, with the side panels of the full map
//! (orientation and a list of places). The village keeps its own map code.
use super::map_kit::{self, with_alpha, Place};
use super::*;

// The maps' boxes in the Persona manner of the menus: these stand in for the rounded ones of
// the old look in this file (corners cut on the diagonal instead of rounded; tiny swatches and
// pills keep a plain shape).
fn cut_points(rect: Rect, radius: f32) -> Vec<[f32; 2]> {
    let c = (radius * 0.8).min(rect.w * 0.3).min(rect.h * 0.3);
    vec![
        [rect.x + c, rect.y],
        [rect.x + rect.w, rect.y],
        [rect.x + rect.w, rect.y + rect.h - c],
        [rect.x + rect.w - c, rect.y + rect.h],
        [rect.x, rect.y + rect.h],
        [rect.x, rect.y + c],
    ]
}

fn push_rounded_rect(vertices: &mut Vec<MapVertex>, rect: Rect, radius: f32, color: [f32; 4]) {
    if radius < 6.0 || rect.w < 16.0 || rect.h < 12.0 {
        super::push_rounded_rect(vertices, rect, radius.min(2.0), color);
        return;
    }
    map_kit::cut_plate(vertices, rect, radius * 0.8, color);
}

fn push_rounded_border(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    radius: f32,
    width: f32,
    color: [f32; 4],
) {
    if radius < 6.0 || rect.w < 16.0 || rect.h < 12.0 {
        super::push_rounded_border(vertices, rect, radius.min(2.0), width, color);
        return;
    }
    let pts = cut_points(rect, radius);
    for k in 0..pts.len() {
        let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
        push_segment(vertices, a[0], a[1], b[0], b[1], width, color);
    }
}

fn persona_gradient(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    radius: f32,
    top: [f32; 4],
    bottom: [f32; 4],
) {
    let mid = [
        (top[0] + bottom[0]) * 0.5,
        (top[1] + bottom[1]) * 0.5,
        (top[2] + bottom[2]) * 0.5,
        (top[3] + bottom[3]) * 0.5,
    ];
    map_kit::cut_plate(vertices, Rect::new(rect.x + 5.0, rect.y + 6.0, rect.w, rect.h), radius * 0.8, [0.0, 0.0, 0.0, 0.45]);
    map_kit::cut_plate(vertices, rect, radius * 0.8, mid);
}
use prompt_icons::Icon;

/// Colours of the brothel's floors on its plan: (top, bottom).
fn brothel_floor(id: &str) -> ([f32; 4], [f32; 4]) {
    match id {
        "bar" | "entrada" => ([0.56, 0.54, 0.52, 1.0], [0.42, 0.40, 0.39, 1.0]),
        id if id.starts_with("hab") => ([0.64, 0.30, 0.38, 1.0], [0.48, 0.20, 0.28, 1.0]),
        _ => ([0.82, 0.52, 0.26, 1.0], [0.64, 0.36, 0.17, 1.0]),
    }
}

/// The brothel as a floor plan of the floor she is on: floors by material, the red
/// runner that shows the way, furniture in silhouette, walls with their doorways, the
/// stair, the people and places to go to, the next goal in gold, and her marker.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_brothel(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    yaw: f32,
    expanded: bool,
    level: u8,
    rope_open: bool,
) {
    let place = Place::Brothel;
    let tone = map_kit::tone(place);
    let plan = super::super::brothel::plan();
    let ppm = map.w / (bounds.max_x - bounds.min_x).max(0.001); // pixels per metre
    persona_gradient(
        vertices,
        map,
        16.0,
        [0.060, 0.024, 0.034, 0.99],
        [0.034, 0.012, 0.020, 0.99],
    );
    // North (-z) is up, as the house is seen in the game: the map works in flipped z.
    let pt = |x: f32, z: f32| map_point(x, -z, bounds, map);
    let rect_of =
        |x0: f32, x1: f32, z0: f32, z1: f32| map_world_rect_coords(x0, x1, -z1, -z0, bounds, map);

    // The house's outline, a shade under it.
    let b = plan.bounds;
    let (hz1, hx0) = if level == 1 {
        (plan.gallery_z, b.x0)
    } else {
        (b.z1, b.x0)
    };
    let house = rect_of(hx0, b.x1, b.z0, hz1);
    push_rounded_rect(
        vertices,
        Rect::new(house.x + 4.0, house.y + 6.0, house.w, house.h),
        6.0,
        [0.0, 0.0, 0.0, 0.45],
    );

    // Floors.
    let zones: Vec<_> = plan.zones.iter().filter(|z| z.level == level).collect();
    let here = if plan_contains_stair(plan, player) {
        None
    } else {
        plan.zone_at(player.x, player.z, level)
            .map(|z| z.id.as_str())
    };
    for z in &zones {
        let r = rect_of(z.x0, z.x1, z.z0, z.z1);
        let r = Rect::new(r.x - 0.4, r.y - 0.4, r.w + 0.8, r.h + 0.8);
        let (top, bottom) = brothel_floor(&z.id);
        let first = vertices.len();
        push_rect(vertices, r, top);
        for v in &mut vertices[first..] {
            let t = ((v.position[1] - r.y) / r.h.max(1.0)).clamp(0.0, 1.0);
            v.color = [
                top[0] + (bottom[0] - top[0]) * t,
                top[1] + (bottom[1] - top[1]) * t,
                top[2] + (bottom[2] - top[2]) * t,
                top[3],
            ];
        }
        // Boards (wood) or slabs (stone), faintly.
        let stone = z.id == "bar" || z.id == "entrada";
        let step = if stone { 1.0 } else { 0.5 } * ppm;
        if step > 5.0 {
            let mut t = r.y + step;
            while t < r.y + r.h - 1.0 {
                push_rect(vertices, Rect::new(r.x, t, r.w, 1.0), [0.0, 0.0, 0.0, 0.07]);
                t += step;
            }
            if stone {
                let mut t = r.x + step;
                while t < r.x + r.w - 1.0 {
                    push_rect(vertices, Rect::new(t, r.y, 1.0, r.h), [0.0, 0.0, 0.0, 0.07]);
                    t += step;
                }
            }
        }
        if here == Some(z.id.as_str()) {
            push_rect(vertices, r, [1.0, 0.86, 0.50, 0.08]);
        }
    }
    // Upstairs, the hall below the gallery is a well: the ground floor, dimmed.
    if level == 1 {
        let well = rect_of(plan.stair.x0, b.x1, plan.gallery_z, b.z1);
        push_rect(vertices, well, [0.10, 0.05, 0.06, 0.92]);
        for k in 0..((well.h / 9.0) as usize) {
            let y = well.y + 4.0 + k as f32 * 9.0;
            push_rect(
                vertices,
                Rect::new(well.x, y, well.w, 1.0),
                [1.0, 0.8, 0.7, 0.05],
            );
        }
        let size = if expanded { 17.0 } else { 11.5 };
        push_label_centered(
            font,
            vertices,
            "Vestíbulo · abajo",
            well.x + well.w * 0.5,
            well.y + well.h * 0.5 - size * 0.6,
            size,
            with_alpha(tone.soft, 0.55),
        );
        // The gallery rail over it.
        let rail = rect_of(
            plan.stair.x0,
            b.x1,
            plan.gallery_z - 0.1,
            plan.gallery_z + 0.02,
        );
        push_rect(vertices, rail, [0.36, 0.20, 0.12, 1.0]);
    }

    // The runner: the way from the door to the stair and along the corridor.
    let path = super::super::brothel::runner(level);
    let width = (1.2 * ppm).max(4.0);
    for w in path.windows(2) {
        let (a, b2) = (pt(w[0].0, w[0].1), pt(w[1].0, w[1].1));
        push_segment(
            vertices,
            a.0,
            a.1,
            b2.0,
            b2.1,
            width + 3.0,
            [0.94, 0.70, 0.30, 0.9],
        );
        push_circle(
            vertices,
            b2.0,
            b2.1,
            width * 0.5 + 1.5,
            [0.94, 0.70, 0.30, 0.9],
            14,
        );
    }
    for w in path.windows(2) {
        let (a, b2) = (pt(w[0].0, w[0].1), pt(w[1].0, w[1].1));
        push_segment(
            vertices,
            a.0,
            a.1,
            b2.0,
            b2.1,
            width,
            [0.66, 0.04, 0.10, 1.0],
        );
        push_circle(
            vertices,
            b2.0,
            b2.1,
            width * 0.5,
            [0.66, 0.04, 0.10, 1.0],
            14,
        );
    }

    // The stair: treads, and an arrow showing which way is up.
    let st = &plan.stair;
    let stair = rect_of(st.x0, st.x1, st.z0, st.z1);
    push_rect(vertices, stair, [0.30, 0.16, 0.10, 1.0]);
    let treads = 12;
    for k in 0..treads {
        let x = stair.x + stair.w * k as f32 / treads as f32;
        push_rect(
            vertices,
            Rect::new(
                x + 1.0,
                stair.y + 1.5,
                stair.w / treads as f32 - 2.0,
                stair.h - 3.0,
            ),
            [0.58, 0.40, 0.26, 1.0],
        );
    }
    let (ax, ay) = (stair.x + stair.w * 0.5, stair.y + stair.h * 0.5);
    let dir = if level == 0 { -1.0 } else { 1.0 };
    let arrow = stair.h * 0.34;
    push_triangle(
        vertices,
        [
            [ax + dir * arrow, ay],
            [ax - dir * arrow * 0.3, ay - arrow * 0.8],
            [ax - dir * arrow * 0.3, ay + arrow * 0.8],
        ],
        [1.0, 0.92, 0.72, 0.9],
    );
    if !rope_open && level == 0 {
        // The velvet rope across its foot.
        let (x, y) = pt(st.x1 + 0.42, (st.z0 + st.z1) * 0.5);
        push_segment(
            vertices,
            x,
            y - stair.h * 0.5,
            x,
            y + stair.h * 0.5,
            3.0,
            [0.86, 0.14, 0.22, 1.0],
        );
        push_circle(
            vertices,
            x,
            y - stair.h * 0.5,
            3.0,
            [0.96, 0.78, 0.36, 1.0],
            10,
        );
        push_circle(
            vertices,
            x,
            y + stair.h * 0.5,
            3.0,
            [0.96, 0.78, 0.36, 1.0],
            10,
        );
    }

    // Furniture in silhouette.
    for c in plan.boxes.iter().filter(|c| c.level == level) {
        let r = rect_of(
            c.x - c.w * 0.5,
            c.x + c.w * 0.5,
            c.z - c.d * 0.5,
            c.z + c.d * 0.5,
        );
        if r.w < 2.0 || r.h < 2.0 {
            continue;
        }
        push_rounded_rect(
            vertices,
            Rect::new(r.x + 1.5, r.y + 2.0, r.w, r.h),
            3.0,
            [0.0, 0.0, 0.0, 0.30],
        );
        push_rounded_rect(vertices, r, 3.0, [0.20, 0.085, 0.065, 0.96]);
        push_rounded_border(vertices, r, 3.0, 1.0, [0.86, 0.58, 0.32, 0.65]);
    }

    // Walls: tall ones heavy, low ones lighter; doorways are the gaps.
    for w in plan.walls.iter().filter(|w| w.level == level) {
        let tall = !plan.walls.is_empty() && is_tall(w);
        let thick = if tall {
            (0.4 * ppm).clamp(4.0, 11.0)
        } else {
            (0.3 * ppm).clamp(3.0, 8.0)
        };
        let color = if tall {
            [0.09, 0.035, 0.03, 1.0]
        } else {
            [0.30, 0.14, 0.08, 1.0]
        };
        for &[a, b2] in &w.pieces {
            let (p0, p1) = if w.axis == "x" {
                (pt(a, w.at), pt(b2, w.at))
            } else {
                (pt(w.at, a), pt(w.at, b2))
            };
            push_segment(
                vertices,
                p0.0,
                p0.1,
                p1.0,
                p1.1,
                thick + 2.0,
                [0.0, 0.0, 0.0, 0.55],
            );
            push_segment(vertices, p0.0, p0.1, p1.0, p1.1, thick, color);
            if tall {
                push_segment(
                    vertices,
                    p0.0,
                    p0.1,
                    p1.0,
                    p1.1,
                    1.0,
                    [0.84, 0.62, 0.38, 0.55],
                );
            }
        }
    }
    // The current area's outline, over the walls.
    if let Some(z) = zones.iter().find(|z| here == Some(z.id.as_str())) {
        let r = rect_of(z.x0, z.x1, z.z0, z.z1);
        push_border(
            vertices,
            r,
            if expanded { 2.6 } else { 1.8 },
            [1.0, 0.84, 0.40, 0.95],
        );
    }
    map_kit::vignette(vertices, map, 0.26);

    // Names of the areas (only where there is room for them).
    for z in &zones {
        let r = rect_of(z.x0, z.x1, z.z0, z.z1);
        if z.id == "rellano" {
            continue; // the landing is part of the stair
        }
        let label = z.label.as_str();
        let size = fit_label_size(
            font,
            label,
            r.w - 12.0,
            if expanded { 17.0 } else { 12.0 },
            9.0,
        );
        let width = label_width(font, label, size) + 14.0;
        if width > r.w - 4.0 {
            continue;
        }
        let plate = if expanded {
            Rect::new(r.x + r.w * 0.5 - width * 0.5, r.y + 5.0, width, size + 7.0)
        } else {
            Rect::new(r.x + 4.0, r.y + r.h - size - 11.0, width, size + 7.0)
        };
        push_rounded_rect(vertices, plate, plate.h * 0.5, [0.06, 0.02, 0.03, 0.80]);
        push_label(
            font,
            vertices,
            label,
            plate.x + 7.0,
            plate.y + 3.0,
            size,
            [1.0, 0.94, 0.84, 1.0],
        );
    }

    // People and places.
    let goals = super::super::brothel::goals(level, rope_open);
    for h in plan.hotspots.iter().filter(|h| h.level == level) {
        let is_goal = goals.iter().any(|g| g.id == h.id);
        let (x, y) = pt(h.bubble_x, h.bubble_z);
        let r = if expanded { 15.0 } else { 9.5 };
        // The corner map keeps to what matters: the goal, the stair and the way out as
        // badges; everyone else as a small dot.
        if !expanded && !is_goal && !h.exit && !h.stairs {
            push_circle(vertices, x, y, 4.5, [0.02, 0.01, 0.02, 0.9], 12);
            push_circle(
                vertices,
                x,
                y,
                3.2,
                if h.anchor.is_some() {
                    [0.78, 0.66, 1.0, 1.0]
                } else {
                    [0.96, 0.80, 0.50, 1.0]
                },
                12,
            );
            continue;
        }
        if is_goal {
            push_circle(vertices, x, y, r + 9.0, [1.0, 0.78, 0.30, 0.22], 28);
            push_ring(vertices, x, y, r + 6.0, 2.4, [1.0, 0.84, 0.36, 1.0]);
        }
        push_prompt_badge(
            vertices,
            super::super::brothel::icon_for(h),
            x,
            y,
            r,
            if h.locked { 0.7 } else { 1.0 },
        );
        if expanded {
            let size = 13.5;
            let width = label_width(font, &h.label, size) + 14.0;
            let plate = Rect::new(x - width * 0.5, y + r + 4.0, width, size + 6.0);
            push_rounded_rect(vertices, plate, 6.0, [0.03, 0.01, 0.02, 0.80]);
            push_rounded_border(
                vertices,
                plate,
                6.0,
                1.0,
                if is_goal {
                    [1.0, 0.84, 0.36, 1.0]
                } else {
                    with_alpha(tone.rim, 0.6)
                },
            );
            push_label_centered(
                font,
                vertices,
                &h.label,
                x,
                plate.y + 2.5,
                size,
                if is_goal {
                    [1.0, 0.90, 0.60, 1.0]
                } else {
                    tone.ink
                },
            );
        }
    }

    map_kit::player_pin(
        vertices,
        map,
        bounds,
        Vec3::new(player.x, 0.0, -player.z),
        std::f32::consts::PI - yaw,
        expanded,
    );
    map_kit::compass(font, vertices, map, place, expanded);
    if expanded {
        map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
    }
}

fn plan_contains_stair(plan: &super::super::brothel::Plan, player: Vec3) -> bool {
    let s = &plan.stair;
    player.x > s.x0 && player.x < s.x1 && player.z > s.z0 && player.z < s.z1
}

fn is_tall(w: &super::super::brothel::Wall) -> bool {
    w.kind == "tall"
}

/// Left panel of the brothel's full map: which floor, which room, and the key.
pub(super) fn draw_brothel_orientation(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
    level: u8,
) {
    let place = Place::Brothel;
    let tone = map_kit::tone(place);
    let plan = super::super::brothel::plan();
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::Heart,
        "Orientación",
        "El Corazón Carmesí",
    );
    let card = Rect::new(panel.x + 14.0, panel.y + 96.0, panel.w - 28.0, 112.0);
    persona_gradient(
        vertices,
        card,
        14.0,
        [0.13, 0.05, 0.07, 0.98],
        [0.08, 0.03, 0.045, 0.98],
    );
    map_kit::pill(
        vertices,
        Rect::new(card.x + 12.0, card.y + 10.0, 96.0, 22.0),
        [0.24, 0.08, 0.10, 0.95],
        [1.0, 0.80, 0.44, 0.9],
    );
    push_label_centered(
        font,
        vertices,
        "TU POSICIÓN",
        card.x + 60.0,
        card.y + 13.5,
        12.0,
        [1.0, 0.86, 0.56, 1.0],
    );
    let area = if plan_contains_stair(plan, player) {
        "En la escalera"
    } else {
        plan.zone_at(player.x, player.z, level)
            .map(|z| z.label.as_str())
            .unwrap_or("Pasillo")
    };
    let size = fit_label_size(font, area, card.w - 28.0, 22.0, 13.0);
    push_label(
        font,
        vertices,
        area,
        card.x + 14.0,
        card.y + 42.0,
        size,
        tone.title,
    );
    push_label(
        font,
        vertices,
        if level == 1 {
            "Planta alta"
        } else {
            "Planta baja"
        },
        card.x + 14.0,
        card.y + 76.0,
        15.0,
        tone.soft,
    );

    push_label_shadowed(
        font,
        vertices,
        "Claves del mapa",
        panel.x + 20.0,
        panel.y + 230.0,
        19.0,
        tone.title,
    );
    let mut y = panel.y + 262.0;
    let floor = |id: &'static str| {
        move |v: &mut Vec<MapVertex>, x: f32, cy: f32| {
            let (top, _) = brothel_floor(id);
            push_rounded_rect(v, Rect::new(x - 13.0, cy - 10.0, 26.0, 20.0), 5.0, top);
            push_rounded_border(
                v,
                Rect::new(x - 13.0, cy - 10.0, 26.0, 20.0),
                5.0,
                1.0,
                [0.02, 0.01, 0.02, 0.9],
            );
        }
    };
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        floor("salon"),
        "Madera",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        floor("bar"),
        "Piedra",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        floor("hab1"),
        "Habitaciones",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_segment(v, x - 13.0, cy, x + 13.0, cy, 8.0, [0.62, 0.08, 0.14, 0.95]),
        "Alfombra: el camino",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_ring(v, x, cy, 10.0, 2.4, [1.0, 0.84, 0.36, 1.0]),
        "Siguiente objetivo",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_circle(v, x, cy, 7.0, [1.0, 0.28, 0.58, 1.0], 18),
        "Tu ubicación",
        place,
    );
    let controls = Rect::new(
        panel.x + 14.0,
        panel.y + panel.h - 96.0,
        panel.w - 28.0,
        82.0,
    );
    push_rounded_rect(vertices, controls, 14.0, [0.0, 0.0, 0.0, 0.30]);
    push_rounded_border(
        vertices,
        controls,
        14.0,
        1.0,
        with_alpha(tone.rim_soft, 0.7),
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 26.0,
        "F",
        "Hablar / usar",
        14.0,
        tone.ink,
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 58.0,
        "ESC",
        "Cerrar el mapa",
        14.0,
        tone.ink,
    );
}

/// Right panel of the brothel's full map: who and what is where, the goal first.
pub(super) fn draw_brothel_places(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
    level: u8,
    rope_open: bool,
) {
    let place = Place::Brothel;
    let plan = super::super::brothel::plan();
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::Talk,
        "Lugares",
        if level == 1 {
            "Planta alta"
        } else {
            "Planta baja"
        },
    );
    let goals = super::super::brothel::goals(level, rope_open);
    let mut rows: Vec<_> = plan.hotspots.iter().filter(|h| h.level == level).collect();
    rows.sort_by_key(|h| !goals.iter().any(|g| g.id == h.id));
    let row_h = ((panel.h - 110.0) / rows.len().max(1) as f32).clamp(54.0, 76.0);
    for (i, h) in rows.iter().enumerate() {
        let top = panel.y + 94.0 + i as f32 * row_h;
        if top + row_h > panel.y + panel.h - 8.0 {
            break;
        }
        let row = Rect::new(panel.x + 12.0, top, panel.w - 24.0, row_h - 8.0);
        let d = ((h.x - player.x).powi(2) + (h.z - player.z).powi(2)).sqrt();
        let is_goal = goals.iter().any(|g| g.id == h.id);
        let note = if is_goal {
            "Siguiente objetivo".to_owned()
        } else if h.locked {
            "Cerrada".to_owned()
        } else if h.stairs && !rope_open {
            "Cerrada por el cordón".to_owned()
        } else if d < h.radius {
            "Aquí mismo".to_owned()
        } else {
            format!("{d:.0} m")
        };
        map_kit::place_row(
            font,
            vertices,
            row,
            place,
            super::super::brothel::icon_for(h),
            &h.label,
            &note,
            is_goal,
        );
    }
}

/// Floor colours of the castle's rooms on the plan: (top, bottom).
fn castle_floor(kind: &str) -> ([f32; 4], [f32; 4]) {
    match kind {
        "marble" => ([0.56, 0.50, 0.70, 0.98], [0.42, 0.37, 0.56, 0.98]),
        "wood" => ([0.60, 0.40, 0.27, 0.98], [0.46, 0.29, 0.19, 0.98]),
        "dark" => ([0.34, 0.24, 0.48, 0.98], [0.24, 0.16, 0.36, 0.98]),
        _ => ([0.47, 0.44, 0.58, 0.98], [0.36, 0.33, 0.46, 0.98]),
    }
}

/// The castle's bounds on the map (north, the plan's -z, up).
pub(super) fn castle_bounds() -> MapBounds {
    let [x0, x1, z0, z1] = super::super::castle3d::plan().bounds();
    MapBounds {
        min_x: x0 - 1.5,
        max_x: x1 + 1.5,
        min_z: -z1 - 1.5,
        max_z: -z0 + 1.5,
    }
}

/// The floor of the castle she is on (or nearer to, on the stair), from her height.
pub(super) fn castle_level(player: Vec3) -> u8 {
    u8::from(player.y > super::super::castle3d::plan().level_h * 0.5)
}

/// The castle as a floor plan of the floor she is on: rooms by their floors, thick walls with
/// their doorways, the stair, the room she is in lit, the places to use as landmarks, and her
/// marker. Upstairs, the ground floor's other rooms show as roofs.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_castle(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    yaw: f32,
    expanded: bool,
    picture: Option<[f32; 4]>,
) {
    let place = Place::Castle;
    let p = super::super::castle3d::plan();
    let rect =
        |x0: f32, x1: f32, z0: f32, z1: f32| map_world_rect_coords(x0, x1, -z1, -z0, bounds, map);
    persona_gradient(
        vertices,
        map,
        16.0,
        [0.050, 0.032, 0.070, 0.99],
        [0.028, 0.019, 0.042, 0.99],
    );
    let level = castle_level(player);
    let here = p.room_at(player.x, player.z, level);
    let wall = p.wall_t * 0.5;
    if let Some(picture) = picture {
        // The floor as it is, seen from above; the room she is in ringed.
        push_rect(vertices, map, [0.030, 0.020, 0.045, 1.0]);
        super::push_map_picture(vertices, map, bounds, picture, [1.0, 1.0, 1.0, 1.0]);
        if let Some(i) = here {
            let r = &p.rooms[i];
            let inset = if r.open { 0.2 } else { wall };
            let box_ = rect(r.x0 + inset, r.x1 - inset, r.z0 + inset, r.z1 - inset);
            push_rect(vertices, box_, [1.0, 0.86, 0.45, 0.08]);
            push_border(
                vertices,
                box_,
                if expanded { 3.0 } else { 2.2 },
                [1.0, 0.84, 0.36, 1.0],
            );
        }
    } else {
        // Upstairs: the roofs below, flat and dim, with the parapet's line round them.
        if level == 1 {
            for r in p.rooms.iter().filter(|r| r.level == 0 && !r.open) {
                let roof = rect(r.x0 - wall, r.x1 + wall, r.z0 - wall, r.z1 + wall);
                push_rect(vertices, roof, [0.10, 0.08, 0.14, 0.96]);
                push_border(vertices, roof, 1.0, [0.30, 0.24, 0.40, 0.8]);
            }
        }
        // Walls: a dark slab under every room, a little larger.
        for r in p.rooms.iter().filter(|r| !r.open && r.level == level) {
            push_rect(
                vertices,
                rect(r.x0 - wall, r.x1 + wall, r.z0 - wall, r.z1 + wall),
                [0.012, 0.008, 0.02, 1.0],
            );
        }
        for (i, r) in p.rooms.iter().enumerate() {
            if r.level != level {
                continue;
            }
            let inset = if r.open { 0.2 } else { wall };
            let box_ = rect(r.x0 + inset, r.x1 - inset, r.z0 + inset, r.z1 - inset);
            let (top, bottom) = castle_floor(&r.floor);
            let first = vertices.len();
            push_rect(vertices, box_, top);
            for v in &mut vertices[first..] {
                let t = ((v.position[1] - box_.y) / box_.h.max(1.0)).clamp(0.0, 1.0);
                v.color = [
                    top[0] + (bottom[0] - top[0]) * t,
                    top[1] + (bottom[1] - top[1]) * t,
                    top[2] + (bottom[2] - top[2]) * t,
                    top[3],
                ];
            }
            if r.open {
                push_border(vertices, box_, 1.4, [0.86, 0.66, 0.34, 0.7]);
            }
            if here == Some(i) {
                push_rect(vertices, box_, [1.0, 0.78, 0.30, 0.16]);
                push_border(
                    vertices,
                    box_,
                    if expanded { 3.0 } else { 2.2 },
                    [1.0, 0.84, 0.36, 1.0],
                );
            }
        }
        // The stair: its treads across the footprint (the well, upstairs).
        if let Some(st) = &p.stair {
            push_rect(
                vertices,
                rect(st.x0, st.x1, st.z_top, st.z_bottom),
                [0.08, 0.05, 0.11, 1.0],
            );
            let treads = 10;
            for k in 0..=treads {
                let z = st.z_top + (st.z_bottom - st.z_top) * k as f32 / treads as f32;
                push_rect(
                    vertices,
                    rect(st.x0, st.x1, z - 0.06, z + 0.06),
                    [0.86, 0.66, 0.34, 0.75],
                );
            }
        }
        // Doorways: the floor carried through the wall.
        for d in p.doors.iter().filter(|d| d.level == level) {
            let (x0, x1, z0, z1) = if d.axis == "x" {
                (
                    d.at - wall - 0.05,
                    d.at + wall + 0.05,
                    d.c - d.w / 2.0,
                    d.c + d.w / 2.0,
                )
            } else {
                (
                    d.c - d.w / 2.0,
                    d.c + d.w / 2.0,
                    d.at - wall - 0.05,
                    d.at + wall + 0.05,
                )
            };
            let color = if d.outside {
                [0.95, 0.70, 0.30, 1.0]
            } else {
                [0.40, 0.36, 0.50, 1.0]
            };
            push_rect(vertices, rect(x0, x1, z0, z1), color);
        }
    }
    map_kit::vignette(vertices, map, 0.30);
    // Room names (the full map), the places to use as landmarks.
    if expanded {
        for r in p.rooms.iter().filter(|r| r.level == level && !r.open) {
            // In the room's top left corner, clear of the places marked in it (the balcony's
            // place names it).
            let wall = p.wall_t * 0.5 + 0.35;
            let (x, y) = map_point(r.x0 + wall, -(r.z0 + wall), bounds, map);
            let room_w =
                (r.x1 - r.x0 - wall * 2.0) * map.w / (bounds.max_x - bounds.min_x).max(0.001);
            let size = fit_label_size(font, &r.title, room_w - 20.0, 15.0, 10.0);
            let width = label_width(font, &r.title, size) + 20.0;
            let plate = Rect::new(x, y, width, size + 8.0);
            push_rounded_rect(vertices, plate, plate.h * 0.5, [0.05, 0.03, 0.07, 0.86]);
            push_rounded_border(vertices, plate, plate.h * 0.5, 1.0, [0.86, 0.66, 0.34, 0.8]);
            push_label(
                font,
                vertices,
                &r.title,
                plate.x + 10.0,
                plate.y + 3.5,
                size,
                [1.0, 0.95, 0.82, 1.0],
            );
        }
    }
    for h in p
        .hotspots
        .iter()
        .filter(|h| h.id != "lira" && h.id != "nemi" && h.level == level)
    {
        if expanded
            || here.is_some_and(|i| p.room_at(h.x, h.z, level) == Some(i))
            || h.id == "porton"
        {
            map_kit::landmark(
                font,
                vertices,
                map,
                bounds,
                (h.x, -h.z),
                super::super::castle3d::icon(&h.icon),
                &h.label,
                place,
                expanded,
                false,
            );
        }
    }
    map_kit::player_pin(
        vertices,
        map,
        bounds,
        Vec3::new(player.x, player.y, -player.z),
        std::f32::consts::PI - yaw,
        expanded,
    );
    map_kit::compass(font, vertices, map, place, expanded);
    if expanded {
        map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
    }
}

/// El Peñascal from above: its open places in the rock's colour, the houses dug into the cliffs
/// darker, the doorways, the market stone, the place she is in ringed, and its spots.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_penascal(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    yaw: f32,
    expanded: bool,
    picture: Option<[f32; 4]>,
) {
    let place = Place::Penascal;
    let p = super::super::penascal3d::plan();
    let rect =
        |x0: f32, x1: f32, z0: f32, z1: f32| map_world_rect_coords(x0, x1, -z1, -z0, bounds, map);
    let here = p.room_at(player.x, player.z);
    let wall = p.wall_t * 0.5;
    push_rect(vertices, map, [0.035, 0.028, 0.045, 1.0]);
    if let Some(picture) = picture {
        // The village as it is, seen from above; the place she is in ringed.
        super::push_map_picture(vertices, map, bounds, picture, [1.0, 1.0, 1.0, 1.0]);
        if let Some(i) = here {
            let r = &p.rooms[i];
            let inset = if r.open { 0.2 } else { wall };
            let box_ = rect(r.x0 + inset, r.x1 - inset, r.z0 + inset, r.z1 - inset);
            push_rect(vertices, box_, [1.0, 0.86, 0.45, 0.06]);
            push_border(
                vertices,
                box_,
                if expanded { 3.0 } else { 2.2 },
                [1.0, 0.84, 0.36, 1.0],
            );
        }
    } else {
        draw_penascal_plan(vertices, map, bounds, here, expanded);
    }
    map_kit::vignette(vertices, map, 0.30);
    penascal_marks(
        font, vertices, map, bounds, player, yaw, expanded, here, place,
    );
}

/// El Peñascal drawn as a plan (before its picture is baked).
fn draw_penascal_plan(
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    here: Option<usize>,
    expanded: bool,
) {
    let p = super::super::penascal3d::plan();
    let rect =
        |x0: f32, x1: f32, z0: f32, z1: f32| map_world_rect_coords(x0, x1, -z1, -z0, bounds, map);
    let wall = p.wall_t * 0.5;
    // The crags round the village: a darker slab under every place, a little larger.
    for r in &p.rooms {
        push_rect(
            vertices,
            rect(
                r.x0 - wall - 0.4,
                r.x1 + wall + 0.4,
                r.z0 - wall - 0.4,
                r.z1 + wall + 0.4,
            ),
            [0.016, 0.012, 0.022, 1.0],
        );
    }
    for (i, r) in p.rooms.iter().enumerate() {
        let inset = if r.open { 0.2 } else { wall };
        let box_ = rect(r.x0 + inset, r.x1 - inset, r.z0 + inset, r.z1 - inset);
        let (top, bottom) = if r.open {
            ([0.30, 0.27, 0.30, 1.0], [0.22, 0.20, 0.23, 1.0])
        } else {
            ([0.20, 0.15, 0.13, 1.0], [0.14, 0.10, 0.09, 1.0])
        };
        let first = vertices.len();
        push_rect(vertices, box_, top);
        for v in &mut vertices[first..] {
            let t = ((v.position[1] - box_.y) / box_.h.max(1.0)).clamp(0.0, 1.0);
            v.color = [
                top[0] + (bottom[0] - top[0]) * t,
                top[1] + (bottom[1] - top[1]) * t,
                top[2] + (bottom[2] - top[2]) * t,
                top[3],
            ];
        }
        if here == Some(i) {
            push_rect(vertices, box_, [1.0, 0.78, 0.30, 0.14]);
            push_border(
                vertices,
                box_,
                if expanded { 3.0 } else { 2.2 },
                [1.0, 0.84, 0.36, 1.0],
            );
        } else if r.open {
            push_border(vertices, box_, 1.2, [0.70, 0.60, 0.74, 0.5]);
        }
    }
    // Doorways: the floor carried through the rock.
    for d in &p.doors {
        let (x0, x1, z0, z1) = if d.axis == "x" {
            (
                d.at - wall - 0.05,
                d.at + wall + 0.05,
                d.c - d.w / 2.0,
                d.c + d.w / 2.0,
            )
        } else {
            (
                d.c - d.w / 2.0,
                d.c + d.w / 2.0,
                d.at - wall - 0.05,
                d.at + wall + 0.05,
            )
        };
        let color = if d.outside {
            [0.95, 0.70, 0.30, 1.0]
        } else {
            [0.40, 0.36, 0.48, 1.0]
        };
        push_rect(vertices, rect(x0, x1, z0, z1), color);
    }
    // The market stone.
    let s = p.stone;
    push_rect(
        vertices,
        rect(
            s.x - s.w / 2.0,
            s.x + s.w / 2.0,
            s.z - s.d / 2.0,
            s.z + s.d / 2.0,
        ),
        [0.52, 0.48, 0.50, 1.0],
    );
}

/// El Peñascal's names, spots (in the corner map only the way out and the chief), her pin, the
/// compass and the scale.
#[allow(clippy::too_many_arguments)]
fn penascal_marks(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    yaw: f32,
    expanded: bool,
    here: Option<usize>,
    place: Place,
) {
    let p = super::super::penascal3d::plan();
    let wall = p.wall_t * 0.5;
    if expanded {
        for r in p.rooms.iter() {
            let (x, y) = map_point(r.x0 + wall + 0.4, -(r.z0 + wall + 0.4), bounds, map);
            let room_w =
                (r.x1 - r.x0 - wall * 2.0 - 0.8) * map.w / (bounds.max_x - bounds.min_x).max(0.001);
            if room_w < 60.0 {
                continue;
            }
            let size = fit_label_size(font, &r.title, room_w - 20.0, 15.0, 10.0);
            let width = label_width(font, &r.title, size) + 20.0;
            let plate = Rect::new(x, y, width, size + 8.0);
            push_rounded_rect(vertices, plate, plate.h * 0.5, [0.05, 0.03, 0.07, 0.86]);
            push_rounded_border(vertices, plate, plate.h * 0.5, 1.0, [0.86, 0.66, 0.34, 0.8]);
            push_label(
                font,
                vertices,
                &r.title,
                plate.x + 10.0,
                plate.y + 3.5,
                size,
                [1.0, 0.95, 0.82, 1.0],
            );
        }
    }
    let _ = here;
    for h in &p.hotspots {
        if expanded || h.id == "salida" || h.id == "varrak" {
            map_kit::landmark(
                font,
                vertices,
                map,
                bounds,
                (h.x, -h.z),
                super::super::penascal3d::icon(&h.icon),
                &h.label,
                place,
                expanded,
                false,
            );
        }
    }
    map_kit::player_pin(
        vertices,
        map,
        bounds,
        Vec3::new(player.x, player.y, -player.z),
        std::f32::consts::PI - yaw,
        expanded,
    );
    map_kit::compass(font, vertices, map, place, expanded);
    if expanded {
        map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
    }
}

/// Left panel of the castle's full map: where you are and what the colours are.
pub(super) fn draw_castle_orientation(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
) {
    let place = Place::Castle;
    let tone = map_kit::tone(place);
    let p = super::super::castle3d::plan();
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::Council,
        "Orientación",
        "Tu lugar en el refugio",
    );
    let card = Rect::new(panel.x + 14.0, panel.y + 96.0, panel.w - 28.0, 104.0);
    persona_gradient(
        vertices,
        card,
        14.0,
        [0.10, 0.06, 0.13, 0.98],
        [0.06, 0.036, 0.08, 0.98],
    );
    map_kit::pill(
        vertices,
        Rect::new(card.x + 12.0, card.y + 10.0, 96.0, 22.0),
        [0.20, 0.10, 0.06, 0.95],
        [1.0, 0.80, 0.44, 0.9],
    );
    push_label_centered(
        font,
        vertices,
        "TU POSICIÓN",
        card.x + 60.0,
        card.y + 13.5,
        12.0,
        [1.0, 0.86, 0.56, 1.0],
    );
    let level = castle_level(player);
    let name = p
        .room_at(player.x, player.z, level)
        .map_or("Umbral", |i| p.rooms[i].title.as_str());
    let size = fit_label_size(font, name, card.w - 28.0, 21.0, 13.0);
    push_label(
        font,
        vertices,
        name,
        card.x + 14.0,
        card.y + 42.0,
        size,
        tone.title,
    );
    let floor = if level == 1 {
        "Planta alta"
    } else {
        "Planta baja"
    };
    push_label(
        font,
        vertices,
        &format!("{floor}    X {:+.1}  Z {:+.1}", player.x, player.z),
        card.x + 14.0,
        card.y + 74.0,
        14.5,
        tone.soft,
    );

    push_label_shadowed(
        font,
        vertices,
        "Claves del mapa",
        panel.x + 20.0,
        panel.y + 222.0,
        19.0,
        tone.title,
    );
    let rows: [(&str, [f32; 4]); 4] = [
        ("Piedra", castle_floor("stone").0),
        ("Mármol", castle_floor("marble").0),
        ("Madera", castle_floor("wood").0),
        ("Piedra del sello", castle_floor("dark").0),
    ];
    let mut y = panel.y + 256.0;
    for (label, color) in rows {
        map_kit::legend_row(
            font,
            vertices,
            panel.x + 12.0,
            y,
            panel.w - 24.0,
            |v, x, cy| {
                push_rounded_rect(v, Rect::new(x - 13.0, cy - 10.0, 26.0, 20.0), 5.0, color);
                push_rounded_border(
                    v,
                    Rect::new(x - 13.0, cy - 10.0, 26.0, 20.0),
                    5.0,
                    1.0,
                    [0.02, 0.01, 0.03, 0.9],
                );
            },
            label,
            place,
        );
        y += 42.0;
    }
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_prompt_badge(v, Icon::Leave, x, cy, 12.0, 1.0),
        "Portón al bosque",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_rounded_rect(
                v,
                Rect::new(x - 13.0, cy - 10.0, 26.0, 20.0),
                4.0,
                [0.08, 0.05, 0.11, 1.0],
            );
            for k in 0..4 {
                push_rect(
                    v,
                    Rect::new(x - 13.0, cy - 9.0 + k as f32 * 6.0, 26.0, 1.6),
                    [0.86, 0.66, 0.34, 0.9],
                );
            }
        },
        "Escalera entre plantas",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_circle(v, x, cy, 7.0, [1.0, 0.28, 0.58, 1.0], 18),
        "Tu ubicación",
        place,
    );
    let controls = Rect::new(
        panel.x + 14.0,
        panel.y + panel.h - 96.0,
        panel.w - 28.0,
        82.0,
    );
    push_rounded_rect(vertices, controls, 14.0, [0.0, 0.0, 0.0, 0.30]);
    push_rounded_border(
        vertices,
        controls,
        14.0,
        1.0,
        with_alpha(tone.rim_soft, 0.7),
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 26.0,
        "F",
        "Usar lo que tienes cerca",
        14.0,
        tone.ink,
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 58.0,
        "ESC",
        "Cerrar el mapa",
        14.0,
        tone.ink,
    );
}

/// Right panel of the castle's full map: the rooms of the floor she is on and how far each is.
pub(super) fn draw_castle_rooms(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
) {
    let place = Place::Castle;
    let p = super::super::castle3d::plan();
    map_kit::side_panel(vertices, panel, place);
    let level = castle_level(player);
    let subtitle = if level == 1 {
        "La planta alta"
    } else {
        "La planta baja"
    };
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::Castle,
        "Estancias",
        subtitle,
    );
    let here = p.room_at(player.x, player.z, level);
    let rooms: Vec<(usize, &super::super::castle3d::Room)> = p
        .rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| r.level == level)
        .collect();
    let row_h = ((panel.h - 110.0) / rooms.len().max(1) as f32).clamp(44.0, 86.0);
    for (k, (i, r)) in rooms.into_iter().enumerate() {
        let (cx, cz) = r.centre();
        let d = (cx - player.x).hypot(cz - player.z);
        let row = Rect::new(
            panel.x + 12.0,
            panel.y + 94.0 + k as f32 * row_h,
            panel.w - 24.0,
            row_h - 6.0,
        );
        let note = if here == Some(i) {
            "Estás aquí".to_owned()
        } else {
            format!("{d:.0} m")
        };
        map_kit::place_row(
            font,
            vertices,
            row,
            place,
            super::super::castle3d::icon(&r.icon),
            &r.title,
            &note,
            here == Some(i),
        );
    }
}

// ------------------------------------------------------------------- forest

/// The forest map's bounds in the corner: tight around the player.
pub(super) fn focused_forest_bounds(player: Vec3, content: Rect) -> MapBounds {
    let half_height = 15.0;
    let half_width = half_height * (content.w / content.h.max(1.0)).max(0.35);
    MapBounds {
        min_x: player.x - half_width,
        max_x: player.x + half_width,
        min_z: player.z - half_height,
        max_z: player.z + half_height,
    }
}

/// Where the forest's places are: castle gate, the mage, the mine, the village gate.
pub(super) fn forest_places() -> [(Icon, &'static str, Vec3, f32); 4] {
    let mine = super::super::mine::entry_position();
    [
        (
            Icon::Castle,
            "Castillo",
            super::super::castle_entry_position(),
            2.65,
        ),
        (Icon::Talk, "Mago", super::super::mage_position(), 2.65),
        (
            Icon::Mine,
            "Mina",
            mine,
            super::super::mine::INTERACT_RADIUS,
        ),
        (
            Icon::House,
            "Villa",
            super::super::village_entry_position(),
            2.65,
        ),
    ]
}

/// The Dark Forest from its picture, north up (`bounds` in map space: x, and -z up): the wood
/// seen from above, its places, her pin.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_forest_picture(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    picture: [f32; 4],
    player: Vec3,
    yaw: f32,
    expanded: bool,
) {
    let place = Place::Forest;
    push_rect(vertices, map, [0.020, 0.045, 0.035, 1.0]);
    super::push_map_picture(vertices, map, bounds, picture, [1.0, 1.0, 1.0, 1.0]);
    map_kit::vignette(vertices, map, 0.28);
    for (icon, label, at, _) in forest_places() {
        map_kit::landmark(
            font,
            vertices,
            map,
            bounds,
            (at.x, -at.z),
            icon,
            label,
            place,
            expanded,
            false,
        );
    }
    map_kit::player_pin(
        vertices,
        map,
        bounds,
        Vec3::new(player.x, player.y, -player.z),
        std::f32::consts::PI - yaw,
        expanded,
    );
    map_kit::compass(font, vertices, map, place, expanded);
    map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
}

/// Left panel of the forest's full map.
pub(super) fn draw_forest_orientation(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
) {
    let place = Place::Forest;
    let tone = map_kit::tone(place);
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::Forest,
        "Orientación",
        "Tu paso por el bosque",
    );
    let card = Rect::new(panel.x + 14.0, panel.y + 96.0, panel.w - 28.0, 104.0);
    persona_gradient(
        vertices,
        card,
        14.0,
        [0.04, 0.11, 0.09, 0.98],
        [0.02, 0.07, 0.06, 0.98],
    );
    map_kit::pill(
        vertices,
        Rect::new(card.x + 12.0, card.y + 10.0, 96.0, 22.0),
        [0.20, 0.13, 0.05, 0.95],
        [1.0, 0.80, 0.44, 0.9],
    );
    push_label_centered(
        font,
        vertices,
        "TU POSICIÓN",
        card.x + 60.0,
        card.y + 13.5,
        12.0,
        [1.0, 0.86, 0.56, 1.0],
    );
    let (near_name, near_d) = forest_places()
        .iter()
        .map(|(_, name, at, _)| {
            (
                *name,
                ((at.x - player.x).powi(2) + (at.z - player.z).powi(2)).sqrt(),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or(("", 0.0));
    push_label(
        font,
        vertices,
        &format!("Cerca de: {near_name}"),
        card.x + 14.0,
        card.y + 42.0,
        19.0,
        tone.title,
    );
    push_label(
        font,
        vertices,
        &format!("{near_d:.0} m   ·   X {:+.0}  Z {:+.0}", player.x, player.z),
        card.x + 14.0,
        card.y + 74.0,
        14.5,
        tone.soft,
    );

    push_label_shadowed(
        font,
        vertices,
        "Claves del mapa",
        panel.x + 20.0,
        panel.y + 222.0,
        19.0,
        tone.title,
    );
    let mut y = panel.y + 256.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_segment(v, x - 14.0, cy, x + 14.0, cy, 9.0, [0.13, 0.09, 0.05, 1.0]);
            push_segment(v, x - 14.0, cy, x + 14.0, cy, 6.0, [0.62, 0.48, 0.28, 1.0]);
        },
        "Sendero",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_circle(v, x, cy, 11.0, [0.10, 0.30, 0.21, 1.0], 18);
            push_circle(v, x - 3.0, cy - 3.0, 6.0, [0.20, 0.44, 0.30, 1.0], 12);
        },
        "Árboles del bosque",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_circle(v, x, cy, 11.0, [1.0, 0.62, 0.22, 0.25], 18);
            push_circle(v, x, cy, 4.5, [1.0, 0.88, 0.52, 1.0], 12);
        },
        "Farolas del sendero",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_circle(v, x, cy, 11.0, [0.3, 0.9, 0.9, 0.22], 18);
            push_circle(v, x, cy, 4.0, [0.4, 0.95, 0.95, 1.0], 12);
        },
        "Hongos que brillan",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_circle(v, x, cy, 7.0, [1.0, 0.28, 0.58, 1.0], 18),
        "Tu ubicación",
        place,
    );
    let controls = Rect::new(
        panel.x + 14.0,
        panel.y + panel.h - 96.0,
        panel.w - 28.0,
        82.0,
    );
    push_rounded_rect(vertices, controls, 14.0, [0.0, 0.0, 0.0, 0.30]);
    push_rounded_border(
        vertices,
        controls,
        14.0,
        1.0,
        with_alpha(tone.rim_soft, 0.7),
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 26.0,
        "F",
        "Usar el acceso cercano",
        14.0,
        tone.ink,
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 58.0,
        "ESC",
        "Cerrar el mapa",
        14.0,
        tone.ink,
    );
}

/// Right panel of the forest's full map: the places and their distance.
pub(super) fn draw_forest_places(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
) {
    let place = Place::Forest;
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::Forest,
        "Lugares",
        "Bosque Oscuro",
    );
    let places = forest_places();
    let row_h = ((panel.h - 130.0) / places.len() as f32).clamp(70.0, 100.0);
    for (i, (icon, name, at, radius)) in places.iter().enumerate() {
        let d = ((at.x - player.x).powi(2) + (at.z - player.z).powi(2)).sqrt();
        let near = d <= *radius;
        let full = match *name {
            "Castillo" => "Castillo de la Reina",
            "Mago" => "El mago",
            "Mina" => "Mina · Cavernas del Sello",
            _ => "Villa del valle",
        };
        let note = if near {
            "Cerca · pulsa F".to_owned()
        } else {
            format!("{d:.0} m")
        };
        let row = Rect::new(
            panel.x + 12.0,
            panel.y + 94.0 + i as f32 * row_h,
            panel.w - 24.0,
            row_h - 8.0,
        );
        map_kit::place_row(font, vertices, row, place, *icon, full, &note, near);
    }
}

// ------------------------------------------------------------------- village

fn village_icon(name: &str) -> Icon {
    match name {
        "Tienda" => Icon::Shop,
        "Taberna" => Icon::Tavern,
        "Termas" | "Baños termales" => Icon::Bath,
        "Consejo" | "Burgomaestre" | "Casa del Burgomaestre" => Icon::Council,
        "Burdel" => Icon::Heart,
        _ => Icon::House,
    }
}

/// Left panel of the village's full map.
pub(super) fn draw_village_orientation(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
) {
    let place = Place::Village;
    let tone = map_kit::tone(place);
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::House,
        "Orientación",
        "Tu recorrido por la villa",
    );
    let centre = super::super::fantasy_village_center();
    let card = Rect::new(panel.x + 14.0, panel.y + 96.0, panel.w - 28.0, 104.0);
    persona_gradient(
        vertices,
        card,
        14.0,
        [0.13, 0.10, 0.06, 0.98],
        [0.08, 0.064, 0.04, 0.98],
    );
    map_kit::pill(
        vertices,
        Rect::new(card.x + 12.0, card.y + 10.0, 96.0, 22.0),
        [0.20, 0.10, 0.06, 0.95],
        [1.0, 0.80, 0.44, 0.9],
    );
    push_label_centered(
        font,
        vertices,
        "TU POSICIÓN",
        card.x + 60.0,
        card.y + 13.5,
        12.0,
        [1.0, 0.86, 0.56, 1.0],
    );
    push_label(
        font,
        vertices,
        &format!("X {:+.1}", player.x - centre.x),
        card.x + 14.0,
        card.y + 44.0,
        19.0,
        tone.title,
    );
    push_label(
        font,
        vertices,
        &format!("Z {:+.1}", player.z - centre.z),
        card.x + 14.0,
        card.y + 72.0,
        19.0,
        tone.title,
    );

    push_label_shadowed(
        font,
        vertices,
        "Claves del mapa",
        panel.x + 20.0,
        panel.y + 222.0,
        19.0,
        tone.title,
    );
    let mut y = panel.y + 256.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_rounded_rect(
                v,
                Rect::new(x - 15.0, cy - 6.0, 30.0, 12.0),
                5.0,
                [0.86, 0.76, 0.55, 1.0],
            );
        },
        "Camino",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_triangle(
                v,
                [[x, cy - 12.0], [x - 13.0, cy], [x + 13.0, cy]],
                [0.71, 0.37, 0.23, 1.0],
            );
            push_rect(
                v,
                Rect::new(x - 10.0, cy, 20.0, 10.0),
                [0.86, 0.78, 0.58, 1.0],
            );
        },
        "Edificio",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            push_circle(v, x, cy, 10.0, [0.22, 0.50, 0.30, 1.0], 16);
            push_circle(v, x - 3.0, cy - 3.0, 5.0, [0.36, 0.64, 0.40, 1.0], 12);
        },
        "Bosque",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| {
            for k in 0..3 {
                push_segment(
                    v,
                    x - 14.0 + k as f32 * 11.0,
                    cy,
                    x - 8.0 + k as f32 * 11.0,
                    cy,
                    3.0,
                    [0.90, 0.88, 0.55, 1.0],
                );
            }
        },
        "Límite de villa",
        place,
    );
    y += 42.0;
    map_kit::legend_row(
        font,
        vertices,
        panel.x + 12.0,
        y,
        panel.w - 24.0,
        |v, x, cy| push_circle(v, x, cy, 7.0, [1.0, 0.28, 0.58, 1.0], 18),
        "Tu ubicación",
        place,
    );
    let note = Rect::new(panel.x + 14.0, panel.y + 480.0, panel.w - 28.0, 92.0);
    push_rounded_rect(vertices, note, 14.0, [0.0, 0.0, 0.0, 0.30]);
    push_rounded_border(vertices, note, 14.0, 1.0, with_alpha(tone.rim_soft, 0.7));
    push_label(
        font,
        vertices,
        "Borde transitable",
        note.x + 14.0,
        note.y + 12.0,
        15.5,
        tone.title,
    );
    push_label(
        font,
        vertices,
        "La línea marca el límite",
        note.x + 14.0,
        note.y + 40.0,
        13.5,
        tone.soft,
    );
    push_label(
        font,
        vertices,
        "del recorrido a pie.",
        note.x + 14.0,
        note.y + 60.0,
        13.5,
        tone.soft,
    );
    let controls = Rect::new(
        panel.x + 14.0,
        panel.y + panel.h - 96.0,
        panel.w - 28.0,
        82.0,
    );
    push_rounded_rect(vertices, controls, 14.0, [0.0, 0.0, 0.0, 0.30]);
    push_rounded_border(
        vertices,
        controls,
        14.0,
        1.0,
        with_alpha(tone.rim_soft, 0.7),
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 26.0,
        "F",
        "Usar el acceso cercano",
        14.0,
        tone.ink,
    );
    map_kit::key_caption(
        font,
        vertices,
        controls.x + 14.0,
        controls.y + 58.0,
        "ESC",
        "Cerrar el mapa",
        14.0,
        tone.ink,
    );
}

/// Right panel of the village's full map: its buildings, nearest to you first
/// lit when you are at the door.
pub(super) fn draw_village_places(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    panel: Rect,
    player: Vec3,
) {
    let place = Place::Village;
    map_kit::side_panel(vertices, panel, place);
    map_kit::side_heading(
        font,
        vertices,
        panel,
        place,
        Icon::House,
        "Lugares",
        "A poca distancia de ti",
    );
    let layout = super::super::village::layout();
    let count = layout.footprints.len();
    let badge = Rect::new(panel.x + panel.w - 62.0, panel.y + 30.0, 40.0, 24.0);
    map_kit::pill(
        vertices,
        badge,
        [0.10, 0.07, 0.04, 0.96],
        [0.86, 0.66, 0.34, 0.9],
    );
    push_label_centered(
        font,
        vertices,
        &count.to_string(),
        badge.x + badge.w * 0.5,
        badge.y + 3.5,
        15.0,
        [1.0, 0.90, 0.65, 1.0],
    );
    let row_h = ((panel.h - 190.0) / count.max(1) as f32).clamp(58.0, 90.0);
    for (i, house) in layout.footprints.iter().enumerate() {
        let d = super::super::village::building_distance(player, i);
        let near = d <= super::super::village::BUILDING_INTERACT_RADIUS;
        let row = Rect::new(
            panel.x + 12.0,
            panel.y + 94.0 + i as f32 * row_h,
            panel.w - 24.0,
            row_h - 8.0,
        );
        if row.y + row.h > panel.y + panel.h - 64.0 {
            break;
        }
        let note = if near {
            "Cerca · pulsa F".to_owned()
        } else {
            format!("{d:.0} m")
        };
        map_kit::place_row(
            font,
            vertices,
            row,
            place,
            village_icon(house.name.as_str()),
            house.name.as_str(),
            &note,
            near,
        );
    }
    let entry = Rect::new(
        panel.x + 14.0,
        panel.y + panel.h - 56.0,
        panel.w - 28.0,
        42.0,
    );
    push_rounded_rect(vertices, entry, 14.0, [0.0, 0.0, 0.0, 0.30]);
    push_rounded_border(
        vertices,
        entry,
        14.0,
        1.0,
        with_alpha(map_kit::tone(place).rim_soft, 0.7),
    );
    map_kit::key_caption(
        font,
        vertices,
        entry.x + 14.0,
        entry.y + 21.0,
        "F",
        "Portón del bosque",
        14.0,
        map_kit::tone(place).ink,
    );
}

// ------------------------------------------------------------------ baths
/// The hot springs from above (north up: the prototype's -z): the valley, its sandy paths, the
/// turquoise pools, the houses and the gate; their names as landmarks.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_baths(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    yaw: f32,
    expanded: bool,
    picture: Option<[f32; 4]>,
) {
    let place = Place::Baths;
    let p = super::super::baths::plan();
    if let Some(picture) = picture {
        // The valley as it is, seen from above.
        push_rect(vertices, map, [0.05, 0.10, 0.07, 1.0]);
        super::push_map_picture(vertices, map, bounds, picture, [1.0, 1.0, 1.0, 1.0]);
        map_kit::vignette(vertices, map, 0.22);
        for h in &p.hotspots {
            map_kit::landmark(
                font,
                vertices,
                map,
                bounds,
                (h.x, -h.z),
                super::super::baths::icon(&h.icon),
                &h.label,
                place,
                expanded,
                h.z > 0.0,
            );
        }
        map_kit::player_pin(
            vertices,
            map,
            bounds,
            Vec3::new(player.x, player.y, -player.z),
            std::f32::consts::PI - yaw,
            expanded,
        );
        map_kit::compass(font, vertices, map, place, expanded);
        map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
        return;
    }
    let scale = map.w / (bounds.max_x - bounds.min_x).max(0.001);
    let at = |x: f32, z: f32| map_point(x, -z, bounds, map);
    persona_gradient(
        vertices,
        map,
        16.0,
        [0.10, 0.20, 0.13, 0.99],
        [0.06, 0.13, 0.09, 0.99],
    );
    // The valley's floor inside its walls.
    let floor: Vec<[f32; 2]> = (0..48)
        .map(|k| {
            let a = k as f32 / 48.0 * std::f32::consts::TAU;
            let r = p.half - 2.0;
            let (x, y) = at(a.cos() * r / 0.86, a.sin() * r);
            [x, y]
        })
        .collect();
    push_polygon(vertices, &floor, [0.20, 0.36, 0.20, 0.96]);
    // Paths: a dark edge, then sand.
    for (grow, color) in [
        (0.5, [0.24, 0.20, 0.12, 0.95]),
        (0.0, [0.74, 0.62, 0.40, 0.98]),
    ] {
        for path in &p.paths {
            for pair in path.points.windows(2) {
                push_map_segment(
                    vertices,
                    map,
                    bounds,
                    pair[0][0],
                    -pair[0][1],
                    pair[1][0],
                    -pair[1][1],
                    (path.width + grow) * scale,
                    color,
                );
            }
        }
    }
    // Pools: a stone rim, then the water.
    for pool in &p.pools {
        for (grow, color) in [
            (1.12, [0.42, 0.40, 0.36, 0.95]),
            (1.0, [0.30, 0.78, 0.76, 0.98]),
            (0.55, [0.16, 0.56, 0.62, 0.98]),
        ] {
            let pts: Vec<[f32; 2]> = (0..40)
                .map(|k| {
                    let a = k as f32 / 40.0 * std::f32::consts::TAU;
                    let (x, z) = pool.shore(a);
                    let (x, z) = (pool.x + (x - pool.x) * grow, pool.z + (z - pool.z) * grow);
                    let (sx, sy) = at(x, z);
                    [sx, sy]
                })
                .collect();
            push_polygon(vertices, &pts, color);
        }
    }
    // The houses.
    for b in &p.buildings {
        let (x0, y0) = at(b.x - b.w / 2.0, b.z - b.d / 2.0);
        let (x1, y1) = at(b.x + b.w / 2.0, b.z + b.d / 2.0);
        let r = Rect::new(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs());
        push_rect(
            vertices,
            Rect::new(r.x + 2.0, r.y + 3.0, r.w, r.h),
            [0.0, 0.0, 0.0, 0.35],
        );
        push_rect(vertices, r, [0.16, 0.38, 0.36, 0.98]);
        push_border(vertices, r, 1.5, [0.70, 0.86, 0.80, 0.9]);
    }
    map_kit::vignette(vertices, map, 0.35);
    for h in &p.hotspots {
        map_kit::landmark(
            font,
            vertices,
            map,
            bounds,
            (h.x, -h.z),
            super::super::baths::icon(&h.icon),
            &h.label,
            place,
            expanded,
            h.z > 0.0,
        );
    }
    map_kit::player_pin(
        vertices,
        map,
        bounds,
        Vec3::new(player.x, player.y, -player.z),
        std::f32::consts::PI - yaw,
        expanded,
    );
    map_kit::compass(font, vertices, map, place, expanded);
    map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
}
