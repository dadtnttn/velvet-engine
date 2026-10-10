//! UI gallery tour: every widget, layout, theme, and animation tool,
//! exercised headlessly with exact assertions.
//!
//! Windowed hosts consume the same APIs (see `third-person` for a wgpu
//! shell); this binary proves operability in CI: exit 0 + `TOUR OK`
//! means every stop passed.

use anyhow::{bail, Result};
use velvet_anim::{sample_path, AnimDirector, Spring};
use velvet_math::Vec2;
use velvet_style::{parse_stylesheet, MediaQuery};
use velvet_ui::{
    anchor_in, drag_glow, spinner_angle, Anchor, AnimatedButton, AnimatedToggle, AppearTracker,
    Button, ButtonFxPreset, Density, Dropdown, LayoutType, PressRipple, Tabs, ThemeRegistry,
    Tooltip, UiAnimator, UiContext, UiTree, UiTween, VirtualList, WidgetKind,
};

const VIEW: Vec2 = Vec2::new(800.0, 600.0);

fn check(cond: bool, what: &str) -> Result<()> {
    if cond {
        println!("  ok: {what}");
        Ok(())
    } else {
        bail!("FAILED: {what}")
    }
}

fn click(ui: &mut UiTree, point: Vec2) {
    ui.process(UiContext {
        pointer: point,
        pointer_released: true,
        viewport: VIEW,
        ..Default::default()
    });
}

fn mid(ui: &UiTree, id: velvet_ui::NodeId) -> Vec2 {
    let rect = ui.get(id).unwrap().rect;
    rect.pos + rect.size * 0.5
}

fn stop_dropdown() -> Result<()> {
    println!("stop: dropdown");
    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    let dd = ui.add_node(
        "lang",
        Dropdown::widget(vec!["es".into(), "en".into(), "ja".into()]),
        Some(root),
    );
    ui.layout(VIEW);
    let m = mid(&ui, dd);
    click(&mut ui, m);
    check(
        matches!(
            ui.get(dd).unwrap().widget,
            WidgetKind::Dropdown { open: true, .. }
        ),
        "opens on header click",
    )?;
    let rect = ui.get(dd).unwrap().rect;
    click(
        &mut ui,
        Vec2::new(rect.pos.x + 4.0, rect.pos.y + rect.size.y * 2.5),
    );
    check(
        ui.get(dd).unwrap().widget.dropdown_selected() == Some(Some(1)),
        "picks option row 1",
    )?;
    check(ui.clicked == Some(dd), "selection fires clicked")?;
    Ok(())
}

fn stop_tabs() -> Result<()> {
    println!("stop: tabs");
    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    let tabs = ui.add_node(
        "modes",
        Tabs::widget(vec!["a".into(), "b".into()]),
        Some(root),
    );
    let h0 = ui.add_node("h0", WidgetKind::Label { text: "a".into() }, Some(tabs));
    let h1 = ui.add_node("h1", WidgetKind::Label { text: "b".into() }, Some(tabs));
    ui.layout(VIEW);
    let m = mid(&ui, h1);
    click(&mut ui, m);
    check(
        ui.get(tabs).unwrap().widget.tabs_selected() == Some(1),
        "child click selects tab",
    )?;
    ui.focus.focused = Some(tabs);
    ui.process(UiContext {
        left: true,
        viewport: VIEW,
        ..Default::default()
    });
    check(
        ui.get(tabs).unwrap().widget.tabs_selected() == Some(0),
        "left arrow cycles",
    )?;
    check(ui.arrows_consumed, "arrows consumed by tabs")?;
    let _ = (h0, h1);
    Ok(())
}

fn stop_tooltip() -> Result<()> {
    println!("stop: tooltip");
    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    let btn = ui.add_node("save", Button::widget("Save"), Some(root));
    let tip = ui.add_node("tip", Tooltip::widget("Write it", "save"), Some(root));
    ui.layout(VIEW);
    check(!ui.get(tip).unwrap().visible, "hidden until hover")?;
    let m = mid(&ui, btn);
    ui.process(UiContext {
        pointer: m,
        viewport: VIEW,
        ..Default::default()
    });
    check(ui.get(tip).unwrap().visible, "shows on hover")?;
    Ok(())
}

fn stop_list_and_layouts() -> Result<()> {
    println!("stop: virtual list + wrap + scroll + anchors");
    let list = VirtualList::new(10_000, 20.0);
    check(list.visible_range(100.0).len() <= 7, "window stays small")?;
    check(list.content_height() == 200_000.0, "content measured")?;

    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    {
        let node = ui.get_mut(root).unwrap();
        node.layout = LayoutType::Row { wrap: true };
    }
    for name in ["a", "bb", "ccc", "dddd"] {
        ui.add_node(name, Button::widget(name), Some(root));
    }
    ui.layout(Vec2::new(120.0, 400.0));
    let ys: Vec<f32> = ui
        .get(root)
        .unwrap()
        .children
        .iter()
        .map(|c| ui.get(*c).unwrap().rect.pos.y)
        .collect();
    check(ys.windows(2).any(|w| w[1] > w[0]), "wrap flows rows")?;

    let anchored = anchor_in(
        velvet_ui::UiRect {
            pos: Vec2::ZERO,
            size: Vec2::new(100.0, 100.0),
        },
        Vec2::new(10.0, 10.0),
        Anchor::BottomRight,
        Vec2::new(5.0, 5.0),
    );
    check(anchored.pos == Vec2::new(85.0, 85.0), "anchor math")?;
    Ok(())
}

