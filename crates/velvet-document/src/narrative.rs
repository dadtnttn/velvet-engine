//! Simplified narrative block editor → Velvet Script scenes.
//!
//! Writers edit an ordered list of blocks; we emit `scene { … }` source and can
//! re-parse a subset of common story constructs back into blocks.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from narrative block editing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NarrativeError {
    /// Unknown block / parse failure.
    #[error("{0}")]
    Message(String),
}

/// One narrative block in the simplified editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NarrativeBlock {
    /// Background image.
    Background {
        /// Asset path.
        path: String,
    },
    /// Music track.
    Music {
        /// Path.
        path: String,
        /// Optional fade-in seconds.
        fade_in: Option<f64>,
    },
    /// Show character / expression / placement.
    Show {
        /// Character id.
        character: String,
        /// Expression tag.
        expression: Option<String>,
        /// Placement (left/right/center).
        at: Option<String>,
    },
    /// Hide character.
    Hide {
        /// Character id.
        character: String,
    },
    /// Spoken line.
    Dialogue {
        /// Speaker id (None = narrator).
        speaker: Option<String>,
        /// Line text.
        text: String,
    },
    /// Narration / monologue (no speaker).
    Narration {
        /// Text.
        text: String,
    },
    /// Thought bubble style (emitted as speaker line with prefix).
    Thought {
        /// Character.
        speaker: String,
        /// Text.
        text: String,
    },
    /// Branching choice.
    Decision {
        /// Arms.
        options: Vec<DecisionArm>,
    },
    /// Set variable.
    SetVar {
        /// Name.
        name: String,
        /// Value literal text.
        value: String,
    },
    /// Conditional jump (simple).
    Condition {
        /// Variable name tested for truthiness.
        cond: String,
        /// Jump if true.
        then_jump: String,
        /// Jump if false.
        else_jump: Option<String>,
    },
    /// Jump to scene/label.
    Jump {
        /// Target.
        target: String,
    },
    /// Call scene.
    Call {
        /// Target.
        target: String,
    },
    /// Ending.
    Ending {
        /// Optional ending id.
        id: Option<String>,
    },
    /// Freeform advanced lines preserved as raw script.
    Advanced {
        /// Raw body.
        body: String,
    },
    /// Comment only.
    Comment {
        /// Text without //.
        text: String,
    },
}

/// One choice arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionArm {
    /// Display text.
    pub text: String,
    /// Nested blocks (typically setvar + jump).
    pub body: Vec<NarrativeBlock>,
}

/// A full scene in the block editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeScene {
    /// Scene name.
    pub name: String,
    /// Ordered blocks.
    pub blocks: Vec<NarrativeBlock>,
}

/// Multi-scene narrative project (simplified view).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NarrativeDocument {
    /// Scenes in order.
    pub scenes: Vec<NarrativeScene>,
    /// Leading preamble (characters/state) preserved raw when possible.
    pub preamble: String,
}

impl NarrativeDocument {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an empty scene.
    pub fn add_scene(&mut self, name: impl Into<String>) {
        self.scenes.push(NarrativeScene {
            name: name.into(),
            blocks: Vec::new(),
        });
    }

    /// Find scene mutably.
    pub fn scene_mut(&mut self, name: &str) -> Option<&mut NarrativeScene> {
        self.scenes.iter_mut().find(|s| s.name == name)
    }

    /// Append a dialogue line to a scene.
    pub fn push_dialogue(
        &mut self,
        scene: &str,
        speaker: Option<&str>,
        text: impl Into<String>,
    ) -> Result<(), NarrativeError> {
        let sc = self
            .scene_mut(scene)
            .ok_or_else(|| NarrativeError::Message(format!("unknown scene {scene}")))?;
        sc.blocks.push(NarrativeBlock::Dialogue {
            speaker: speaker.map(str::to_string),
            text: text.into(),
        });
        Ok(())
    }

