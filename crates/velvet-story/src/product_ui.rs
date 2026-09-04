//! Product UI frame builder — turns [`VnSession`] state into draw-ready layout
//! (namebox, body metrics, choice labels, language menu, font attachment).

use serde::{Deserialize, Serialize};
use velvet_text::{measure_width, TextStyle};

use crate::product::VnSession;
use crate::runtime::StoryWait;

/// Resolved font attachment for a line of dialogue (multi-script).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontAttachment {
    /// Logical family key: `latin` | `sc` | `hi` | `mixed`.
    pub family: String,
    /// Relative path suggestion under project `fonts/` (may not exist on disk).
    pub path: String,
}

/// One product UI frame ready for a host renderer or log export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductUiFrame {
    /// Scene name.
    pub scene: String,
    /// Wait kind string.
    pub wait: String,
    /// Namebox text (speaker display).
    pub namebox: String,
    /// Body text (plain, markup stripped by session).
    pub body: String,
    /// Measured body width (px, product helper).
    pub body_width: f32,
    /// Measured body height (px).
    pub body_height: f32,
    /// Font attachment for this body.
    pub font: FontAttachment,
    /// Choice labels (empty when not in choice wait).
    pub choices: Vec<String>,
    /// Selected choice index in the menu.
    pub selected_choice: usize,
    /// Language codes available for the language menu.
    pub language_options: Vec<String>,
    /// Active language.
    pub language: String,
    /// Background asset path if any.
    pub background: Option<String>,
    /// Visible sprite ids (ordered by z).
    pub sprite_ids: Vec<String>,
    /// Whether say box should be drawn.
    pub say_visible: bool,
    /// Whether choice panel should be drawn.
    pub choice_visible: bool,
    /// Whether language menu should be offered.
    pub language_menu_visible: bool,
    /// Effective font scale from accessibility prefs.
    #[serde(default = "default_font_scale")]
    pub font_scale: f32,
    /// High-contrast mode from accessibility prefs.
    #[serde(default)]
    pub high_contrast: bool,
    /// Reduced-motion mode from accessibility prefs.
    #[serde(default)]
    pub reduced_motion: bool,
    /// Backlog: last history lines `Speaker: text` (max 30) for History screen.
    #[serde(default)]
    pub backlog: Vec<String>,
    /// A cinema clip is currently playing (hosts hide say UI).
    #[serde(default)]
    pub video_playing: bool,
    /// Active clip progress 0..=1 (0 when none).
    #[serde(default)]
    pub video_progress: f32,
    /// Game screens currently shown (Ren'Py-style `screen show/call`).
    #[serde(default)]
    pub game_screens: Vec<GameScreenFrame>,
}

/// One shown game screen, resolved for display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameScreenFrame {
    /// Screen id.
    pub id: String,
    /// Modal `screen call` (blocks until return).
    pub modal: bool,
    /// Call args in display form.
    #[serde(default)]
    pub args: Vec<String>,
    /// Screen-level `title` property when present.
    #[serde(default)]
    pub title: String,
    /// `text` widget contents in tree order (variables resolved).
    #[serde(default)]
    pub texts: Vec<String>,
    /// `textbutton` labels in tree order (variables resolved).
    #[serde(default)]
    pub labels: Vec<String>,
    /// Typewriter-visible prefixes of `texts` (same length as `texts`).
    #[serde(default)]
    pub revealed_texts: Vec<String>,
    /// Typewriter-visible prefixes of `labels` (same length as `labels`).
    #[serde(default)]
    pub revealed_labels: Vec<String>,
    /// True when the whole screen text is revealed.
    #[serde(default)]
    pub text_complete: bool,
    /// `zorder:` prop (paint stacking hint).
    #[serde(default)]
    pub zorder: Option<f32>,
    /// `tag:` prop (same-tag screens replace each other).
    #[serde(default)]
    pub tag: Option<String>,
    /// Enter transition name (`with` motion or transform).
    #[serde(default)]
    pub enter: Option<String>,
    /// Paint alpha 0..=1 (enter/exit fades; 1 = fully shown).
    #[serde(default = "default_screen_alpha")]
    pub alpha: f32,
}

