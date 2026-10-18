//! The report of a build or a check: what was built, what was up to date, and every problem that
//! the run found. It has a text form for people and a JSON form (`velvet-art-build-report`) for tools.

use std::fmt::Write as _;

use serde::Serialize;

/// The `format` tag of the JSON report.
pub const REPORT_FORMAT: &str = "velvet-art-build-report";

/// The version of the JSON report.
pub const REPORT_VERSION: u32 = 1;

/// One output file: a picture in `art/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputRecord {
    /// The file's path inside `art/`, with `/` separators, such as `portrait/character.lyra.portrait.png`.
    pub path: String,
    /// The extension of the format: `avif`, `webp` or `png`.
    pub format: String,
    /// The source picture, relative to the source folder.
    pub source: String,
    /// The output width in pixels: the box times the scale.
    pub width: u32,
    /// The output height in pixels: the box times the scale.
    pub height: u32,
    /// The size of the file on disk, when it exists. A check does not write, so it is None for an output
    /// that a build would make.
    pub bytes: Option<u64>,
}

/// An art name with no source picture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissingSource {
    /// The art id that the name belongs to.
    pub id: String,
    /// The kind of the id.
    pub kind: String,
    /// The name of the picture, such as `card.wood` for the base picture or `card.wood.red` for a variant.
    pub name: String,
    /// The variant, or None for the base picture.
    pub variant: Option<String>,
    /// True when `art/` already has a file for the name, from an earlier build or by hand. The runtime
    /// shows that file, not a placeholder. False means the game shows the placeholder (or, for a variant,
    /// the base picture).
    pub has_file: bool,
}

/// A source whose proportions differ from the box by more than 15 percent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AspectWarning {
    /// The art name, such as `icon.coin`.
    pub name: String,
    /// The source picture, relative to the source folder.
    pub source: String,
    /// The source size in pixels.
    pub source_size: [u32; 2],
    /// The box in logical pixels.
    pub box_size: [u32; 2],
    /// The fit of the name: `cover` (the crop removes part of the picture) or `stretch` (the picture is
    /// distorted).
    pub fit: String,
    /// How far the source's proportions are from the box's, as a percentage: 100 times the source's
    /// aspect ratio divided by the box's, minus 1, in absolute value.
    pub difference_percent: f64,
}

/// An output file that is heavier than the limit of the config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeavyOutput {
    /// The file's path inside `art/`.
    pub path: String,
    /// The size of the file in bytes.
    pub bytes: u64,
    /// The limit in bytes that the file is over.
    pub limit: u64,
}

/// An output that was not written, because its format or its size cannot be written here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkippedOutput {
    /// The output's path inside `art/`.
    pub path: String,
    /// The format's extension.
    pub format: String,
    /// Why it was skipped, in a sentence.
    pub reason: String,
}

/// A source or an output that failed. The rest of the run goes on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    /// The source's path inside the source folder, or the output's path inside `art/`.
    pub path: String,
    /// What went wrong, in a sentence.
    pub message: String,
}

/// The report of one build or check.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ArtReport {
    /// True for a check: nothing was written, and `built` lists the outputs that a build would make.
    pub dry_run: bool,
    /// The outputs written by this build (a check: the outputs that a build would write).
    pub built: Vec<OutputRecord>,
    /// The outputs that were already up to date, and so were not made again.
    pub up_to_date: Vec<OutputRecord>,
    /// The art names that have no source picture.
    pub missing: Vec<MissingSource>,
    /// Files in `art/` that no entry of the manifest resolves to. The runtime never shows them.
    pub orphans: Vec<String>,
    /// Sources whose proportions differ from their box by more than 15 percent.
    pub aspect: Vec<AspectWarning>,
    /// Files in `art/` that the runtime may show and that are heavier than the limit.
    pub heavy: Vec<HeavyOutput>,
    /// Outputs that were not written because their format is not available or their size is too large.
    pub skipped: Vec<SkippedOutput>,
    /// Sources or outputs that failed.
    pub failed: Vec<Failure>,
}

impl ArtReport {
    /// An empty report for a build (`dry_run` false) or a check (`dry_run` true).
    pub fn new(dry_run: bool) -> Self {
        ArtReport {
            dry_run,
            ..ArtReport::default()
        }
    }

    /// True when the run found something to fix: a missing source, an orphan, an aspect warning, a heavy
    /// file, a skipped output or a failure. The `check` command exits with 1 then.
    pub fn has_problems(&self) -> bool {
        !(self.missing.is_empty()
            && self.orphans.is_empty()
            && self.aspect.is_empty()
            && self.heavy.is_empty()
            && self.skipped.is_empty()
            && self.failed.is_empty())
    }