    /// Append a decision with two arms that jump to targets.
    pub fn push_binary_decision(
        &mut self,
        scene: &str,
        a_text: &str,
        a_jump: &str,
        b_text: &str,
        b_jump: &str,
    ) -> Result<(), NarrativeError> {
        let sc = self
            .scene_mut(scene)
            .ok_or_else(|| NarrativeError::Message(format!("unknown scene {scene}")))?;
        sc.blocks.push(NarrativeBlock::Decision {
            options: vec![
                DecisionArm {
                    text: a_text.into(),
                    body: vec![NarrativeBlock::Jump {
                        target: a_jump.into(),
                    }],
                },
                DecisionArm {
                    text: b_text.into(),
                    body: vec![NarrativeBlock::Jump {
                        target: b_jump.into(),
                    }],
                },
            ],
        });
        Ok(())
    }

    /// Emit Velvet Script source.
    pub fn to_source(&self) -> String {
        let mut out = String::new();
        if !self.preamble.is_empty() {
            out.push_str(self.preamble.trim_end());
            out.push_str("\n\n");
        }
        for scene in &self.scenes {
            out.push_str(&format!("scene {} {{\n", scene.name));
            for b in &scene.blocks {
                out.push_str(&emit_block(b, 1));
            }
            out.push_str("}\n\n");
        }
        out
    }

    /// Parse a simplified subset of story `.vel` into blocks.
    pub fn from_source(source: &str) -> Result<Self, NarrativeError> {
        parse_narrative(source)
    }

    /// Validate jumps / decision targets exist as scenes.
    pub fn validate(&self) -> Vec<String> {
        let names: std::collections::HashSet<_> =
            self.scenes.iter().map(|s| s.name.as_str()).collect();
        let mut issues = Vec::new();
        for scene in &self.scenes {
            collect_jump_issues(&scene.blocks, &names, &mut issues, &scene.name);
        }
        issues
    }
}

fn collect_jump_issues(
    blocks: &[NarrativeBlock],
    names: &std::collections::HashSet<&str>,
    issues: &mut Vec<String>,
    scene: &str,
) {
    for b in blocks {
        match b {
            NarrativeBlock::Jump { target } | NarrativeBlock::Call { target } => {
                if !names.contains(target.as_str()) {
                    issues.push(format!(
                        "scene `{scene}`: jump/call to missing scene `{target}`"
                    ));
                }
            }
            NarrativeBlock::Condition {
                then_jump,
                else_jump,
                ..
            } => {
                if !names.contains(then_jump.as_str()) {
                    issues.push(format!(
                        "scene `{scene}`: condition then_jump missing `{then_jump}`"
                    ));
                }
                if let Some(e) = else_jump {
                    if !names.contains(e.as_str()) {
                        issues.push(format!(
                            "scene `{scene}`: condition else_jump missing `{e}`"
                        ));
                    }
                }
            }
            NarrativeBlock::Decision { options } => {
                for arm in options {
                    collect_jump_issues(&arm.body, names, issues, scene);
                }
            }
            NarrativeBlock::Ending { .. }
            | NarrativeBlock::Background { .. }
            | NarrativeBlock::Music { .. }
            | NarrativeBlock::Show { .. }
            | NarrativeBlock::Hide { .. }
            | NarrativeBlock::Dialogue { .. }
            | NarrativeBlock::Narration { .. }
            | NarrativeBlock::Thought { .. }
            | NarrativeBlock::SetVar { .. }
            | NarrativeBlock::Advanced { .. }
            | NarrativeBlock::Comment { .. } => {}
        }
    }
}

fn indent(n: usize) -> String {
    "    ".repeat(n)
}