/// Full alpha for frames serialized before fades existed.
fn default_screen_alpha() -> f32 {
    1.0
}

/// Default font scale for [`ProductUiFrame`] (old JSON without the field).
fn default_font_scale() -> f32 {
    1.0
}

/// Format one backlog line.
pub fn format_backlog_entry(speaker: &str, text: &str) -> String {
    if speaker.trim().is_empty() {
        text.to_string()
    } else {
        format!("{}: {}", speaker.trim(), text)
    }
}

/// Search backlog lines (case-insensitive substring). Returns matching indexes.
pub fn search_backlog(backlog: &[String], query: &str) -> Vec<usize> {
    let q = query.to_ascii_lowercase();
    if q.trim().is_empty() {
        return Vec::new();
    }
    backlog
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_ascii_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect()
}

/// Detect primary script class for font attachment.
pub fn detect_script_family(text: &str) -> FontAttachment {
    let mut sc = 0usize;
    let mut hi = 0usize;
    let mut latin = 0usize;
    for ch in text.chars() {
        let u = ch as u32;
        // CJK Unified + Hirana/Katakana + Hangul
        if (0x4E00..=0x9FFF).contains(&u)
            || (0x3040..=0x30FF).contains(&u)
            || (0xAC00..=0xD7AF).contains(&u)
            || (0x3400..=0x4DBF).contains(&u)
        {
            sc += 1;
        } else if (0x0900..=0x097F).contains(&u) {
            // Devanagari
            hi += 1;
        } else if ch.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    if sc > 0 && hi == 0 && latin == 0 {
        FontAttachment {
            family: "sc".into(),
            path: "fonts/NotoSansSC-Regular.otf".into(),
        }
    } else if hi > 0 && sc == 0 {
        FontAttachment {
            family: "hi".into(),
            path: "fonts/NotoSansDevanagari-Regular.ttf".into(),
        }
    } else if sc > 0 || hi > 0 {
        FontAttachment {
            family: "mixed".into(),
            path: "fonts/NotoSansSC-Regular.otf".into(),
        }
    } else {
        FontAttachment {
            family: "latin".into(),
            path: "fonts/DejaVuSans.ttf".into(),
        }
    }
}

/// Measure say body for product UI (real `velvet_text::measure_width`).
pub fn measure_say_body(text: &str, size: f32) -> (f32, f32) {
    let style = TextStyle {
        size,
        ..TextStyle::default()
    };
    let width = measure_width(text, &style);
    let height = style.size * style.line_height;
    (width, height)
}

/// Build a draw-ready frame from a live product session.
pub fn build_product_ui_frame(session: &VnSession) -> ProductUiFrame {
    let wait = match session.player().wait() {
        StoryWait::Line => "line",
        StoryWait::Choice => "choice",
        StoryWait::ScreenCall { .. } => "screen_call",
        StoryWait::Ended => "ended",
        StoryWait::Ready => "ready",
        StoryWait::Pause { .. } => "pause",
        StoryWait::Host { .. } => "host",
    };
    // Prefer typewriter-visible text so product hosts can animate cps.
    let body = if session.say.text_complete || session.say.visible_text.is_empty() {
        session.say.full_text.clone()
    } else {
        session.say.visible_text.clone()
    };
    let (body_width, body_height) = measure_say_body(&session.say.full_text, 28.0);
    let font = detect_script_family(&body);
    let choices: Vec<String> = session
        .choice
        .options
        .iter()
        .map(|o| o.text.clone())
        .collect();
    let sprite_ids: Vec<String> = session
        .presentation
        .sprites_by_z()
        .into_iter()
        .map(|s| s.id.clone())
        .collect();

    let a11y = &session.accessibility;
    let backlog: Vec<String> = session
        .history_entries()
        .iter()
        .rev()
        .take(30)
        .rev()
        .map(|e| format_backlog_entry(&e.speaker, &e.text))
        .collect();

    ProductUiFrame {
        scene: session.player().scene_name().to_string(),
        wait: wait.into(),
        namebox: session.say.namebox.clone(),
        body,
        body_width,
        body_height,
        font,
        choices,
        selected_choice: session.choice.selected,
        language_options: session.available_languages(),
        language: session.language.clone(),
        background: session.presentation.background.clone(),
        sprite_ids,
        say_visible: session.say.visible,
        choice_visible: session.choice.open,
        language_menu_visible: session.available_languages().len() > 1,
        font_scale: a11y.font_scale,
        high_contrast: a11y.high_contrast,
        reduced_motion: a11y.reduced_motion,
        backlog,
        video_playing: session.video.is_playing(),
        video_progress: session.video.progress().unwrap_or(0.0),
        game_screens: game_screen_frames(session),
    }
}

/// Typewriter prefix of one string given a shared char budget.
/// `None` (no entry yet) and `usize::MAX` (revealed) show everything.
fn reveal_prefix(text: &str, remaining: &mut Option<usize>) -> String {
    match remaining {
        None | Some(usize::MAX) => text.to_string(),
        Some(left) => {
            let take = (*left).min(text.chars().count());
            *left -= take;
            text.chars().take(take).collect()
        }
    }
}

/// Resolve shown game screens into display frames.
pub fn game_screen_frames(session: &VnSession) -> Vec<GameScreenFrame> {
    let mut frames: Vec<GameScreenFrame> = session
        .game_screens
        .iter()
        .map(|shown| game_screen_frame(session, shown))
        .collect();
    // Exiting screens paint under new ones, fading out.
    for exit in session.screen_exiting.values() {
        let mut frame = exit.frame.clone();
        frame.alpha = (1.0 - exit.clock / crate::product::SCREEN_EXIT_SECS).clamp(0.0, 1.0);
        frames.push(frame);
    }
    frames
}

/// Resolve one shown screen into a display frame.
pub fn game_screen_frame(
    session: &VnSession,
    shown: &crate::runtime::ActiveGameScreen,
) -> GameScreenFrame {
    let mut frame = GameScreenFrame {
        id: shown.id.clone(),
        modal: shown.modal,
        args: shown.args.iter().map(|v| v.display_str()).collect(),
        title: String::new(),
        texts: Vec::new(),
        labels: Vec::new(),
        revealed_texts: Vec::new(),
        revealed_labels: Vec::new(),
        text_complete: true,
        zorder: None,
        tag: None,
        enter: session.screen_enters.get(&shown.id).cloned(),
        alpha: enter_alpha(session, &shown.id),
    };
    if let Some(def) = session.game_screen_def(&shown.id) {
        frame.zorder = def.zorder;
        frame.tag = def.tag.clone();
        for prop in &def.properties {
            if prop.name == "title" {
                frame.title = session.resolve_screen_value(&prop.value);
            }
        }
        let mut visiting = vec![def.name.clone()];
        collect_screen_texts(session, &def.children, &mut visiting, 0, &mut frame);
    }
    let mut remaining = session.screen_reveal.get(&shown.id).copied();
    frame.revealed_texts = frame
        .texts
        .iter()
        .map(|t| reveal_prefix(t, &mut remaining))
        .collect();
    frame.revealed_labels = frame
        .labels
        .iter()
        .map(|t| reveal_prefix(t, &mut remaining))
        .collect();
    frame.text_complete =
        frame.revealed_texts == frame.texts && frame.revealed_labels == frame.labels;
    frame
}

/// Enter-fade alpha for a shown screen (1 without a `with` transition,
// frozen at reduced motion).
fn enter_alpha(session: &VnSession, id: &str) -> f32 {
    if !session.screen_enters.contains_key(id) || session.accessibility.reduced_motion {
        return 1.0;
    }
    let clock = session.screen_enter.get(id).copied().unwrap_or(0.0);
    (clock / crate::product::SCREEN_ENTER_SECS).clamp(0.0, 1.0)
}

fn collect_screen_texts(
    session: &VnSession,
    children: &[crate::screens::ScreenChildRt],
    visiting: &mut Vec<String>,
    depth: usize,
    frame: &mut GameScreenFrame,
) {
    if depth > 8 {
        return;
    }
    for child in children {
        match child.kind.as_str() {
            "text" => {
                if let Some(prop) = child.props.iter().find(|p| p.name == "text") {
                    frame.texts.push(session.resolve_screen_value(&prop.value));
                }
            }
            "textbutton" => {
                if let Some(prop) = child.props.iter().find(|p| p.name == "label") {
                    frame.labels.push(session.resolve_screen_value(&prop.value));
                }
            }
            "use" => {
                let target = child
                    .props
                    .first()
                    .map(|p| session.resolve_screen_value(&p.value))
                    .unwrap_or_default();
                // Cycle guard: a screen reusing an ancestor stops here.
                if !target.is_empty() && !visiting.contains(&target) {
                    if let Some(def) = session.game_screen_def(&target) {
                        visiting.push(target);
                        collect_screen_texts(session, &def.children, visiting, depth + 1, frame);
                        visiting.pop();
                    }
                }
            }
            _ => {}
        }
        collect_screen_texts(session, &child.children, visiting, depth + 1, frame);
    }
}

/// Sync a `velvet_ui::DialogueBox` from session product state (optional host bridge).
/// Kept free of hard dependency: returns fields the host can apply.
pub fn dialogue_box_fields(session: &VnSession) -> (Option<String>, String, Vec<String>, usize) {
    let frame = build_product_ui_frame(session);
    let speaker = if frame.namebox.trim().is_empty() {
        None
    } else {
        Some(frame.namebox)
    };
    (speaker, frame.body, frame.choices, frame.selected_choice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::product::VnSession;
    use crate::runtime::{StoryPlayer, StoryWait};

    #[test]
    fn ui_frame_exposes_shown_game_screens() {
        let src = r#"
screen help {
  title: "Help"
  text "Read me."
  textbutton ok action Return("ok")
}
scene main {
  screen show help
  "x"
}
"#;
        let program = load_program_from_source(src, Some("gs.vel"), "GS").unwrap();
        let session = VnSession::new(StoryPlayer::start(program));
        let frame = build_product_ui_frame(&session);
        assert_eq!(frame.game_screens.len(), 1);
        let gs = &frame.game_screens[0];
        assert_eq!(gs.id, "help");
        assert!(!gs.modal);
        assert_eq!(gs.title, "Help");
        assert_eq!(gs.texts, vec!["Read me.".to_string()]);
        assert_eq!(gs.labels, vec!["ok".to_string()]);
    }

    #[test]
    fn ui_frame_from_real_story_has_namebox_body_choices() {
        let src = r#"
character hero { name: "Hero" }
scene main {
    hero "Hello there friend."
    choice {
        "Yes path" { jump end }
        "No path" { jump end }
    }
}
scene end { "Ending: Done" }
"#;
        let program = load_program_from_source(src, Some("ui.vel"), "UI").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 20 {
            session.advance();
            g += 1;
        }
        let frame = build_product_ui_frame(&session);
        assert!(frame.say_visible);
        assert!(!frame.namebox.is_empty(), "namebox from speaker");
        assert!(
            !frame.body.is_empty() && frame.body.contains("Hello"),
            "body from real line: {:?}",
            frame.body
        );
        assert!(frame.body_width > 0.0, "measured width > 0");
        assert!(frame.body_height > 0.0);

        // Advance to choice
        session.say.reveal_all();
        session.advance();
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Choice) && g < 10 {
            session.advance();
            g += 1;
        }
        let frame2 = build_product_ui_frame(&session);
        assert!(frame2.choice_visible);
        assert!(
            frame2.choices.len() >= 2,
            "choices from real menu: {:?}",
            frame2.choices
        );
        assert!(frame2.choices.iter().any(|c| c.contains("Yes")));
    }

    #[test]
    fn cjk_measure_positive_width_and_font() {
        let zh = "你好世界";
        let hi = "नमस्ते";
        let (w_zh, h_zh) = measure_say_body(zh, 28.0);
        let (w_hi, _) = measure_say_body(hi, 28.0);
        let (w_en, _) = measure_say_body("Hi", 28.0);
        assert!(w_zh > 0.0 && h_zh > 0.0, "CJK width={w_zh}");
        assert!(w_hi > 0.0, "Hindi width={w_hi}");
        assert!(
            w_zh > w_en,
            "CJK sample should measure wider than short latin"
        );
        let f = detect_script_family(zh);
        assert_eq!(f.family, "sc");
        assert!(f.path.contains("NotoSansSC"));
        let f2 = detect_script_family(hi);
        assert_eq!(f2.family, "hi");
    }

    #[test]
    fn frame_carries_backlog_and_accessibility() {
        let src = r#"
scene main {
    luna "Hello there."
    luna "Second line here."
    "Ending: X"
}
"#;
        let program = load_program_from_source(src, Some("b.vel"), "b").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        session.accessibility.font_scale = 1.5;
        session.accessibility.high_contrast = true;
        // advance through two lines
        for _ in 0..8 {
            match session.player().wait().clone() {
                StoryWait::Line => session.advance(),
                StoryWait::Ended => break,
                _ => break,
            }
        }
        let frame = build_product_ui_frame(&session);
        assert!((frame.font_scale - 1.5).abs() < 1e-5);
        assert!(frame.high_contrast);
        assert!(!frame.backlog.is_empty());
        assert!(frame.backlog.iter().any(|l| l.contains("Hello")));
        assert!(!search_backlog(&frame.backlog, "hello").is_empty());
        assert!(search_backlog(&frame.backlog, "").is_empty());
        assert_eq!(format_backlog_entry("", "Hi"), "Hi");
        // old JSON without new fields still loads
        let old = r#"{"scene":"s","wait":"line","namebox":"N","body":"B","body_width":1.0,"body_height":1.0,"font":{"family":"latin","path":"x"},"choices":[],"selected_choice":0,"language_options":["en"],"language":"en","background":null,"sprite_ids":[],"say_visible":true,"choice_visible":false,"language_menu_visible":false}"#;
        let back: ProductUiFrame = serde_json::from_str(old).unwrap();
        assert!((back.font_scale - 1.0).abs() < 1e-5);
        assert!(back.backlog.is_empty());
    }

    #[test]
    fn language_menu_lists_es_when_tl_present() {
        let dir = tempfile::tempdir().unwrap();
        let src = r#"
character h { name: "H" }
scene main { h "Hello" choice { "Go" { jump e } } }
scene e { "Ending: X" }
"#;
        let program = load_program_from_source(src, Some("t.vel"), "t").unwrap();
        let cat = crate::extract_loc_keys(&program);
        let mut es = crate::TranslationTable::new();
        for e in &cat.entries {
            es.insert(e.key.clone(), format!("ES:{0}", e.source));
        }
        crate::write_tl_scaffold(dir.path(), &program, "es", &es).unwrap();
        let session = VnSession::new(StoryPlayer::start(program)).with_project_root(dir.path());
        let frame = build_product_ui_frame(&session);
        assert!(frame.language_menu_visible);
        assert!(frame.language_options.iter().any(|l| l == "es"));
    }
}