    /// True when a source or an output failed. The `build` command exits with 1 then.
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// The report as JSON, with the `velvet-art-build-report` envelope.
    pub fn to_json(&self) -> String {
        #[derive(Serialize)]
        struct Envelope<'a> {
            format: &'static str,
            version: u32,
            #[serde(flatten)]
            report: &'a ArtReport,
        }
        let envelope = Envelope {
            format: REPORT_FORMAT,
            version: REPORT_VERSION,
            report: self,
        };
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string())
    }

    /// The report as plain text, for people: a summary line, then one section for each kind of item
    /// that is not empty.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        let built_label = if self.dry_run { "would build" } else { "built" };
        let _ = writeln!(
            text,
            "{}: {} {}, {} up to date, {} missing, {} orphan, {} aspect, {} heavy, {} skipped, {} failed",
            if self.dry_run { "art check" } else { "art build" },
            self.built.len(),
            built_label,
            self.up_to_date.len(),
            self.missing.len(),
            self.orphans.len(),
            self.aspect.len(),
            self.heavy.len(),
            self.skipped.len(),
            self.failed.len(),
        );
        section(
            &mut text,
            &format!("{} ({})", capitalised(built_label), self.built.len()),
            &self.built,
            |record| {
                let size = record
                    .bytes
                    .map_or(String::new(), |bytes| format!("  {}", human_size(bytes)));
                format!(
                    "{}  {}x{}  from {}{size}",
                    record.path, record.width, record.height, record.source
                )
            },
        );
        if !self.missing.is_empty() {
            let _ = writeln!(text, "\nMissing source pictures ({}):", self.missing.len());
            for item in &self.missing {
                let consequence = match (&item.variant, item.has_file) {
                    (_, true) => "art/ already has a file for it, and the game shows that file",
                    (Some(_), false) => "the game shows the base picture instead",
                    (None, false) => "the game shows the placeholder",
                };
                let _ = writeln!(
                    text,
                    "  {} ({}): no source picture; {consequence}",
                    item.name, item.kind
                );
            }
        }
        section(
            &mut text,
            "Orphan files in art/ (no manifest entry resolves to them)",
            &self.orphans,
            |path| path.clone(),
        );
        section(
            &mut text,
            "Aspect ratio (the source differs from its box by more than 15%)",
            &self.aspect,
            |warning| {
                let ratio = warning.source_size[0] as f64 / warning.source_size[1] as f64;
                let box_ratio = warning.box_size[0] as f64 / warning.box_size[1] as f64;
                let effect = if warning.fit == "stretch" {
                    format!("stretched by {:.0}%", warning.difference_percent)
                } else {
                    let lost = (1.0 - ratio.min(box_ratio) / ratio.max(box_ratio)) * 100.0;
                    format!("cover crops away about {lost:.0}% of the picture")
                };
                format!(
                    "{}: {} is {}x{}, the box is {}x{}; {effect}",
                    warning.name,
                    warning.source,
                    warning.source_size[0],
                    warning.source_size[1],
                    warning.box_size[0],
                    warning.box_size[1]
                )
            },
        );
        section(
            &mut text,
            "Heavy outputs (over the size limit)",
            &self.heavy,
            |item| {
                format!(
                    "{}  {} (limit {})",
                    item.path,
                    human_size(item.bytes),
                    human_size(item.limit)
                )
            },
        );
        section(&mut text, "Skipped outputs", &self.skipped, |item| {
            format!("{}: {}", item.path, item.reason)
        });
        section(&mut text, "Failed", &self.failed, |item| {
            format!("{}: {}", item.path, item.message)
        });
        text
    }
}

/// Appends a titled list of items, one line each, when there are any.
fn section<T>(text: &mut String, title: &str, items: &[T], line: impl Fn(&T) -> String) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(text, "\n{title}:");
    for item in items {
        let _ = writeln!(text, "  {}", line(item));
    }
}

/// Capitalises the first letter of a word.
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A size in bytes as people read it: bytes, KiB or MiB.
pub fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MiB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_report_has_no_problems() {
        let report = ArtReport::new(false);
        assert!(!report.has_problems());
        assert!(!report.has_failures());
        assert!(report.to_text().starts_with("art build: 0 built"));
    }

    #[test]
    fn a_missing_source_is_a_problem_but_not_a_failure() {
        let mut report = ArtReport::new(true);
        report.missing.push(MissingSource {
            id: "card.wood".into(),
            kind: "card".into(),
            name: "card.wood".into(),
            variant: None,
            has_file: false,
        });
        assert!(report.has_problems());
        assert!(!report.has_failures());
        assert!(report.to_text().contains("the game shows the placeholder"));
    }

    #[test]
    fn json_carries_the_envelope() {
        let json: serde_json::Value =
            serde_json::from_str(&ArtReport::new(false).to_json()).unwrap();
        assert_eq!(json["format"], REPORT_FORMAT);
        assert_eq!(json["version"], 1);
        assert_eq!(json["built"], serde_json::json!([]));
    }

    #[test]
    fn sizes_read_as_people_expect() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KiB");
        assert_eq!(human_size(3 * 1024 * 1024), "3.00 MiB");
    }
}