fn emit_block(b: &NarrativeBlock, depth: usize) -> String {
    let ind = indent(depth);
    match b {
        NarrativeBlock::Background { path } => format!("{ind}background \"{path}\"\n"),
        NarrativeBlock::Music { path, fade_in } => {
            if let Some(f) = fade_in {
                format!("{ind}music \"{path}\" fade_in {f}\n")
            } else {
                format!("{ind}music \"{path}\"\n")
            }
        }
        NarrativeBlock::Show {
            character,
            expression,
            at,
        } => {
            let mut s = format!("{ind}show {character}");
            if let Some(e) = expression {
                s.push('.');
                s.push_str(e);
            }
            if let Some(a) = at {
                s.push_str(" at ");
                s.push_str(a);
            }
            s.push('\n');
            s
        }
        NarrativeBlock::Hide { character } => format!("{ind}hide {character}\n"),
        NarrativeBlock::Dialogue { speaker, text } => {
            if let Some(sp) = speaker {
                format!("{ind}{sp} \"{}\"\n", escape(text))
            } else {
                format!("{ind}\"{}\"\n", escape(text))
            }
        }
        NarrativeBlock::Narration { text } => format!("{ind}\"{}\"\n", escape(text)),
        NarrativeBlock::Thought { speaker, text } => {
            format!("{ind}{speaker} \"({})\"\n", escape(text))
        }
        NarrativeBlock::Decision { options } => {
            let mut s = format!("{ind}choice {{\n");
            for arm in options {
                s.push_str(&format!("{ind}    \"{}\" {{\n", escape(&arm.text)));
                for bb in &arm.body {
                    s.push_str(&emit_block(bb, depth + 2));
                }
                s.push_str(&format!("{ind}    }}\n"));
            }
            s.push_str(&format!("{ind}}}\n"));
            s
        }
        NarrativeBlock::SetVar { name, value } => format!("{ind}{name} = {value}\n"),
        NarrativeBlock::Condition {
            cond,
            then_jump,
            else_jump,
        } => {
            let mut s = format!("{ind}if {cond} {{\n{ind}    jump {then_jump}\n{ind}}}");
            if let Some(e) = else_jump {
                s.push_str(&format!(" else {{\n{ind}    jump {e}\n{ind}}}"));
            }
            s.push('\n');
            s
        }
        NarrativeBlock::Jump { target } => format!("{ind}jump {target}\n"),
        NarrativeBlock::Call { target } => format!("{ind}call {target}\n"),
        NarrativeBlock::Ending { id } => match id {
            Some(i) => format!("{ind}end \"{}\"\n", escape(i)),
            None => format!("{ind}end\n"),
        },
        NarrativeBlock::Advanced { body } => {
            let mut s = String::new();
            for line in body.lines() {
                s.push_str(&ind);
                s.push_str(line);
                s.push('\n');
            }
            s
        }
        NarrativeBlock::Comment { text } => format!("{ind}// {text}\n"),
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Very small line-oriented parser for common story scene forms.
///
/// Choice blocks use brace-depth tracking so nested arm bodies (`"label" { … }`)
/// do not close the outer `choice { … }` on the first arm's `}`.
fn parse_narrative(source: &str) -> Result<NarrativeDocument, NarrativeError> {
    let mut doc = NarrativeDocument::new();
    let mut preamble = String::new();
    let mut current: Option<NarrativeScene> = None;
    // While inside a choice: arms collected, open arm, brace depth (choice open = 1).
    let mut choice: Option<ChoiceParseState> = None;

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim();
        if line.is_empty() {
            // Preserve blank lines only in preamble (outside scenes).
            if current.is_none() && choice.is_none() {
                preamble.push('\n');
            }
            i += 1;
            continue;
        }

        // --- New scene header (always closes prior open scene) ---
        if line.starts_with("scene ") && line.contains('{') && choice.is_none() {
            if let Some(sc) = current.take() {
                doc.scenes.push(sc);
            }
            let name = line
                .trim_start_matches("scene ")
                .trim()
                .trim_end_matches('{')
                .trim()
                .to_string();
            current = Some(NarrativeScene {
                name,
                blocks: Vec::new(),
            });
            i += 1;
            continue;
        }

        // --- Inside multi-arm choice (brace depth) ---
        if let Some(st) = choice.as_mut() {
            let opens = line.chars().filter(|&c| c == '{').count();
            let closes = line.chars().filter(|&c| c == '}').count();

            // New arm: "text" {  (optionally with body on same line — rare)
            if line.starts_with('"') && line.contains('{') {
                if let Some(end) = line[1..].find('"') {
                    let text = line[1..1 + end].to_string();
                    if let Some(arm) = st.cur_arm.take() {
                        st.arms.push(arm);
                    }
                    st.cur_arm = Some(DecisionArm {
                        text,
                        body: Vec::new(),
                    });
                }
                st.depth = st.depth.saturating_add(opens).saturating_sub(closes);
                i += 1;
                continue;
            }

            // Pure closing brace(s): arm and/or choice
            if line.chars().all(|c| c == '}' || c.is_whitespace()) {
                st.depth = st.depth.saturating_sub(closes);
                if closes >= 1 {
                    // First `}` closes the current arm body.
                    if let Some(arm) = st.cur_arm.take() {
                        st.arms.push(arm);
                    }
                }
                if st.depth == 0 {
                    // Choice fully closed.
                    let finished = choice.take().unwrap();
                    if let Some(sc) = current.as_mut() {
                        sc.blocks.push(NarrativeBlock::Decision {
                            options: finished.arms,
                        });
                    }
                }
                i += 1;
                continue;
            }

            // Body line of current arm (may also adjust depth for rare inline braces).
            st.depth = st.depth.saturating_add(opens).saturating_sub(closes);
            if let Some(arm) = st.cur_arm.as_mut() {
                if let Some(b) = parse_simple_line(line) {
                    arm.body.push(b);
                } else if !line.is_empty() && line != "{" {
                    arm.body.push(NarrativeBlock::Advanced {
                        body: line.to_string(),
                    });
                }
            }
            if st.depth == 0 {
                let finished = choice.take().unwrap();
                if let Some(sc) = current.as_mut() {
                    sc.blocks.push(NarrativeBlock::Decision {
                        options: finished.arms,
                    });
                }
            }
            i += 1;
            continue;
        }

        // --- Outside choice ---
        if line == "}" {
            if let Some(sc) = current.take() {
                doc.scenes.push(sc);
            } else {
                // Closing brace of a preamble construct (e.g. multi-line `state { … }`).
                preamble.push_str(raw);
                preamble.push('\n');
            }
            i += 1;
            continue;
        }

        if current.is_none() {
            preamble.push_str(raw);
            preamble.push('\n');
            i += 1;
            continue;
        }

        let sc = current.as_mut().unwrap();

        if line.starts_with("choice") {
            let opens = line.chars().filter(|&c| c == '{').count();
            choice = Some(ChoiceParseState {
                arms: Vec::new(),
                cur_arm: None,
                depth: opens.max(1),
            });
            i += 1;
            continue;
        }

        if let Some(b) = parse_simple_line(line) {
            sc.blocks.push(b);
        } else if line.starts_with("//") {
            sc.blocks.push(NarrativeBlock::Comment {
                text: line.trim_start_matches('/').trim().to_string(),
            });
        } else {
            sc.blocks.push(NarrativeBlock::Advanced {
                body: line.to_string(),
            });
        }
        i += 1;
    }
    if let Some(st) = choice.take() {
        if let Some(sc) = current.as_mut() {
            let mut arms = st.arms;
            if let Some(arm) = st.cur_arm {
                arms.push(arm);
            }
            sc.blocks.push(NarrativeBlock::Decision { options: arms });
        }
    }
    if let Some(sc) = current.take() {
        doc.scenes.push(sc);
    }
    doc.preamble = preamble;
    Ok(doc)
}

struct ChoiceParseState {
    arms: Vec<DecisionArm>,
    cur_arm: Option<DecisionArm>,
    /// Brace depth: 1 after `choice {`, +1 for each arm `{`, −1 for each `}`.
    depth: usize,
}

fn parse_simple_line(line: &str) -> Option<NarrativeBlock> {
    if let Some(rest) = line.strip_prefix("background ") {
        let path = rest.trim().trim_matches('"').to_string();
        return Some(NarrativeBlock::Background { path });
    }
    if let Some(rest) = line.strip_prefix("music ") {
        let parts: Vec<_> = rest.split_whitespace().collect();
        let path = parts.first()?.trim_matches('"').to_string();
        let fade_in = parts
            .windows(2)
            .find(|w| w[0] == "fade_in")
            .and_then(|w| w[1].parse().ok());
        return Some(NarrativeBlock::Music { path, fade_in });
    }
    if let Some(rest) = line.strip_prefix("jump ") {
        return Some(NarrativeBlock::Jump {
            target: rest.trim().to_string(),
        });
    }
    if let Some(rest) = line.strip_prefix("call ") {
        return Some(NarrativeBlock::Call {
            target: rest.trim().to_string(),
        });
    }
    if line == "end" {
        return Some(NarrativeBlock::Ending { id: None });
    }
    if let Some(rest) = line.strip_prefix("end ") {
        return Some(NarrativeBlock::Ending {
            id: Some(rest.trim().trim_matches('"').to_string()),
        });
    }
    if let Some(rest) = line.strip_prefix("hide ") {
        return Some(NarrativeBlock::Hide {
            character: rest.trim().to_string(),
        });
    }
    if let Some(rest) = line.strip_prefix("show ") {
        // show aria.neutral at right
        let mut character = rest.to_string();
        let mut expression = None;
        let mut at = None;
        if let Some((left, right)) = rest.split_once(" at ") {
            character = left.to_string();
            at = Some(right.trim().to_string());
        }
        if let Some((c, e)) = character.clone().split_once('.') {
            character = c.to_string();
            expression = Some(e.to_string());
        }
        return Some(NarrativeBlock::Show {
            character,
            expression,
            at,
        });
    }
    // assignment name = value
    if let Some((name, value)) = line.split_once('=') {
        let name = name.trim();
        let value = value.trim();
        if !name.contains(' ') && !name.contains('"') {
            return Some(NarrativeBlock::SetVar {
                name: name.into(),
                value: value.into(),
            });
        }
    }
    // speaker "text" or "text"
    if let Some(q) = line.find('"') {
        let speaker_part = line[..q].trim();
        let rest = &line[q + 1..];
        if let Some(end) = rest.rfind('"') {
            let text = rest[..end].to_string();
            if speaker_part.is_empty() {
                return Some(NarrativeBlock::Narration { text });
            }
            return Some(NarrativeBlock::Dialogue {
                speaker: Some(speaker_part.to_string()),
                text,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_scene_visually_and_emit() {
        let mut doc = NarrativeDocument::new();
        doc.preamble = r#"character aria { name: "Aria" }
state { trust: int = 0 }
"#
        .into();
        doc.add_scene("apartment");
        doc.scene_mut("apartment")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Background {
                path: "bg/apartment.png".into(),
            });
        doc.push_dialogue("apartment", Some("aria"), "Pensé que no vendrías.")
            .unwrap();
        doc.push_binary_decision(
            "apartment",
            "Disculparme",
            "conversation",
            "Ignorarla",
            "hallway",
        )
        .unwrap();
        doc.add_scene("conversation");
        doc.push_dialogue("conversation", Some("aria"), "Por lo menos lo admites.")
            .unwrap();
        doc.scene_mut("conversation")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Ending {
                id: Some("good".into()),
            });
        doc.add_scene("hallway");
        doc.push_dialogue("hallway", None, "Silencio en el pasillo.")
            .unwrap();
        doc.scene_mut("hallway")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Ending {
                id: Some("cold".into()),
            });

        let src = doc.to_source();
        assert!(src.contains("scene apartment"));
        assert!(src.contains("aria \"Pensé que no vendrías.\""));
        assert!(src.contains("choice"));
        assert!(src.contains("jump conversation"));
        assert!(src.contains("jump hallway"));

        let issues = doc.validate();
        assert!(issues.is_empty(), "{issues:?}");

        // Round-trip emit → parse → emit preserves scene names and jumps
        let again = NarrativeDocument::from_source(&src).unwrap();
        assert!(again.scenes.iter().any(|s| s.name == "apartment"));
        assert!(again.scenes.iter().any(|s| s.name == "conversation"));
        let src2 = again.to_source();
        assert!(src2.contains("jump conversation"));
        assert!(src2.contains("jump hallway"));
    }

    #[test]
    fn validate_detects_missing_jump() {
        let mut doc = NarrativeDocument::new();
        doc.add_scene("a");
        doc.scene_mut("a")
            .unwrap()
            .blocks
            .push(NarrativeBlock::Jump {
                target: "missing".into(),
            });
        let issues = doc.validate();
        assert!(!issues.is_empty());
    }

    #[test]
    fn multi_arm_choice_and_preamble_roundtrip() {
        let src = r##"character hero { name: "Hero" color: "#ff4f8b" }
state {
    trust: int = 0
    chapter: int = 1
}

scene main {
    background "assets/bg/room.png"
    music "assets/music/soft.ogg" fade_in 1.0
    hero "I said I'd be here."
    show friend at right
    friend "You always say that."
    choice {
        "I keep promises." {
            trust += 1
            jump warm
        }
        "Don't make this heavy." {
            trust -= 1
            jump cool
        }
        "Ask about Kai." {
            jump rival_path
        }
    }
}

scene warm {
    friend "Then maybe tonight is different."
    jump ending_good
}

scene cool {
    "You walk home alone."
    jump ending_lonely
}

scene rival_path {
    rival "Still playing house?"
    jump ending_good
}

scene ending_good {
    "Ending: Warm Lights"
}

scene ending_lonely {
    "Ending: Cool Air"
}
"##;
        let mut doc = NarrativeDocument::from_source(src).unwrap();
        assert!(
            doc.preamble.contains("state {"),
            "preamble should keep state open: {}",
            doc.preamble
        );
        assert!(
            doc.preamble.contains('}'),
            "preamble must close state brace: {}",
            doc.preamble
        );

        let main = doc.scenes.iter().find(|s| s.name == "main").unwrap();
        let decisions: Vec<_> = main
            .blocks
            .iter()
            .filter(|b| matches!(b, NarrativeBlock::Decision { .. }))
            .collect();
        assert_eq!(
            decisions.len(),
            1,
            "expected one choice block, got {:?}",
            main.blocks
        );
        if let NarrativeBlock::Decision { options } = &main.blocks[main
            .blocks
            .iter()
            .position(|b| matches!(b, NarrativeBlock::Decision { .. }))
            .unwrap()]
        {
            assert_eq!(options.len(), 3, "expected 3 choice arms, got {options:?}");
            assert!(options.iter().any(|o| o.text.contains("promises")));
            assert!(options.iter().any(|o| o.text.contains("heavy")));
            assert!(options.iter().any(|o| o.text.contains("Kai")));
        }

        doc.push_dialogue("main", Some("hero"), "Una linea nueva del editor.")
            .unwrap();
        doc.push_binary_decision("main", "Ir a warm", "warm", "Ir a cool", "cool")
            .unwrap();

        let out = doc.to_source();
        assert!(out.contains("state {"), "emit must keep state: {out}");
        assert!(
            out.contains("chapter: int = 1"),
            "emit must keep state fields: {out}"
        );
        // Closing brace of state must appear before first scene.
        let state_pos = out.find("state {").unwrap();
        let scene_pos = out.find("scene main").unwrap();
        let state_close = out[state_pos..scene_pos].rfind('}');
        assert!(
            state_close.is_some(),
            "state block must close before scene main:\n{out}"
        );
        assert!(out.contains("jump warm"), "{out}");
        assert!(out.contains("jump cool"), "{out}");
        assert!(out.contains("jump rival_path"), "{out}");
        assert!(out.contains("Una linea nueva del editor."), "{out}");
        assert!(out.contains("Ir a warm"), "{out}");

        // Re-parse emitted source must still see 3 original arms + new decision.
        let again = NarrativeDocument::from_source(&out).unwrap();
        let main2 = again.scenes.iter().find(|s| s.name == "main").unwrap();
        let arm_count: usize = main2
            .blocks
            .iter()
            .filter_map(|b| match b {
                NarrativeBlock::Decision { options } => Some(options.len()),
                _ => None,
            })
            .sum();
        assert!(
            arm_count >= 5,
            "expected original 3 arms + binary 2, got {arm_count} in {:?}",
            main2.blocks
        );
    }
}
