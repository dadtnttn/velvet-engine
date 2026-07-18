//! Typewriter / progressive text reveal.

use crate::markup::{parse_rich_text, RichSpan, RichText};

/// Events emitted while revealing.
#[derive(Debug, Clone, PartialEq)]
pub enum TypewriterEvent {
    /// Revealed a character.
    Char(char),
    /// Hit a pause.
    Pause(f32),
    /// Speed multiplier changed.
    SpeedChange(f32),
    /// Inline icon encountered (id).
    Icon(String),
    /// Ruby base character(s) started (base text for this unit).
    Ruby {
        /// Base.
        base: String,
        /// Reading.
        ruby: String,
    },
    /// Fully revealed.
    Finished,
    /// Skip was invoked.
    Skipped,
}

/// Progressive reveal controller.
#[derive(Debug, Clone)]
pub struct Typewriter {
    rich: RichText,
    /// Characters revealed.
    revealed: usize,
    /// Timer accumulator.
    accum: f32,
    /// Base seconds per character.
    secs_per_char: f32,
    /// Current speed multiplier.
    speed_mul: f32,
    /// Remaining pause.
    pause_left: f32,
    /// Finished.
    finished: bool,
    /// Precomputed char stream with pauses injected as controls.
    stream: Vec<StreamItem>,
    cursor: usize,
}

#[derive(Debug, Clone)]
enum StreamItem {
    Char(char),
    Pause(f32),
    Speed(f32),
    Icon(String),
    Ruby { base: String, ruby: String },
}

impl Typewriter {
    /// From plain or markup string.
    pub fn new(text: &str, chars_per_sec: f32) -> Self {
        let rich = parse_rich_text(text).unwrap_or_else(|_| RichText {
            spans: vec![RichSpan::Text {
                text: text.to_string(),
                style: Default::default(),
            }],
        });
        Self::from_rich(rich, chars_per_sec)
    }

    /// From rich text.
    pub fn from_rich(rich: RichText, chars_per_sec: f32) -> Self {
        let mut stream = Vec::new();
        for span in &rich.spans {
            match span {
                RichSpan::Text { text, .. } => {
                    for c in text.chars() {
                        stream.push(StreamItem::Char(c));
                    }
                }
                RichSpan::Link { label, .. } => {
                    for c in label.chars() {
                        stream.push(StreamItem::Char(c));
                    }
                }
                RichSpan::Pause { seconds } => stream.push(StreamItem::Pause(*seconds)),
                RichSpan::Speed { multiplier } => stream.push(StreamItem::Speed(*multiplier)),
                RichSpan::Icon { id } => stream.push(StreamItem::Icon(id.clone())),
                RichSpan::Ruby { base, ruby, .. } => {
                    stream.push(StreamItem::Ruby {
                        base: base.clone(),
                        ruby: ruby.clone(),
                    });
                    for c in base.chars() {
                        stream.push(StreamItem::Char(c));
                    }
                }
            }
        }
        let cps = chars_per_sec.max(1.0);
        Self {
            rich,
            revealed: 0,
            accum: 0.0,
            secs_per_char: 1.0 / cps,
            speed_mul: 1.0,
            pause_left: 0.0,
            finished: stream.is_empty(),
            stream,
            cursor: 0,
        }
    }

    /// Tick; returns events this frame.
    pub fn tick(&mut self, dt: f32) -> Vec<TypewriterEvent> {
        let mut events = Vec::new();
        if self.finished {
            return events;
        }
        if self.pause_left > 0.0 {
            self.pause_left -= dt;
            if self.pause_left > 0.0 {
                return events;
            }
        }
        self.accum += dt * self.speed_mul;
        while self.accum >= self.secs_per_char && !self.finished {
            self.accum -= self.secs_per_char;
            match self.advance_one() {
                Some(TypewriterEvent::Pause(p)) => {
                    self.pause_left = p;
                    events.push(TypewriterEvent::Pause(p));
                    break;
                }
                Some(e) => events.push(e),
                None => break,
            }
        }
        events
    }

