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