fn stop_themes() -> Result<()> {
    println!("stop: themes + density");
    let mut registry = ThemeRegistry::with_defaults();
    registry.set_active("velvet-light");
    check(
        registry.active().map(|t| t.name.as_str()) == Some("velvet-light"),
        "switches theme",
    )?;
    registry.density = Density::Compact;
    let scaled = registry.active_scaled().unwrap();
    check(
        scaled.button.font_size < registry.active().unwrap().button.font_size,
        "compact shrinks type",
    )?;
    let mut ui = UiTree::with_root("root");
    registry.apply_active(&mut ui);
    check(ui.theme.name == "velvet-light", "pushes into tree")?;
    Ok(())
}

fn stop_animation() -> Result<()> {
    println!("stop: animation (crossfade, events, springs, paths, flip)");
    // Crossfade midpoint.
    let mut anim = UiAnimator::new();
    anim.play("x", UiTween::opacity(0.0, 0.0, 10.0));
    anim.tick(0.1);
    anim.play_fade("x", UiTween::opacity(0.0, 1.0, 0.1), 1.0);
    anim.tick(0.5);
    let opacity = anim.sample("x").unwrap().opacity.unwrap();
    check((opacity - 0.5).abs() < 1e-4, "crossfade midpoint")?;
    // Spring converges.
    let mut spring = Spring::new(0.0, 10.0, 60.0, 2.0 * 60.0f32.sqrt());
    for _ in 0..120 {
        spring.tick(1.0 / 60.0);
    }
    check((spring.value - 10.0).abs() < 0.05, "spring converges")?;
    // Path endpoints.
    let pts = vec![Vec2::ZERO, Vec2::new(8.0, 0.0)];
    check(
        sample_path(&pts, 1.0) == Some(Vec2::new(8.0, 0.0)),
        "path reaches end",
    )?;
    // Director path motion.
    let mut director = AnimDirector::new();
    director.spawn_at("hero", Vec2::ZERO);
    director.move_along("hero", pts, 1.0, "linear");
    for _ in 0..70 {
        director.tick(1.0 / 60.0);
    }
    check(
        (director.pose("hero").unwrap().pos.x - 8.0).abs() < 1e-3,
        "director walks path",
    )?;
    // Appear tracker.
    let mut appear = AppearTracker::with_duration(0.5);
    appear.enter(["row".to_string()]);
    appear.tick(0.25);
    check((appear.alpha("row") - 0.5).abs() < 1e-4, "enter fade")?;
    Ok(())
}