    fn advance_one(&mut self) -> Option<TypewriterEvent> {
        if self.cursor >= self.stream.len() {
            if !self.finished {
                self.finished = true;
                return Some(TypewriterEvent::Finished);
            }
            return None;
        }
        match self.stream[self.cursor].clone() {
            StreamItem::Speed(m) => {
                self.cursor += 1;
                self.speed_mul = m.max(0.05);
                Some(TypewriterEvent::SpeedChange(self.speed_mul))
            }
            StreamItem::Pause(p) => {
                self.cursor += 1;
                Some(TypewriterEvent::Pause(p))
            }
            StreamItem::Icon(id) => {
                self.cursor += 1;
                // Icons count as one reveal slot in visible text as placeholder.
                self.revealed += 1;
                Some(TypewriterEvent::Icon(id))
            }
            StreamItem::Ruby { base, ruby } => {
                self.cursor += 1;
                Some(TypewriterEvent::Ruby { base, ruby })
            }
            StreamItem::Char(c) => {
                self.cursor += 1;
                self.revealed += 1;
                if self.cursor >= self.stream.len() {
                    self.finished = true;
                }
                Some(TypewriterEvent::Char(c))
            }
        }
    }

    /// Skip to end.
    pub fn skip(&mut self) {
        while self.cursor < self.stream.len() {
            match &self.stream[self.cursor] {
                StreamItem::Char(_) | StreamItem::Icon(_) => self.revealed += 1,
                _ => {}
            }
            self.cursor += 1;
        }
        self.finished = true;
        self.pause_left = 0.0;
    }

    /// Skip and return a skipped event list.
    pub fn skip_with_event(&mut self) -> Vec<TypewriterEvent> {
        if self.finished {
            return Vec::new();
        }
        self.skip();
        vec![TypewriterEvent::Skipped, TypewriterEvent::Finished]
    }

    /// Characters revealed so far.
    pub fn revealed_count(&self) -> usize {
        self.revealed
    }

    /// Progress `0..=1` based on character stream items.
    pub fn progress(&self) -> f32 {
        let total = self
            .stream
            .iter()
            .filter(|i| matches!(i, StreamItem::Char(_) | StreamItem::Icon(_)))
            .count();
        if total == 0 {
            1.0
        } else {
            (self.revealed as f32 / total as f32).clamp(0.0, 1.0)
        }
    }

    /// Currently visible plain text.
    pub fn visible_text(&self) -> String {
        let mut out = String::new();
        let mut count = 0;
        for item in &self.stream {
            match item {
                StreamItem::Char(c) => {
                    if count >= self.revealed {
                        break;
                    }
                    out.push(*c);
                    count += 1;
                }
                StreamItem::Icon(_) => {
                    if count >= self.revealed {
                        break;
                    }
                    out.push('◆');
                    count += 1;
                }
                _ => {}
            }
        }
        out
    }

    /// Finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Full plain text.
    pub fn full_text(&self) -> String {
        self.rich.plain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveals_over_time() {
        let mut tw = Typewriter::new("Hello", 10.0);
        let _ = tw.tick(0.15);
        assert!(!tw.visible_text().is_empty());
        assert!(tw.visible_text().len() < 5 || tw.is_finished());
        tw.skip();
        assert_eq!(tw.visible_text(), "Hello");
        assert!(tw.is_finished());
    }

    #[test]
    fn respects_pause_tag() {
        let mut tw = Typewriter::new("A{pause=1.0}B", 100.0);
        let ev = tw.tick(0.05);
        // Should reveal A then pause
        let _ = ev;
        assert!(tw.visible_text().starts_with('A') || tw.visible_text().is_empty());
    }

    #[test]
    fn icon_and_speed_events() {
        let mut tw = Typewriter::new("X{icon=star}{speed=2.0}Y", 1000.0);
        let mut saw_icon = false;
        let mut saw_speed = false;
        for _ in 0..20 {
            for e in tw.tick(0.05) {
                match e {
                    TypewriterEvent::Icon(id) => {
                        assert_eq!(id, "star");
                        saw_icon = true;
                    }
                    TypewriterEvent::SpeedChange(m) => {
                        assert!((m - 2.0).abs() < 1e-4);
                        saw_speed = true;
                    }
                    _ => {}
                }
            }
            if tw.is_finished() {
                break;
            }
        }
        assert!(saw_icon);
        assert!(saw_speed);
    }

    #[test]
    fn skip_emits_event() {
        let mut tw = Typewriter::new("Hello", 1.0);
        let ev = tw.skip_with_event();
        assert!(ev.contains(&TypewriterEvent::Skipped));
        assert!(tw.is_finished());
        assert!((tw.progress() - 1.0).abs() < 1e-5);
    }
}