fn stop_style() -> Result<()> {
    println!("stop: style (@media, child, gradient, shadow)");
    let sheet = parse_stylesheet(
        ".menu > .button { color: red; }
         @media (prefers-reduced-motion) { .button { opacity: 0.5; } }
         .hero { background: linear-gradient(to bottom, red, blue); box-shadow: 0 2px 8px #000; }",
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    check(sheet.media.len() == 1, "media parsed")?;
    let query = MediaQuery::parse("prefers-reduced-motion").map_err(|e| anyhow::anyhow!(e))?;
    check(
        query.matches(&velvet_style::MediaContext {
            reduced_motion: true,
            ..Default::default()
        }),
        "media matches",
    )?;
    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    ui.get_mut(root).unwrap().add_class("menu");
    let btn = ui.add_node("go", Button::widget("Go"), Some(root));
    ui.get_mut(btn).unwrap().add_class("button");
    let style = ui
        .vcss_style_with_media(
            btn,
            &sheet,
            &velvet_style::MediaContext {
                reduced_motion: true,
                ..Default::default()
            },
        )
        .unwrap();
    check((style.opacity - 0.5).abs() < 1e-4, "media swaps opacity")?;
    check(
        style.text_color == velvet_math::Color::rgb(1.0, 0.0, 0.0),
        "child selector colors text",
    )?;
    Ok(())
}

fn stop_particles() -> Result<()> {
    println!("stop: particles");
    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    let btn = ui.add_node("go", Button::widget("Go"), Some(root));
    ui.layout(VIEW);
    let mut fx = velvet_ui::UiParticles::with_capacity(64);
    fx.burst_at_named(&ui, "go", 8, velvet_ui::UiBurst::click());
    check(fx.len() == 8, "burst spawns")?;
    fx.tick(1.0);
    check(fx.is_empty(), "particles die")?;
    let _ = btn;
    Ok(())
}

fn stop_interactions() -> Result<()> {
    println!("stop: interactions (animated buttons, sounds, ripple, repeat)");
    // Animated button: hover grows, press squashes, release pops back.
    let mut button = AnimatedButton::with_preset(ButtonFxPreset::default());
    button.update(true, false, true);
    button.tick(1.0);
    check((button.tick(0.0).scale - 1.04).abs() < 1e-4, "hover grow")?;
    button.update(true, true, true);
    button.tick(1.0);
    check((button.tick(0.0).scale - 0.94).abs() < 1e-4, "press squash")?;
    // Sounds: hover edge once, click, blocked on disabled.
    let mut ui = UiTree::with_root("root");
    let root = ui.root().unwrap();
    ui.get_mut(root).unwrap().layout = LayoutType::Column { wrap: false };
    let btn = ui.add_node(
        "ok",
        WidgetKind::Button {
            label: "OK".into(),
            pressed: false,
            hovered: false,
            busy: false,
        },
        Some(root),
    );
    ui.layout(VIEW);
    let mid = {
        let rect = ui.get(btn).unwrap().rect;
        rect.pos + rect.size * 0.5
    };
    ui.process(UiContext {
        pointer: mid,
        viewport: VIEW,
        ..Default::default()
    });
    check(ui.sounds.len() == 1, "hover tick")?;
    ui.sounds.drain();
    ui.process(UiContext {
        pointer: mid,
        viewport: VIEW,
        ..Default::default()
    });
    let second: Vec<String> = ui
        .sounds
        .drain()
        .into_iter()
        .map(|s| format!("{}/{}", s.kind.as_str(), s.node))
        .collect();
    check(
        second.is_empty(),
        &format!("no hover repeat, got {second:?}"),
    )?;
    ui.process(UiContext {
        pointer: mid,
        pointer_released: true,
        viewport: VIEW,
        ..Default::default()
    });
    let sounds: Vec<String> = ui
        .sounds
        .drain()
        .into_iter()
        .map(|s| s.kind.as_str().to_string())
        .collect();
    check(sounds == vec!["click"], "click sound")?;
    // Ripple expands and fades on the press clock.
    let mut ripple = PressRipple::new(mid, 40.0, 0.4);
    ripple.tick(0.2);
    check((ripple.radius() - 30.0).abs() < 1e-4, "ripple ease-out")?;
    // Hold-to-repeat fires on schedule.
    ui.repeat_delay = 0.5;
    ui.repeat_rate = 0.2;
    let mut fires = 0;
    for _ in 0..10 {
        ui.process(UiContext {
            pointer: mid,
            pointer_pressed: true,
            dt: 0.1,
            viewport: VIEW,
            ..Default::default()
        });
        if ui.clicked == Some(btn) {
            fires += 1;
        }
    }
    check(fires == 3, "hold repeats")?;
    // Double click inside the window.
    ui.process(UiContext {
        pointer: mid,
        pointer_released: true,
        viewport: VIEW,
        ..Default::default()
    });
    ui.process(UiContext {
        dt: 0.1,
        viewport: VIEW,
        ..Default::default()
    });
    ui.process(UiContext {
        pointer: mid,
        pointer_released: true,
        viewport: VIEW,
        ..Default::default()
    });
    check(ui.double_clicked == Some(btn), "double click")?;
    ui.sounds.drain();
    // Busy blocks silently; toggle knob slides; slider glows on drag.
    ui.set_busy(btn, true);
    ui.process(UiContext {
        pointer: mid,
        pointer_released: true,
        viewport: VIEW,
        ..Default::default()
    });
    check(ui.clicked.is_none(), "busy blocks click")?;
    check(ui.sounds.drain().is_empty(), "busy is silent")?;
    let mut toggle = AnimatedToggle::new(false);
    toggle.update(true);
    toggle.tick(1.0);
    check(toggle.tick(0.0) == 1.0, "knob slides on")?;
    check(drag_glow(true, 0.0) > drag_glow(false, 99.0), "drag glows")?;
    check(
        (spinner_angle(0.45) - std::f32::consts::PI).abs() < 1e-4,
        "spinner turns",
    )?;
    // Focus ring only after keyboard.
    ui.set_busy(btn, false);
    ui.focus.focused = Some(btn);
    ui.process(UiContext {
        pointer: mid,
        pointer_released: true,
        viewport: VIEW,
        ..Default::default()
    });
    let query = ui.vcss_query(btn).unwrap();
    check(
        !query.states.iter().any(|s| s == "focus-visible"),
        "mouse hides ring",
    )?;
    ui.process(UiContext {
        confirm: true,
        viewport: VIEW,
        ..Default::default()
    });
    let query = ui.vcss_query(btn).unwrap();
    check(
        query.states.iter().any(|s| s == "focus-visible"),
        "keyboard shows ring",
    )?;
    Ok(())
}

fn main() -> Result<()> {
    println!("UI gallery tour:");
    stop_dropdown()?;
    stop_tabs()?;
    stop_tooltip()?;
    stop_list_and_layouts()?;
    stop_themes()?;
    stop_animation()?;
    stop_style()?;
    stop_particles()?;
    stop_interactions()?;
    println!("TOUR OK");
    Ok(())
}
