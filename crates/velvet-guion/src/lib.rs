//! Guiones: dialogue and visual-novel scenes written as Markdown, the way a writer drafts them.
//! One file holds scenes (`## heading`); a scene is a run of lines, one box each:
//!
//! ```text
//! ---
//! id: ana
//! sala: posada
//! ---
//! # Ana (a title, for the writer)
//! > A note for the writer: ignored.
//!
//! ## saludo {quien=ana}
//! **Ana:** —¿Qué te pongo, viajera?
//! La capitana se sentó junto a la chimenea.     (narration)
//! *clic.*                                       (a sound)
//! (Tengo que decírselo ya.)                     (the thinker thinks)
//! ? ¿Qué le pide?
//! - Un té -> te {ana_confianza +1}
//! - Hablar de Teo [ana_confianza >= 2] -> teo
//! - Nada -> fin
//! @si ana_confianza >= 3 -> confianza
//! @var ana_confianza +1
//! @vn recuerdo_1         (opens the visual novel at that scene)
//! @fin
//! ```
//!
//! Any other `key: value` line in the front matter is the file's metadata ([`Guion::meta`], read
//! with [`Guion::meta_value`]), kept in file order with its value as written.
//!
//! Scene attributes, in braces after the heading: `quien=<npc>` (talking to that NPC can start it),
//! `en=<place>` (using that hotspot can), `si=<condition>` (only while it holds), `una_vez` (only
//! the first time), `vn` (a visual-novel scene: compiled into the story's nodes). Stage directions
//! for the visual novel: `@sala`, `@fondo`, `@cg`, `@quitar`, `@sprite <quien> <imagen>`.
//!
//! Who speaks is a name (`**Ana:**`, `**Ana (enfadada):**` with the look of her portrait); a
//! game's [`Cast`] turns the names its writers use into its character ids, and says whose thoughts
//! the bracketed lines are.
//!
//! The same file plays in a 3D world as a dialogue box ([`Conversation`], with the story's
//! [`Vars`]) and, for `vn` scenes, in the visual novel ([`compile_vn`] turns them into
//! velvet-story nodes). [`Portraits`] are the faces of the dialogue box.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ the script
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cond {
    pub var: String,
    pub op: Op,
    pub value: i32,
}

impl Cond {
    pub fn holds(&self, value: i32) -> bool {
        match self.op {
            Op::Eq => value == self.value,
            Op::Ne => value != self.value,
            Op::Gt => value > self.value,
            Op::Gte => value >= self.value,
            Op::Lt => value < self.value,
            Op::Lte => value <= self.value,
        }
    }
    fn json_op(&self) -> &'static str {
        match self.op {
            Op::Eq => "eq",
            Op::Ne => "ne",
            Op::Gt => "gt",
            Op::Gte => "gte",
            Op::Lt => "lt",
            Op::Lte => "lte",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffOp {
    Set,
    Add,
    Sub,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub var: String,
    pub op: EffOp,
    pub value: i32,
}

impl Effect {
    pub fn apply(&self, current: i32) -> i32 {
        match self.op {
            EffOp::Set => self.value,
            EffOp::Add => current + self.value,
            EffOp::Sub => current - self.value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Dialogue,
    Narration,
    Sound,
    Thought,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Opt {
    pub label: String,
    pub cond: Vec<Cond>,
    pub effects: Vec<Effect>,
    /// The scene it leads to (`None`: on with the next line).
    pub to: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// `mood`: how the speaker looks while saying it (`**Lira (enfadada):**`), for the portrait.
    Line {
        who: String,
        name: String,
        text: String,
        kind: LineKind,
        line: usize,
        mood: Option<String>,
    },
    Choice {
        prompt: Option<String>,
        options: Vec<Opt>,
        line: usize,
    },
    /// `@salta x` (no condition) or `@si cond -> x`.
    Jump {
        to: String,
        cond: Vec<Cond>,
        line: usize,
    },
    Set {
        effects: Vec<Effect>,
    },
    End,
    Vn {
        to: String,
        line: usize,
    },
    /// Staging: the visual novel's (`@sala`, `@fondo`, `@cg`, `@quitar`, `@sprite`) and the
    /// world's full-screen stage's (`@izq`, `@der`, `@pose`, `@titulo`, `@sello`, `@sacude`,
    /// `@bote`, `@destello`, `@barrido`, `@objeto`, `@panel`, `@revela`, `@giro`, `@desbloquea`,
    /// `@cierre`, `@recuerdo`).
    /// The 3D box shows none of it; a [`Conversation`] hands it over with [`Conversation::take_staging`].
    Stage {
        cmd: String,
        arg: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attrs {
    pub quien: Option<String>,
    pub en: Option<String>,
    pub si: Vec<Cond>,
    pub una_vez: bool,
    pub vn: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// `<file id>.<label>`: how other files and the game name it.
    pub id: String,
    pub label: String,
    pub title: String,
    pub attrs: Attrs,
    pub steps: Vec<Step>,
    pub line: usize,
    /// Some jump or option of its file leads here: a branch, which a scene before it does not
    /// run on into.
    pub targeted: bool,
}

impl Scene {
    fn is_entry(&self) -> bool {
        self.attrs.quien.is_some() || self.attrs.en.is_some() || self.attrs.vn
    }

    /// Whether the scene before runs on into this one (the drafts' sections follow each other;
    /// entries and branches stand alone).
    fn follows_on(&self) -> bool {
        !self.is_entry() && !self.targeted
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Guion {
    pub id: String,
    pub title: String,
    pub sala: Option<String>,
    /// The open keys of the front matter: every `key: value` line other than `id`, `sala` and
    /// `titulo`, in the order the file gives them. Values are kept as written (a list stays
    /// `a, b, c`; the game splits it). See docs/formats/GUION.md, "Metadata".
    pub meta: Vec<(String, String)>,
    pub scenes: Vec<Scene>,
    pub path: PathBuf,
}

impl Guion {
    /// The value of an open metadata key (`categoria`, say), or None when the front matter has no
    /// such key.
    pub fn meta_value(&self, key: &str) -> Option<&str> {
        self.meta
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// A mistake in a guion, with where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub file: String,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.message)
    }
}

// ------------------------------------------------------------------ reading
/// A label as an identifier: lower case, accents off, words joined by `_`.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' => 'u',
            'ñ' | 'Ñ' => 'n',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_owned();
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("s{out}")
    } else {
        out
    }
}

/// What a thought's speaker id ends with: the bracketed lines are spoken by `<thinker>_thought`,
/// and show the thinker's portrait.
pub const THOUGHT_SUFFIX: &str = "_thought";

/// Who is who in a game's guiones: the names each character goes by (slugged, so `La Capitana`
/// and `la capitana` are one name) and whose thoughts the bracketed lines are. `narrador` and `narrator`
/// name the narrator; a name not listed is its own id.
#[derive(Clone, Debug, PartialEq)]
pub struct Cast {
    aliases: BTreeMap<String, String>,
    /// (character id, name shown) of the one who thinks the bracketed lines.
    thinker: Option<(String, String)>,
}

impl Default for Cast {
    fn default() -> Self {
        Self::new()
    }
}

impl Cast {
    pub fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
            thinker: None,
        }
        .alias("narrator", &["narrador", "narrator"])
    }

    /// Every one of `names` (written any way: they are slugged) names character `id`.
    pub fn alias(mut self, id: &str, names: &[&str]) -> Self {
        for name in names {
            self.aliases.insert(slug(name), id.to_owned());
        }
        self
    }

    /// The bracketed lines are character `id`'s thoughts, shown under `name`.
    pub fn thinker(mut self, id: &str, name: &str) -> Self {
        self.thinker = Some((id.to_owned(), name.to_owned()));
        self
    }

    /// Who speaks, as the story's character id.
    pub fn speaker_id(&self, name: &str) -> String {
        let name = slug(name);
        self.aliases.get(&name).cloned().unwrap_or(name)
    }

    /// The speaker id and the name shown of a thought (nobody's in particular without a thinker).
    pub fn thought(&self) -> (String, String) {
        match &self.thinker {
            Some((id, name)) => (format!("{id}{THOUGHT_SUFFIX}"), name.clone()),
            None => (
                THOUGHT_SUFFIX.trim_start_matches('_').to_owned(),
                String::new(),
            ),
        }
    }
}

fn parse_cond(text: &str) -> Option<Cond> {
    let t = text.trim();
    for (sym, op) in [
        (">=", Op::Gte),
        ("<=", Op::Lte),
        ("!=", Op::Ne),
        ("==", Op::Eq),
        (">", Op::Gt),
        ("<", Op::Lt),
        ("=", Op::Eq),
    ] {
        if let Some((a, b)) = t.split_once(sym) {
            let var = slug(a);
            let value = b.trim().parse().ok()?;
            if var.is_empty() {
                return None;
            }
            return Some(Cond { var, op, value });
        }
    }
    // A bare name: "it is set" (above zero).
    let var = slug(t);
    (!var.is_empty()).then_some(Cond {
        var,
        op: Op::Gt,
        value: 0,
    })
}

fn parse_conds(text: &str) -> Option<Vec<Cond>> {
    text.split([',', '&'])
        .filter(|s| !s.trim().is_empty())
        .map(parse_cond)
        .collect()
}

/// `x +1`, `x -2`, `x = 3`, `x` (= 1).
fn parse_effect(text: &str) -> Option<Effect> {
    let t = text.trim();
    if let Some((a, b)) = t.split_once('=') {
        return Some(Effect {
            var: slug(a),
            op: EffOp::Set,
            value: b.trim().parse().ok()?,
        });
    }
    if let Some(i) = t.find(['+', '-']) {
        let (a, b) = t.split_at(i);
        let n: i32 = b[1..].trim().parse().ok()?;
        let op = if b.starts_with('+') {
            EffOp::Add
        } else {
            EffOp::Sub
        };
        return Some(Effect {
            var: slug(a),
            op,
            value: n,
        });
    }
    let var = slug(t);
    (!var.is_empty()).then_some(Effect {
        var,
        op: EffOp::Set,
        value: 1,
    })
}

fn parse_effects(text: &str) -> Option<Vec<Effect>> {
    text.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(parse_effect)
        .collect()
}

/// The `{...}` at the end of a heading: its attributes.
fn parse_attrs(text: &str) -> Option<Attrs> {
    let mut a = Attrs::default();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('=').map(|(k, v)| (slug(k), v.trim())) {
            Some((k, v)) if k == "quien" => a.quien = Some(slug(v)),
            Some((k, v)) if k == "en" => a.en = Some(slug(v)),
            Some((k, v)) if k == "si" => a.si = parse_conds(v)?,
            None if slug(part) == "una_vez" => a.una_vez = true,
            None if slug(part) == "vn" => a.vn = true,
            _ => return None,
        }
    }
    Some(a)
}

/// Splits `label [cond] -> target {effects}` into its pieces.
fn parse_option(text: &str) -> Option<Opt> {
    let mut rest = text.trim().to_owned();
    let mut effects = Vec::new();
    if rest.ends_with('}') {
        let open = rest.rfind('{')?;
        effects = parse_effects(&rest[open + 1..rest.len() - 1])?;
        rest.truncate(open);
    }
    let mut to = None;
    if let Some(i) = rest.find("->") {
        let target = rest[i + 2..].trim().to_owned();
        to = Some(target);
        rest.truncate(i);
    }
    let mut cond = Vec::new();
    let r = rest.trim_end();
    if r.ends_with(']') {
        let open = r.rfind('[')?;
        cond = parse_conds(&r[open + 1..r.len() - 1])?;
        rest = r[..open].to_owned();
    }
    let label = rest.trim().to_owned();
    (!label.is_empty()).then_some(Opt {
        label,
        cond,
        effects,
        to,
    })
}

/// A dialogue's text without its leading dash (the drafts open each line with "—").
fn spoken(text: &str) -> String {
    text.trim()
        .trim_start_matches(['—', '–', '-'])
        .trim()
        .to_owned()
}

/// Whether `key` may name an open metadata entry: a lower-case ASCII letter, then lower-case letters,
/// digits or `_`. Starting with a letter keeps a key apart from a number, so a JavaScript object
/// keeps the keys in the order of the file (JavaScript sorts number-like keys first).
fn is_meta_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Reads one guion, naming its speakers by `cast`. Problems are collected (with their lines)
/// rather than stopping at the first.
pub fn parse(source: &str, path: &Path, cast: &Cast) -> (Guion, Vec<Problem>) {
    let file = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut g = Guion {
        id: slug(
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
                .as_str(),
        ),
        path: path.to_path_buf(),
        ..Default::default()
    };
    let mut problems = Vec::new();
    let mut problem = |line: usize, message: String| {
        problems.push(Problem {
            file: file.clone(),
            line,
            message,
        })
    };
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    // Front matter: the keys it has set (a key given twice is reported and its second value dropped),
    // then the open metadata of every other key.
    if lines.first().is_some_and(|l| l.trim() == "---") {
        i = 1;
        let mut set: BTreeSet<&str> = BTreeSet::new();
        while i < lines.len() && lines[i].trim() != "---" {
            if let Some((k, v)) = lines[i].split_once(':') {
                let written = k.trim();
                // The three known keys are matched as slugs, as they always were.
                let known = match slug(written).as_str() {
                    "id" => Some("id"),
                    "sala" => Some("sala"),
                    "titulo" | "title" => Some("titulo"),
                    _ => None,
                };
                if known.is_none() && !is_meta_key(written) {
                    problem(
                        i + 1,
                        format!("clave no válida en la cabecera: {written} (solo minúsculas, dígitos y _, y una letra al principio)"),
                    );
                } else if !set.insert(known.unwrap_or(written)) {
                    problem(i + 1, format!("clave repetida en la cabecera: {written}"));
                } else {
                    match known {
                        Some("id") => g.id = slug(v),
                        Some("sala") => g.sala = Some(slug(v)),
                        Some(_) => g.title = v.trim().to_owned(),
                        None => g.meta.push((written.to_owned(), v.trim().to_owned())),
                    }
                }
            }
            i += 1;
        }
        i += 1;
    }
    let mut comment = false;
    let mut choice: Option<(Option<String>, Vec<Opt>, usize)> = None;
    let flush = |choice: &mut Option<(Option<String>, Vec<Opt>, usize)>,
                 scenes: &mut Vec<Scene>| {
        if let Some((prompt, options, line)) = choice.take() {
            if let Some(s) = scenes.last_mut() {
                s.steps.push(Step::Choice {
                    prompt,
                    options,
                    line,
                });
            }
        }
    };
    while i < lines.len() {
        let n = i + 1;
        let raw = lines[i];
        i += 1;
        let t = raw.trim();
        if comment {
            if t.contains("-->") {
                comment = false;
            }
            continue;
        }
        if t.starts_with("<!--") {
            comment = !t.contains("-->");
            continue;
        }
        if t.is_empty() || t.starts_with('>') || t == "---" {
            continue;
        }
        // An option of the choice being written.
        if let Some(opt) = t.strip_prefix("- ").filter(|_| choice.is_some()) {
            match parse_option(opt) {
                Some(o) => choice.as_mut().unwrap().1.push(o),
                None => problem(n, format!("opción ilegible: {opt}")),
            }
            continue;
        }
        flush(&mut choice, &mut g.scenes);
        if let Some(h) = t.strip_prefix("# ") {
            if g.title.is_empty() {
                g.title = h.trim().to_owned();
            }
            continue;
        }
        if t.starts_with("##") {
            let h = t.trim_start_matches('#').trim();
            let (title, attrs) = match h.rfind('{').filter(|_| h.ends_with('}')) {
                Some(open) => (h[..open].trim(), parse_attrs(&h[open + 1..h.len() - 1])),
                None => (h, Some(Attrs::default())),
            };
            let attrs = attrs.unwrap_or_else(|| {
                problem(n, format!("atributos ilegibles en: {h}"));
                Attrs::default()
            });
            let label = slug(title);
            if label.is_empty() {
                problem(n, "escena sin nombre".into());
            }
            if g.scenes.iter().any(|s| s.label == label) {
                problem(n, format!("escena repetida: {label}"));
            }
            g.scenes.push(Scene {
                id: String::new(),
                label,
                title: title.to_owned(),
                attrs,
                steps: Vec::new(),
                line: n,
                targeted: false,
            });
            continue;
        }
        if g.scenes.is_empty() {
            // Lines before the first scene open an unnamed one.
            g.scenes.push(Scene {
                id: String::new(),
                label: "inicio".into(),
                title: "Inicio".into(),
                attrs: Attrs::default(),
                steps: Vec::new(),
                line: n,
                targeted: false,
            });
        }
        let scene = g.scenes.last_mut().unwrap();
        if let Some(prompt) = t.strip_prefix('?') {
            let prompt = prompt.trim();
            choice = Some((
                (!prompt.is_empty()).then(|| prompt.to_owned()),
                Vec::new(),
                n,
            ));
            continue;
        }
        if let Some(cmd) = t.strip_prefix('@') {
            let (word, arg) = cmd
                .split_once(char::is_whitespace)
                .map(|(w, a)| (w, a.trim()))
                .unwrap_or((cmd, ""));
            let step = match slug(word).as_str() {
                "fin" => Some(Step::End),
                "salta" | "ir" => Some(Step::Jump {
                    to: arg.to_owned(),
                    cond: Vec::new(),
                    line: n,
                }),
                "si" => arg.split_once("->").and_then(|(c, to)| {
                    Some(Step::Jump {
                        to: to.trim().to_owned(),
                        cond: parse_conds(c)?,
                        line: n,
                    })
                }),
                "var" => parse_effects(arg).map(|effects| Step::Set { effects }),
                "vn" => Some(Step::Vn {
                    to: arg.to_owned(),
                    line: n,
                }),
                w @ ("sala" | "fondo" | "cg" | "quitar" | "sprite" | "musica" | "izq" | "der"
                | "pose" | "titulo" | "sello" | "sacude" | "bote" | "destello" | "barrido"
                | "objeto" | "panel" | "revela" | "giro" | "desbloquea" | "cierre"
                | "teclas" | "llevar" | "recuerdo") => Some(Step::Stage {
                    cmd: w.to_owned(),
                    arg: arg.to_owned(),
                }),
                _ => None,
            };
            match step {
                Some(s) => scene.steps.push(s),
                None => problem(n, format!("orden desconocida o ilegible: @{cmd}")),
            }
            continue;
        }
        // A line: dialogue, a sound, a thought, or narration.
        let line = if let Some(rest) = t.strip_prefix("**") {
            match rest.split_once("**") {
                Some((name, text)) => {
                    let name = name.trim().trim_end_matches(':').trim();
                    let text = spoken(text.trim_start_matches(':'));
                    // `Lira (enfadada)`: the name, and the mood in brackets.
                    let (name, mood) = match name.find('(') {
                        Some(i) if name.ends_with(')') => (
                            name[..i].trim(),
                            Some(slug(&name[i + 1..name.len() - 1])).filter(|m| !m.is_empty()),
                        ),
                        _ => (name, None),
                    };
                    Step::Line {
                        who: cast.speaker_id(name),
                        name: name.to_owned(),
                        text,
                        kind: LineKind::Dialogue,
                        line: n,
                        mood,
                    }
                }
                None => Step::Line {
                    who: "narrator".into(),
                    name: String::new(),
                    text: t.to_owned(),
                    kind: LineKind::Narration,
                    line: n,
                    mood: None,
                },
            }
        } else if t.starts_with('*') && t.ends_with('*') && t.len() > 2 {
            Step::Line {
                who: "narrator".into(),
                name: String::new(),
                text: t.trim_matches('*').trim().to_owned(),
                kind: LineKind::Sound,
                line: n,
                mood: None,
            }
        } else if t.starts_with('(') && t.ends_with(')') {
            let (who, name) = cast.thought();
            Step::Line {
                who,
                name,
                text: t[1..t.len() - 1].trim().to_owned(),
                kind: LineKind::Thought,
                line: n,
                mood: None,
            }
        } else {
            Step::Line {
                who: "narrator".into(),
                name: String::new(),
                text: t.to_owned(),
                kind: LineKind::Narration,
                line: n,
                mood: None,
            }
        };
        if let Step::Line { text, .. } = &line {
            if text.is_empty() {
                problem(n, "línea sin texto".into());
                continue;
            }
        }
        scene.steps.push(line);
    }
    flush(&mut choice, &mut g.scenes);
    for s in &mut g.scenes {
        s.id = format!("{}.{}", g.id, s.label);
    }
    // Which scenes are branches (some jump of the file lands on them).
    let mut targets: BTreeSet<String> = BTreeSet::new();
    for s in &g.scenes {
        for step in &s.steps {
            match step {
                Step::Jump { to, .. } | Step::Vn { to, .. } if !to.contains('.') => {
                    targets.insert(slug(to));
                }
                Step::Choice { options, .. } => targets.extend(
                    options
                        .iter()
                        .filter_map(|o| o.to.as_deref())
                        .filter(|t| !t.contains('.'))
                        .map(slug),
                ),
                _ => {}
            }
        }
    }
    for s in &mut g.scenes {
        s.targeted = targets.contains(&s.label);
    }
    // Every jump must land somewhere.
    let labels: BTreeSet<&str> = g.scenes.iter().map(|s| s.label.as_str()).collect();
    for s in &g.scenes {
        for step in &s.steps {
            let targets: Vec<(&str, usize)> = match step {
                Step::Jump { to, line, .. } | Step::Vn { to, line } => vec![(to.as_str(), *line)],
                Step::Choice { options, line, .. } => options
                    .iter()
                    .filter_map(|o| o.to.as_deref().map(|t| (t, *line)))
                    .collect(),
                _ => Vec::new(),
            };
            for (t, line) in targets {
                let local = slug(t);
                if t != "fin" && !t.contains('.') && !labels.contains(local.as_str()) {
                    problem(line, format!("no hay ninguna escena «{t}» en este guion"));
                }
            }
        }
        if let Some(Step::Choice { options, line, .. }) = s
            .steps
            .iter()
            .find(|st| matches!(st, Step::Choice { options, .. } if options.is_empty()))
        {
            let _ = options;
            problem(*line, "una elección sin opciones".into());
        }
    }
    (g, problems)
}

// ------------------------------------------------------------------ the library
/// A game's guiones, read with its [`Cast`].
#[derive(Clone, Debug, Default)]
pub struct Library {
    pub guiones: Vec<Guion>,
    pub problems: Vec<Problem>,
    /// Who is who in them (the staging of [`compile_vn`] names sprites by it too).
    pub cast: Cast,
}

impl Library {
    /// An empty library, for guiones read with `cast`.
    pub fn new(cast: Cast) -> Self {
        Self {
            guiones: Vec::new(),
            problems: Vec::new(),
            cast,
        }
    }

    /// Every `.md` guion in `dir`, in name order.
    pub fn load(dir: &Path, cast: Cast) -> Self {
        let mut lib = Self::new(cast);
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|d| {
                d.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "md"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for path in files {
            match std::fs::read_to_string(&path) {
                Ok(text) => lib.add(&text, &path),
                Err(e) => lib.problems.push(Problem {
                    file: path.display().to_string(),
                    line: 0,
                    message: e.to_string(),
                }),
            }
        }
        lib
    }

    /// The variables that playing the scene at `at` can set or change, with everything it leads
    /// to in the guiones (its jumps, its options' scenes, the scenes that follow on from it).
    pub fn vars_set_from(&self, at: (usize, usize)) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut todo = vec![at];
        let mut visited = BTreeSet::new();
        while let Some((f, s)) = todo.pop() {
            if !visited.insert((f, s)) {
                continue;
            }
            let Some(scene) = self.guiones.get(f).and_then(|g| g.scenes.get(s)) else {
                continue;
            };
            for step in &scene.steps {
                match step {
                    Step::Set { effects } => out.extend(effects.iter().map(|e| e.var.clone())),
                    Step::Choice { options, .. } => {
                        for o in options {
                            out.extend(o.effects.iter().map(|e| e.var.clone()));
                            if let Some(next) =
                                o.to.as_deref().and_then(|to| self.find(to, Some(f)))
                            {
                                todo.push(next);
                            }
                        }
                    }
                    Step::Jump { to, .. } => {
                        if let Some(next) = self.find(to, Some(f)) {
                            todo.push(next);
                        }
                    }
                    _ => {}
                }
            }
            if self.guiones[f]
                .scenes
                .get(s + 1)
                .is_some_and(|next| next.follows_on())
            {
                todo.push((f, s + 1));
            }
        }
        out
    }

    /// Reads one more guion (`path` names it); its problems join the library's.
    pub fn add(&mut self, source: &str, path: &Path) {
        let (g, problems) = parse(source, path, &self.cast);
        self.problems.extend(problems);
        self.guiones.push(g);
    }

    /// (file, scene) of a scene named `id` (`file.label`), or `label` within `from`'s file.
    pub fn find(&self, id: &str, from: Option<usize>) -> Option<(usize, usize)> {
        if let Some((file, label)) = id.split_once('.') {
            let f = self.guiones.iter().position(|g| g.id == slug(file))?;
            let s = self.guiones[f]
                .scenes
                .iter()
                .position(|s| s.label == slug(label))?;
            return Some((f, s));
        }
        let f = from?;
        let s = self.guiones[f]
            .scenes
            .iter()
            .position(|s| s.label == slug(id))?;
        Some((f, s))
    }

    /// The scene to play on meeting `who` (or using place `en`): the first, in file order, whose
    /// condition holds and that is not a spent `una_vez`.
    pub fn entry(
        &self,
        who: Option<&str>,
        en: Option<&str>,
        vars: &Vars,
    ) -> Option<(usize, usize)> {
        let (who, en) = (who.map(slug), en.map(slug));
        let (who, en) = (who.as_deref(), en.as_deref());
        for (f, g) in self.guiones.iter().enumerate() {
            for (s, scene) in g.scenes.iter().enumerate() {
                let a = &scene.attrs;
                let matches = (who.is_some() && a.quien.as_deref() == who)
                    || (en.is_some() && a.en.as_deref() == en);
                if !matches || (a.una_vez && vars.seen(&scene.id)) {
                    continue;
                }
                if a.si.iter().all(|c| c.holds(vars.get(&c.var))) {
                    return Some((f, s));
                }
            }
        }
        None
    }
}

// ------------------------------------------------------------------ portraits
/// One portrait: an image of the game's art (an asset key, without `.png`) and the square of it
/// that holds the head and shoulders (`[x, y, width, height]` in its pixels).
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
pub struct Portrait {
    #[serde(rename = "imagen")]
    pub image: String,
    #[serde(rename = "recorte")]
    pub crop: [u32; 4],
}

/// The portraits of the dialogue box (a `retratos.json`): for each character, one per mood
/// (`normal` is the default). Synonyms are listed as `"enojada": "enfadada"` (a mood naming
/// another):
///
/// ```json
/// { "ana": { "normal": { "imagen": "retratos/ana", "recorte": [120, 40, 512, 512] }, "enojada": "enfadada" } }
/// ```
#[derive(Clone, Debug, Default)]
pub struct Portraits {
    by_who: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
}

impl Portraits {
    /// The portraits listed in the file at `path` (none, said on stderr, if it cannot be read).
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| {
                Self::parse(&text)
                    .map_err(|e| eprintln!("[guion] {}: {e}", path.display()))
                    .ok()
            })
            .unwrap_or_default()
    }

    /// The portraits listed in `text`.
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        Ok(Self {
            by_who: serde_json::from_str(text)?,
        })
    }

    /// The portrait of `who` looking `mood` (else their normal one). A thought shows its thinker.
    pub fn find(&self, who: &str, mood: Option<&str>) -> Option<Portrait> {
        let who = who.strip_suffix(THOUGHT_SUFFIX).unwrap_or(who);
        let set = self.by_who.get(who)?;
        let mut key = mood
            .map(slug)
            .filter(|m| set.contains_key(m))
            .unwrap_or_else(|| "normal".into());
        // A mood may name another (a synonym); follow a few.
        for _ in 0..4 {
            match set.get(&key)? {
                serde_json::Value::String(other) => key = slug(other),
                value => return serde_json::from_value(value.clone()).ok(),
            }
        }
        None
    }

    /// Every portrait listed, as (who, mood).
    pub fn entries(&self) -> Vec<(String, String)> {
        self.by_who
            .iter()
            .flat_map(|(w, set)| set.keys().map(move |m| (w.clone(), m.clone())))
            .collect()
    }
}

// ------------------------------------------------------------------ the story's state
/// The story's variables and the scenes already seen (saved with the game).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vars {
    #[serde(default)]
    pub values: BTreeMap<String, i32>,
    #[serde(default)]
    pub seen: BTreeSet<String>,
}

impl Vars {
    pub fn get(&self, var: &str) -> i32 {
        self.values.get(var).copied().unwrap_or(0)
    }
    pub fn apply(&mut self, e: &Effect) {
        let v = e.apply(self.get(&e.var));
        self.values.insert(e.var.clone(), v);
    }
    pub fn seen(&self, scene: &str) -> bool {
        self.seen.contains(scene)
    }
}

// ------------------------------------------------------------------ playing it in the world
/// What the dialogue box shows now.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    Line {
        who: String,
        name: String,
        text: String,
        kind: LineKind,
        mood: Option<String>,
    },
    /// A choice: its prompt, and each option with whether it can be taken.
    Choice {
        prompt: Option<String>,
        options: Vec<(String, bool)>,
    },
    /// The conversation hands over to the visual novel at this scene.
    Vn(String),
    End,
}

/// How the words a conversation shows reach the player's language: the host sets it once; with
/// none, the text is shown as written.
static TRANSLATOR: std::sync::OnceLock<fn(&str) -> String> = std::sync::OnceLock::new();

/// Sets how lines, names, prompts and options are translated before they are shown.
pub fn set_translator(translate: fn(&str) -> String) {
    let _ = TRANSLATOR.set(translate);
}

fn translated(s: &str) -> String {
    TRANSLATOR.get().map_or_else(|| s.to_owned(), |t| t(s))
}

/// A conversation in progress: which scene, which step.
#[derive(Clone, Debug, PartialEq)]
pub struct Conversation {
    file: usize,
    scene: usize,
    step: usize,
    finished: bool,
    vn: Option<String>,
    /// The staging passed since it was last taken (command, argument), in order.
    staged: Vec<(String, String)>,
}

impl Conversation {
    /// Starts scene (file, scene) and runs it up to its first box.
    pub fn start(lib: &Library, at: (usize, usize), vars: &mut Vars) -> Self {
        let mut c = Self {
            file: at.0,
            scene: at.1,
            step: 0,
            finished: false,
            vn: None,
            staged: Vec::new(),
        };
        vars.seen.insert(lib.guiones[at.0].scenes[at.1].id.clone());
        c.settle(lib, vars);
        c
    }

    pub fn scene_id<'a>(&self, lib: &'a Library) -> &'a str {
        &lib.guiones[self.file].scenes[self.scene].id
    }

    fn goto(&mut self, lib: &Library, target: &str, vars: &mut Vars) {
        if slug(target) == "fin" {
            self.finished = true;
            return;
        }
        match lib.find(target, Some(self.file)) {
            Some((f, s)) => {
                self.file = f;
                self.scene = s;
                self.step = 0;
                vars.seen.insert(lib.guiones[f].scenes[s].id.clone());
            }
            None => self.finished = true,
        }
    }

    /// Runs what is not shown (variables, jumps, staging) until a box, a choice or the end.
    fn settle(&mut self, lib: &Library, vars: &mut Vars) {
        for _ in 0..10_000 {
            if self.finished || self.vn.is_some() {
                return;
            }
            let g = &lib.guiones[self.file];
            let scene = &g.scenes[self.scene];
            let Some(step) = scene.steps.get(self.step) else {
                // Off the end of a scene: on into the next one, unless that one is an entry of its own.
                match g.scenes.get(self.scene + 1) {
                    Some(next) if next.follows_on() => {
                        self.scene += 1;
                        self.step = 0;
                        vars.seen.insert(next.id.clone());
                        continue;
                    }
                    _ => {
                        self.finished = true;
                        return;
                    }
                }
            };
            match step {
                Step::Line { .. } | Step::Choice { .. } => return,
                Step::Set { effects } => {
                    for e in effects {
                        vars.apply(e);
                    }
                    self.step += 1;
                }
                Step::Jump { to, cond, .. } => {
                    if cond.iter().all(|c| c.holds(vars.get(&c.var))) {
                        let to = to.clone();
                        self.goto(lib, &to, vars);
                    } else {
                        self.step += 1;
                    }
                }
                Step::Stage { cmd, arg } => {
                    self.staged.push((cmd.clone(), arg.clone()));
                    self.step += 1;
                }
                Step::End => self.finished = true,
                Step::Vn { to, .. } => {
                    let full = if to.contains('.') {
                        to.clone()
                    } else {
                        format!("{}.{}", g.id, slug(to))
                    };
                    self.vn = Some(full);
                }
            }
        }
        self.finished = true;
    }

    /// What to show now.
    pub fn shown(&self, lib: &Library, vars: &Vars) -> Shown {
        if let Some(to) = &self.vn {
            return Shown::Vn(to.clone());
        }
        if self.finished {
            return Shown::End;
        }
        match lib.guiones[self.file].scenes[self.scene]
            .steps
            .get(self.step)
        {
            Some(Step::Line {
                who,
                name,
                text,
                kind,
                mood,
                ..
            }) => Shown::Line {
                who: who.clone(),
                name: translated(name),
                text: translated(text),
                kind: *kind,
                mood: mood.clone(),
            },
            Some(Step::Choice {
                prompt, options, ..
            }) => Shown::Choice {
                prompt: prompt.as_deref().map(translated),
                options: options
                    .iter()
                    .map(|o| {
                        (
                            translated(&o.label),
                            o.cond.iter().all(|c| c.holds(vars.get(&c.var))),
                        )
                    })
                    .collect(),
            },
            _ => Shown::End,
        }
    }

    /// On from a line (a choice waits for [`Self::choose`]).
    pub fn advance(&mut self, lib: &Library, vars: &mut Vars) {
        if matches!(
            lib.guiones[self.file].scenes[self.scene]
                .steps
                .get(self.step),
            Some(Step::Line { .. })
        ) {
            self.step += 1;
            self.settle(lib, vars);
        }
    }

    /// Takes option `index` of the choice shown (if it can be taken).
    pub fn choose(&mut self, lib: &Library, vars: &mut Vars, index: usize) -> bool {
        let Some(Step::Choice { options, .. }) = lib.guiones[self.file].scenes[self.scene]
            .steps
            .get(self.step)
        else {
            return false;
        };
        let Some(o) = options.get(index) else {
            return false;
        };
        if !o.cond.iter().all(|c| c.holds(vars.get(&c.var))) {
            return false;
        }
        for e in &o.effects {
            vars.apply(e);
        }
        match o.to.clone() {
            Some(to) => self.goto(lib, &to, vars),
            None => self.step += 1,
        }
        self.settle(lib, vars);
        true
    }

    pub fn is_over(&self) -> bool {
        self.finished
    }

    /// The staging run through since the last call (`@fondo`, `@izq`, `@sello`…), in order.
    pub fn take_staging(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.staged)
    }
}

// ------------------------------------------------------------------ the visual novel
/// A node id the story accepts from a scene id and a counter.
fn node_id(scene: &str, n: usize) -> String {
    format!("g_{}_{n}", scene.replace('.', "__"))
}

/// The first visual-novel node of a guion scene (`file.label`).
pub fn vn_start_node(scene: &str) -> String {
    node_id(scene, 0)
}

/// How [`compile_vn`] stages the visual novel: the canvas backgrounds and CGs fill, where sprites
/// stand (one slot each, in turn, by speaker id) and how big, and the tags every node carries.
#[derive(Clone, Debug, PartialEq)]
pub struct VnLayout {
    pub canvas: (u32, u32),
    pub sprite_slots: Vec<(i32, i32)>,
    pub sprite_size: (u32, u32),
    pub tags: Vec<String>,
}

impl Default for VnLayout {
    /// A 1920×1080 story with two sprite slots, right then left.
    fn default() -> Self {
        Self {
            canvas: (1920, 1080),
            sprite_slots: vec![(1260, 20), (90, 20)],
            sprite_size: (580, 1010),
            tags: vec!["guion".into()],
        }
    }
}

/// The visual novel of a library's `vn` scenes: velvet-story nodes (JSON objects), and the
/// characters (id → name) and variables they use, for the game to add to its story.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VnStory {
    pub nodes: Vec<serde_json::Value>,
    pub characters: BTreeMap<String, String>,
    pub variables: BTreeSet<String>,
}

/// The `vn` scenes of a library as velvet-story nodes, staged by `layout`. `default_room` is the
/// room of a guion with no `sala`; `world_entry` is what a finished scene's ending carries (in a
/// game with a 3D world, where it returns to: `{ "world": ..., "spawn": ... }`).
///
/// Every scene a `vn` scene can lead to (running on, jumping, choosing, `@vn`) is compiled too,
/// as are the scenes the 3D conversations hand over to with `@vn`. Each scene becomes one node
/// per line; a choice hangs on the line before it (or on a node of its own for its prompt); an
/// `@si` becomes a crossroads (`pass` node) the story goes through without showing; a scene with
/// no lines still gets a node (a crossroads) so jumps to it land. `-> fin` and the end of the road
/// lead to the guion's closing node, which returns to the world.
pub fn compile_vn(
    lib: &Library,
    default_room: &str,
    world_entry: serde_json::Value,
    layout: &VnLayout,
) -> VnStory {
    use serde_json::json;
    let world = world_entry;
    let tags = &layout.tags;
    let (canvas_w, canvas_h) = layout.canvas;
    let (sprite_w, sprite_h) = layout.sprite_size;
    let mut nodes = Vec::new();
    let mut characters = BTreeMap::new();
    let mut vars = BTreeSet::new();

    // Which scenes play in the visual novel: those marked `vn`, those `@vn` hands over to, and
    // everything they lead to.
    let mut todo: Vec<(usize, usize)> = Vec::new();
    for (f, g) in lib.guiones.iter().enumerate() {
        for (s, scene) in g.scenes.iter().enumerate() {
            if scene.attrs.vn {
                todo.push((f, s));
            }
            for step in &scene.steps {
                if let Step::Vn { to, .. } = step {
                    if let Some(at) = lib.find(to, Some(f)) {
                        todo.push(at);
                    }
                }
            }
        }
    }
    let mut compiled: BTreeSet<(usize, usize)> = BTreeSet::new();
    while let Some((f, s)) = todo.pop() {
        if !compiled.insert((f, s)) {
            continue;
        }
        let g = &lib.guiones[f];
        let scene = &g.scenes[s];
        for step in &scene.steps {
            let targets: Vec<&str> = match step {
                Step::Jump { to, .. } | Step::Vn { to, .. } => vec![to.as_str()],
                Step::Choice { options, .. } => {
                    options.iter().filter_map(|o| o.to.as_deref()).collect()
                }
                _ => Vec::new(),
            };
            for t in targets {
                if slug(t) != "fin" {
                    if let Some(at) = lib.find(t, Some(f)) {
                        todo.push(at);
                    }
                }
            }
        }
        if g.scenes.get(s + 1).is_some_and(|next| next.follows_on()) {
            todo.push((f, s + 1));
        }
    }

    let conds = |cs: &[Cond], vars: &mut BTreeSet<String>| -> serde_json::Value {
        cs.iter()
            .map(|c| {
                vars.insert(c.var.clone());
                json!({ "var": c.var, "op": c.json_op(), "value": c.value })
            })
            .collect()
    };
    let effects = |es: &[Effect], vars: &mut BTreeSet<String>| -> Vec<serde_json::Value> {
        es.iter()
            .map(|e| {
                vars.insert(e.var.clone());
                let (op, value) = match e.op {
                    EffOp::Set => ("set", e.value),
                    EffOp::Add => ("add", e.value),
                    EffOp::Sub => ("subtract", e.value),
                };
                json!({ "op": op, "var": e.var, "value": value })
            })
            .collect()
    };

    /// How a node goes on.
    #[derive(Clone)]
    enum Then {
        /// To the next node of the scene, else on into the next scene, else to the end.
        Next,
        Goto(String),
        End,
    }
    const AFTER: &str = "__after__";

    for (f, g) in lib.guiones.iter().enumerate() {
        let end_id = format!("g_{}__fin", g.id);
        let mut end_used = false;
        let mut end_room = g.sala.clone().unwrap_or_else(|| default_room.to_owned());
        // A scene's first node: where jumps into it land.
        let entry = |to: &str| -> Option<String> {
            if slug(to) == "fin" {
                return None;
            }
            lib.find(to, Some(f))
                .map(|(tf, ts)| node_id(&lib.guiones[tf].scenes[ts].id, 0))
        };
        // A scene without its own `@sala` stays where the scenes before it (in the file) left off,
        // not back in the file's room: a branch of the prologue's bedroom is still the bedroom.
        let mut inherited = g.sala.clone().unwrap_or_else(|| default_room.to_owned());
        let rooms: Vec<String> = g
            .scenes
            .iter()
            .map(|scene| {
                for step in &scene.steps {
                    if let Step::Stage { cmd, arg } = step {
                        if cmd == "sala" {
                            inherited = slug(arg);
                        }
                    }
                }
                inherited.clone()
            })
            .collect();
        let starts: Vec<String> = g
            .scenes
            .iter()
            .enumerate()
            .map(|(s, scene)| {
                let own = scene
                    .steps
                    .iter()
                    .any(|step| matches!(step, Step::Stage { cmd, .. } if cmd == "sala"));
                if own || s == 0 {
                    g.sala.clone().unwrap_or_else(|| default_room.to_owned())
                } else {
                    rooms[s - 1].clone()
                }
            })
            .collect();
        // The picture likewise: a scene with no room of its own that opens with words (no picture
        // of its own before its first line) goes on with the picture the scene before it in the
        // file ended on, not with a bare room: the prologue's branches in the dream keep the dream.
        let mut ended: Picture = Default::default();
        let pictures: Vec<Picture> = g
            .scenes
            .iter()
            .enumerate()
            .map(|(s, scene)| {
                let own_room = scene
                    .steps
                    .iter()
                    .any(|step| matches!(step, Step::Stage { cmd, .. } if cmd == "sala"));
                let opens_with_words = scene
                    .steps
                    .iter()
                    .find(|step| match step {
                        Step::Line { .. } => true,
                        Step::Stage { cmd, .. } => PICTURE_DIRECTIVES.contains(&cmd.as_str()),
                        _ => false,
                    })
                    .is_some_and(|step| matches!(step, Step::Line { .. }));
                let start = if s > 0 && !own_room && opens_with_words {
                    ended.clone()
                } else {
                    Picture::default()
                };
                let mut now = start.clone();
                for step in &scene.steps {
                    if let Step::Stage { cmd, arg } = step {
                        stage_picture(lib, &mut now, cmd, arg);
                    }
                }
                ended = now;
                start
            })
            .collect();
        for (s, scene) in g.scenes.iter().enumerate() {
            if !compiled.contains(&(f, s)) {
                continue;
            }
            let mut room = starts[s].clone();
            let (mut background, mut cg, mut sprites) = pictures[s].clone();
            // `@titulo` shows on the next line said; `@teclas` on every line until the next one
            // (`@teclas` alone, or `@teclas -`, takes them away).
            let mut title: Option<String> = None;
            let mut keys: Option<String> = None;
            let mut pending: Vec<serde_json::Value> = Vec::new();
            let mut items: Vec<(serde_json::Value, Then)> = Vec::new();
            let description = format!("Guion {}: {}", g.id, scene.title);
            let crossroads = |room: &str, description: &str| {
                json!({
                    "room": room, "speaker": "narrator", "text": "·",
                    "visual": { "layers": [], "description": description, "shot": "Cruce del guion (no se ve)", "status": "optional" },
                    "show_hud": false, "tags": tags, "pass": true,
                })
            };
            let push = |items: &mut Vec<(serde_json::Value, Then)>,
                        mut node: serde_json::Value,
                        then: Then,
                        pending: &mut Vec<serde_json::Value>| {
                if !pending.is_empty() {
                    node["on_enter"] = serde_json::Value::Array(std::mem::take(pending));
                }
                items.push((node, then));
            };
            for step in &scene.steps {
                match step {
                    Step::Stage { cmd, arg } => match cmd.as_str() {
                        "sala" => room = slug(arg),
                        picture if PICTURE_DIRECTIVES.contains(&picture) => {
                            let mut now =
                                (background.take(), cg.take(), std::mem::take(&mut sprites));
                            stage_picture(lib, &mut now, picture, arg);
                            (background, cg, sprites) = now;
                        }
                        "titulo" if !arg.trim().is_empty() => title = Some(arg.trim().to_owned()),
                        "teclas" => {
                            keys = Some(arg.trim().to_owned()).filter(|k| !k.is_empty() && k != "-")
                        }
                        _ => {}
                    },
                    Step::Set { effects: es } => pending.extend(effects(es, &mut vars)),
                    Step::Line {
                        who,
                        name,
                        text,
                        kind,
                        mood,
                        ..
                    } => {
                        if !name.is_empty() && who != "narrator" {
                            characters
                                .entry(who.clone())
                                .or_insert_with(|| name.clone());
                        }
                        let mut layers = Vec::new();
                        if let Some(bg) = &background {
                            layers.push(json!({ "asset": bg, "x": 0, "y": 0, "width": canvas_w, "height": canvas_h, "opacity": 1.0 }));
                        }
                        for (k, (_, image)) in sprites.iter().enumerate() {
                            let (x, y) = layout
                                .sprite_slots
                                .get(k % layout.sprite_slots.len().max(1))
                                .copied()
                                .unwrap_or((0, 0));
                            layers.push(json!({ "asset": image, "x": x, "y": y, "width": sprite_w, "height": sprite_h, "opacity": 1.0 }));
                        }
                        if let Some(c) = &cg {
                            layers.push(json!({ "asset": c, "x": 0, "y": 0, "width": canvas_w, "height": canvas_h, "opacity": 1.0 }));
                        }
                        let status = if layers_empty(&background, &cg, &sprites) {
                            "needed"
                        } else {
                            "reference"
                        };
                        let mut node = json!({
                            "room": room, "speaker": who, "text": text,
                            "visual": { "layers": layers, "description": description, "shot": "Guion", "status": status },
                            "show_hud": false, "tags": tags,
                        });
                        if let Some(t) = title.take() {
                            node["title_card"] = json!(t);
                        }
                        // How the speaker looks saying it (`**Chiquita (enfadada):**`): her gesture.
                        if let Some(m) = mood {
                            node["mood"] = json!(m);
                        }
                        if let Some(k) = &keys {
                            node["keys"] = json!(k);
                        }
                        // A sound (`*dos golpes en la puerta*`) is told apart from narration.
                        if matches!(kind, LineKind::Sound) {
                            node["sound"] = json!(true);
                        }
                        push(&mut items, node, Then::Next, &mut pending);
                    }
                    Step::Choice {
                        prompt, options, ..
                    } => {
                        // On the line just said, if it has no choice yet and no prompt is asked;
                        // else on a node of its own that asks.
                        let reuse = prompt.is_none()
                            && pending.is_empty()
                            && items.last().is_some_and(|(n, _)| {
                                n.get("choices").is_none() && n.get("pass").is_none()
                            });
                        if !reuse {
                            let node = json!({
                                "room": room, "speaker": "narrator", "text": prompt.clone().unwrap_or_else(|| "…".into()),
                                "visual": { "layers": [], "description": description, "shot": "Guion", "status": "needed" },
                                "show_hud": false, "tags": tags,
                            });
                            push(&mut items, node, Then::Next, &mut pending);
                        }
                        let choices: Vec<serde_json::Value> = options
                            .iter()
                            .enumerate()
                            .map(|(k, o)| {
                                let to = match o.to.as_deref() {
                                    None => AFTER.to_owned(),
                                    Some(t) => entry(t).unwrap_or_else(|| {
                                        end_used = true;
                                        end_id.clone()
                                    }),
                                };
                                json!({ "id": format!("o{k}"), "label": o.label, "to": to, "enabled_if": conds(&o.cond, &mut vars), "effects": effects(&o.effects, &mut vars) })
                            })
                            .collect();
                        items.last_mut().unwrap().0["choices"] = serde_json::Value::Array(choices);
                    }
                    Step::Jump { to, cond, .. } if !cond.is_empty() => {
                        let target = entry(to).unwrap_or_else(|| {
                            end_used = true;
                            end_id.clone()
                        });
                        let mut node = crossroads(&room, &description);
                        node["branches"] =
                            json!([{ "when": conds(cond, &mut vars), "to": target }]);
                        push(&mut items, node, Then::Next, &mut pending);
                    }
                    Step::Jump { to, .. } | Step::Vn { to, .. } => {
                        let then = entry(to).map(Then::Goto).unwrap_or(Then::End);
                        if pending.is_empty() && !items.is_empty() {
                            items.last_mut().unwrap().1 = then;
                        } else {
                            let node = crossroads(&room, &description);
                            push(&mut items, node, then, &mut pending);
                        }
                    }
                    Step::End => {
                        if pending.is_empty() && !items.is_empty() {
                            items.last_mut().unwrap().1 = Then::End;
                        } else {
                            let node = crossroads(&room, &description);
                            push(&mut items, node, Then::End, &mut pending);
                        }
                    }
                }
            }
            // Variables set at the very end, and a scene with no lines: a crossroads carries them.
            if !pending.is_empty() || items.is_empty() {
                let node = crossroads(&room, &description);
                push(&mut items, node, Then::Next, &mut pending);
            }
            end_room = room.clone();

            // Link: ids, and where each node goes on to.
            let follow = g
                .scenes
                .get(s + 1)
                .filter(|n| n.follows_on() && compiled.contains(&(f, s + 1)))
                .map(|n| node_id(&n.id, 0));
            let count = items.len();
            for (k, (node, then)) in items.iter_mut().enumerate() {
                let on: Option<String> = match then {
                    Then::Goto(t) => Some(t.clone()),
                    Then::End => None,
                    Then::Next => {
                        if k + 1 < count {
                            Some(node_id(&scene.id, k + 1))
                        } else {
                            follow.clone()
                        }
                    }
                };
                node["id"] = json!(node_id(&scene.id, k));
                let is_pass = node.get("pass").is_some();
                if let Some(choices) = node.get_mut("choices").and_then(|c| c.as_array_mut()) {
                    for c in choices {
                        if c["to"] == AFTER {
                            c["to"] = json!(on.clone().unwrap_or_else(|| {
                                end_used = true;
                                end_id.clone()
                            }));
                        }
                    }
                } else if is_pass {
                    node["next"] = json!(on.unwrap_or_else(|| {
                        end_used = true;
                        end_id.clone()
                    }));
                } else {
                    match on {
                        Some(t) => node["next"] = json!(t),
                        None => {
                            node["ending"] = json!(true);
                            node["world_entry"] = world.clone();
                        }
                    }
                }
            }
            nodes.extend(items.into_iter().map(|(n, _)| n));
        }
        if end_used {
            nodes.push(json!({
                "id": end_id, "room": end_room, "speaker": "narrator", "text": "…",
                "visual": { "layers": [], "description": format!("Guion {}: cierre", g.id), "shot": "Guion", "status": "optional" },
                "show_hud": false, "tags": tags, "ending": true, "world_entry": world.clone(),
            }));
        }
    }
    VnStory {
        nodes,
        characters,
        variables: vars,
    }
}

/// What a visual-novel scene shows behind its words: its background, its CG, its sprites (by
/// speaker).
type Picture = (Option<String>, Option<String>, BTreeMap<String, String>);

/// The stage directives that change the picture.
const PICTURE_DIRECTIVES: [&str; 4] = ["fondo", "cg", "quitar", "sprite"];

/// A stage directive's change to the picture (`@fondo`, `@cg`, `@quitar`, `@sprite`).
fn stage_picture(lib: &Library, picture: &mut Picture, cmd: &str, arg: &str) {
    let (background, cg, sprites) = picture;
    match cmd {
        "fondo" => *background = Some(arg.to_owned()),
        "cg" => *cg = Some(arg.to_owned()),
        "quitar" => {
            if slug(arg) == "cg" || arg.is_empty() {
                *cg = None;
            } else {
                sprites.remove(&lib.cast.speaker_id(arg));
            }
        }
        "sprite" => {
            if let Some((who, image)) = arg.split_once(char::is_whitespace) {
                sprites.insert(lib.cast.speaker_id(who), image.trim().to_owned());
            }
        }
        _ => {}
    }
}

fn layers_empty(
    bg: &Option<String>,
    cg: &Option<String>,
    sprites: &BTreeMap<String, String>,
) -> bool {
    bg.is_none() && cg.is_none() && sprites.is_empty()
}

// ------------------------------------------------------------------ tests
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SAMPLE: &str = "---\nid: prueba\nsala: posada\n---\n# Prueba\n> nota\n\n## saludo {quien=ana}\n@si ana_trust >= 3 -> carino\n**Ana:** —¿Qué te pongo, viajera?\n*clic.*\n(Tengo que decírselo.)\nLa capitana se sentó.\n? ¿Qué le pide?\n- Un té -> te {ana_trust +1}\n- Hablar de Teo [ana_trust >= 2] -> teo\n- Nada\n**Ana:** —Como quieras.\n@fin\n\n## te\n**Ana:** —Enseguida.\n\n## despues\n**Ana:** —Aquí tienes.\n\n## teo\n**Ana:** —Teo está bien.\n\n## carino {quien=ana, si=ana_trust>=3}\n**Ana:** —Capitana...\n\n## escena {vn}\n@fondo bg/archivo\n**Ana:** —En el archivo.\n@var visto +1\n**La Capitana:** —Bien.\n";

    fn cast() -> Cast {
        Cast::new()
            .alias("captain", &["capitana", "la capitana"])
            .thinker("captain", "Capitana")
    }

    fn lib() -> Library {
        let mut lib = Library::new(cast());
        lib.add(SAMPLE, Path::new("prueba.md"));
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        lib
    }

    #[test]
    fn a_vn_title_card_goes_on_the_next_line_and_keys_stay_until_taken_away() {
        let mut lib = Library::new(cast());
        lib.add(
            "## pizarra {vn}
@titulo PRÓLOGO · 7 | Lecciones
@teclas W A S D · caminar | F · usar
**Ana:** —Atiende.
**Ana:** —Caminas así.
@teclas -
**Ana:** —Ya.
",
            Path::new("l.md"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let vn = compile_vn(&lib, "sala", serde_json::Value::Null, &VnLayout::default());
        let lines: Vec<&serde_json::Value> =
            vn.nodes.iter().filter(|n| n["speaker"] == "ana").collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["title_card"], "PRÓLOGO · 7 | Lecciones");
        assert!(lines[1].get("title_card").is_none());
        assert_eq!(lines[0]["keys"], "W A S D · caminar | F · usar");
        assert_eq!(lines[1]["keys"], "W A S D · caminar | F · usar");
        assert!(lines[2].get("keys").is_none());
    }

    #[test]
    fn the_stage_is_handed_over_in_order_up_to_each_line() {
        let mut lib = Library::new(cast());
        lib.add(
            "## llegada {en=puerta}
@titulo LA POSADA | DE NOCHE
@fondo posada/entrada
@izq Ana posada/ana_normal
**Ana:** —Pasa.
@pose Ana picara
@sello ¡HOLA! | oro
**Ana:** —Bienvenida.
",
            Path::new("posada.md"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let mut vars = Vars::default();
        let at = lib.entry(None, Some("puerta"), &vars).unwrap();
        let mut c = Conversation::start(&lib, at, &mut vars);
        let first: Vec<String> = c
            .take_staging()
            .into_iter()
            .map(|(k, a)| format!("{k} {a}"))
            .collect();
        assert_eq!(
            first,
            [
                "titulo LA POSADA | DE NOCHE",
                "fondo posada/entrada",
                "izq Ana posada/ana_normal"
            ]
        );
        assert!(c.take_staging().is_empty());
        c.advance(&lib, &mut vars);
        let next: Vec<String> = c
            .take_staging()
            .into_iter()
            .map(|(k, a)| format!("{k} {a}"))
            .collect();
        assert_eq!(next, ["pose Ana picara", "sello ¡HOLA! | oro"]);
        assert!(
            matches!(c.shown(&lib, &vars), Shown::Line { ref text, .. } if text == "Bienvenida.")
        );
    }

    #[test]
    fn a_guion_reads_like_a_draft() {
        let lib = lib();
        let g = &lib.guiones[0];
        assert_eq!(g.id, "prueba");
        assert_eq!(g.title, "Prueba");
        assert_eq!(g.sala.as_deref(), Some("posada"));
        let s = &g.scenes[0];
        assert_eq!(s.id, "prueba.saludo");
        assert_eq!(s.attrs.quien.as_deref(), Some("ana"));
        assert!(
            matches!(&s.steps[1], Step::Line { who, text, kind: LineKind::Dialogue, .. } if who == "ana" && text == "¿Qué te pongo, viajera?")
        );
        assert!(
            matches!(&s.steps[2], Step::Line { kind: LineKind::Sound, text, .. } if text == "clic.")
        );
        assert!(
            matches!(&s.steps[3], Step::Line { kind: LineKind::Thought, who, name, .. } if who == "captain_thought" && name == "Capitana")
        );
        assert!(matches!(
            &s.steps[4],
            Step::Line {
                kind: LineKind::Narration,
                ..
            }
        ));
        let Step::Choice {
            options, prompt, ..
        } = &s.steps[5]
        else {
            panic!("{:?}", s.steps[5])
        };
        assert_eq!(prompt.as_deref(), Some("¿Qué le pide?"));
        assert_eq!(options.len(), 3);
        assert_eq!(
            options[0].effects,
            vec![Effect {
                var: "ana_trust".into(),
                op: EffOp::Add,
                value: 1
            }]
        );
        assert_eq!(
            options[1].cond,
            vec![Cond {
                var: "ana_trust".into(),
                op: Op::Gte,
                value: 2
            }]
        );
        assert_eq!(options[2].to, None);
    }

    #[test]
    fn the_cast_names_speakers_and_the_thinker() {
        let cast = cast();
        assert_eq!(cast.speaker_id("La Capitana"), "captain");
        assert_eq!(cast.speaker_id("Narrador"), "narrator");
        assert_eq!(
            cast.speaker_id("Ana María"),
            "ana_maria",
            "a name not listed is its own id"
        );
        // Without a thinker a thought is nobody's in particular.
        let (g, problems) = parse("(Hmm.)\n", Path::new("t.md"), &Cast::new());
        assert!(problems.is_empty(), "{problems:?}");
        assert!(
            matches!(&g.scenes[0].steps[0], Step::Line { who, name, kind: LineKind::Thought, .. } if who == "thought" && name.is_empty())
        );
    }

    #[test]
    fn a_speaker_may_say_how_they_look() {
        let (g, problems) = parse(
            "## a\n**Ana (Enfadada):** —¡Capitana!\n**Teo:** —Hola.\n",
            Path::new("m.md"),
            &cast(),
        );
        assert!(problems.is_empty(), "{problems:?}");
        assert!(
            matches!(&g.scenes[0].steps[0], Step::Line { who, name, mood: Some(m), .. } if who == "ana" && name == "Ana" && m == "enfadada")
        );
        assert!(matches!(
            &g.scenes[0].steps[1],
            Step::Line { mood: None, .. }
        ));
    }

    #[test]
    fn portraits_fall_back_to_the_normal_face_and_thoughts_show_the_thinker() {
        let portraits = Portraits::parse(
            r#"{ "ana": { "normal": { "imagen": "retratos/ana", "recorte": [0, 0, 10, 10] },
                          "enfadada": { "imagen": "retratos/ana_enfadada", "recorte": [1, 1, 5, 5] },
                          "enojada": "enfadada" },
                 "captain": { "normal": { "imagen": "retratos/capitana", "recorte": [0, 0, 8, 8] } } }"#,
        )
        .unwrap();
        assert_eq!(portraits.entries().len(), 4);
        assert_eq!(
            portraits.find("ana", Some("Enojada")).unwrap().image,
            "retratos/ana_enfadada",
            "a synonym names another mood"
        );
        assert_eq!(
            portraits.find("ana", Some("furiosa")),
            portraits.find("ana", None)
        );
        assert_eq!(
            portraits.find("captain_thought", None).unwrap().crop,
            [0, 0, 8, 8]
        );
        assert_eq!(portraits.find("teo", None), None);
    }

    #[test]
    fn mistakes_are_reported_with_their_line() {
        let (_, problems) = parse(
            "## a\n- suelta\n@nada\n? \n- ir -> nowhere\n",
            Path::new("x.md"),
            &Cast::new(),
        );
        assert!(
            problems
                .iter()
                .any(|p| p.line == 3 && p.message.contains("@nada")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.message.contains("nowhere")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_conversation_plays_choices_and_variables() {
        let lib = lib();
        let mut vars = Vars::default();
        let at = lib.entry(Some("ana"), None, &vars).unwrap();
        let mut c = Conversation::start(&lib, at, &mut vars);
        assert!(
            matches!(c.shown(&lib, &vars), Shown::Line { ref text, .. } if text.starts_with("¿Qué"))
        );
        // The line, the sound, the thought, the narration: then the choice.
        for _ in 0..4 {
            c.advance(&lib, &mut vars);
        }
        let Shown::Choice { options, .. } = c.shown(&lib, &vars) else {
            panic!()
        };
        assert_eq!(options[1], ("Hablar de Teo".to_owned(), false));
        assert!(
            !c.choose(&lib, &mut vars, 1),
            "a closed option cannot be taken"
        );
        assert!(c.choose(&lib, &mut vars, 0));
        assert_eq!(vars.get("ana_trust"), 1);
        assert!(
            matches!(c.shown(&lib, &vars), Shown::Line { ref text, .. } if text == "Enseguida.")
        );
        // Off the end of "te" into "despues" (no jump leads there: it follows on), then out: "teo"
        // is a branch of its own.
        c.advance(&lib, &mut vars);
        assert!(
            matches!(c.shown(&lib, &vars), Shown::Line { ref text, .. } if text == "Aquí tienes.")
        );
        c.advance(&lib, &mut vars);
        assert_eq!(c.shown(&lib, &vars), Shown::End);
        assert!(vars.seen("prueba.saludo") && vars.seen("prueba.despues"));
        // With more trust she greets otherwise (the @si jumps).
        vars.values.insert("ana_trust".into(), 3);
        let mut c = Conversation::start(
            &lib,
            lib.entry(Some("ana"), None, &vars).unwrap(),
            &mut vars,
        );
        assert!(
            matches!(c.shown(&lib, &vars), Shown::Line { ref text, .. } if text == "Capitana...")
        );
        c.advance(&lib, &mut vars);
        assert_eq!(c.shown(&lib, &vars), Shown::End);
    }

    #[test]
    fn vn_crossroads_endings_and_bare_scenes_link_up() {
        let src = "---
id: x
---
## cita {vn}
@si visto >= 1 -> repetida
**Ana:** —Hola.
?
- Irse -> fin
- Quedarse
@var visto +1

## repetida
@salta final

## final
**Ana:** —Otra vez.
";
        let mut lib = Library::new(Cast::new());
        lib.add(src, Path::new("x.md"));
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let VnStory {
            nodes,
            variables: vars,
            ..
        } = compile_vn(
            &lib,
            "sala",
            json!({ "world": "w", "spawn": "s" }),
            &VnLayout::default(),
        );
        let by_id: BTreeMap<String, &serde_json::Value> = nodes
            .iter()
            .map(|n| (n["id"].as_str().unwrap().to_owned(), n))
            .collect();
        let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
        assert_eq!(nodes.len(), 6, "{ids:?}");
        // The @si: a crossroads to "repetida", else on to the line.
        let cross = by_id["g_x__cita_0"];
        assert_eq!(cross["pass"], true);
        assert_eq!(cross["branches"][0]["to"], "g_x__repetida_0");
        assert_eq!(cross["branches"][0]["when"][0]["var"], "visto");
        assert_eq!(cross["next"], "g_x__cita_1");
        // The choice: "Irse" to the closing node; "Quedarse" on to what follows (the variable).
        let line = by_id["g_x__cita_1"];
        assert_eq!(line["choices"][0]["to"], "g_x__fin");
        assert_eq!(line["choices"][1]["to"], "g_x__cita_2");
        let carry = by_id["g_x__cita_2"];
        assert_eq!(carry["on_enter"][0]["var"], "visto");
        assert_eq!(
            carry["next"], "g_x__fin",
            "a branch scene below is not run into"
        );
        // A scene with no lines still has a node, leading on.
        assert_eq!(by_id["g_x__repetida_0"]["next"], "g_x__final_0");
        assert_eq!(by_id["g_x__final_0"]["ending"], true);
        assert_eq!(by_id["g_x__final_0"]["world_entry"]["spawn"], "s");
        assert_eq!(by_id["g_x__fin"]["ending"], true);
        assert!(vars.contains("visto"));
        // Every jump lands on a node that exists.
        for n in &nodes {
            let mut targets: Vec<&str> = Vec::new();
            targets.extend(n.get("next").and_then(|t| t.as_str()));
            for list in ["choices", "branches"] {
                for c in n.get(list).and_then(|c| c.as_array()).into_iter().flatten() {
                    targets.extend(c["to"].as_str());
                }
            }
            for t in targets {
                assert!(by_id.contains_key(t), "{} -> {t}", n["id"]);
            }
        }
    }

    #[test]
    fn vn_scenes_compile_to_story_nodes() {
        let lib = lib();
        let VnStory {
            nodes,
            characters,
            variables,
        } = compile_vn(
            &lib,
            "posada",
            json!({ "world": "pueblo", "spawn": "puerta" }),
            &VnLayout::default(),
        );
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["id"], "g_prueba__escena_0");
        assert_eq!(nodes[0]["next"], "g_prueba__escena_1");
        assert_eq!(nodes[0]["visual"]["layers"][0]["asset"], "bg/archivo");
        assert_eq!(nodes[0]["visual"]["layers"][0]["width"], 1920);
        assert_eq!(nodes[0]["tags"], json!(["guion"]));
        assert_eq!(nodes[1]["speaker"], "captain");
        assert_eq!(nodes[1]["on_enter"][0]["var"], "visto");
        assert_eq!(nodes[1]["ending"], true);
        assert_eq!(characters.get("ana").map(String::as_str), Some("Ana"));
        assert!(variables.contains("visto"));
    }

    #[test]
    fn a_scene_without_sala_stays_in_the_room_before_it() {
        let mut lib = Library::new(cast());
        lib.add(
            "---
id: pro
sala: banquete
---
## a {vn}
@sala alcoba
La alcoba.
? ¿Qué?
- Leer -> b

## b
La carta.
",
            Path::new("pro.md"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let vn = compile_vn(&lib, "sala", serde_json::Value::Null, &VnLayout::default());
        let b = vn
            .nodes
            .iter()
            .find(|n| n["text"] == "La carta.")
            .expect("b's line");
        assert_eq!(b["room"], "alcoba");
    }

    #[test]
    fn what_a_scene_can_set_follows_its_jumps_and_options() {
        let mut lib = Library::new(cast());
        lib.add(
            "---
id: c
---
## mesa {en=mesa, una_vez}
? ¿Qué?
- Cazar -> caza {caza}
- Nada

## caza
@salta cierre

## cierre
@var hecho

## otra {en=mesa}
@var otra
",
            Path::new("c.md"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let at = lib.find("c.mesa", None).expect("mesa");
        let set = lib.vars_set_from(at);
        assert!(set.contains("caza") && set.contains("hecho"), "{set:?}");
        assert!(!set.contains("otra"), "{set:?}");
    }

    #[test]
    fn a_scene_that_opens_with_words_keeps_the_picture_before_it() {
        let mut lib = Library::new(cast());
        lib.add(
            "---
id: pro
sala: banquete
---
## a {vn}
@sala sueno
@cg cg/trono
La Reina en su trono.
? ¿Qué?
- Hablar -> b
- Callar -> c

## b
La Reina habla.

## c
@cg cg/espalda
La Reina se da la vuelta.

## d
@sala alcoba
La alcoba.

## e
La alcoba sigue.
",
            Path::new("pro.md"),
        );
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let vn = compile_vn(&lib, "sala", serde_json::Value::Null, &VnLayout::default());
        let cg_of = |text: &str| {
            let node = vn.nodes.iter().find(|n| n["text"] == text).expect(text);
            node["visual"]["layers"]
                .as_array()
                .and_then(|layers| layers.last())
                .map(|layer| layer["asset"].as_str().unwrap_or("").to_owned())
        };
        // The branch that opens with words goes on in the dream's picture...
        assert_eq!(cg_of("La Reina habla.").as_deref(), Some("cg/trono"));
        // ...one with a picture of its own shows it...
        assert_eq!(
            cg_of("La Reina se da la vuelta.").as_deref(),
            Some("cg/espalda")
        );
        // ...and a new room starts bare, as does what follows it without a picture.
        assert_eq!(cg_of("La alcoba."), None);
        assert_eq!(cg_of("La alcoba sigue."), None);
    }

    #[test]
    fn staging_puts_sprites_in_the_layouts_slots() {
        let mut lib = Library::new(cast());
        lib.add("## s {vn}\n@fondo bg/posada\n@sprite Ana ana/feliz\n@sprite capitana capitana/seria\n**Ana:** —Hola.\n@quitar ana\n**Ana:** —Adiós.\n", Path::new("s.md"));
        assert!(lib.problems.is_empty(), "{:?}", lib.problems);
        let layout = VnLayout {
            canvas: (1280, 720),
            sprite_slots: vec![(100, 0), (700, 10)],
            sprite_size: (400, 700),
            tags: vec!["mio".into()],
        };
        let vn = compile_vn(&lib, "sala", serde_json::Value::Null, &layout);
        let layers = vn.nodes[0]["visual"]["layers"].as_array().unwrap();
        assert_eq!(layers[0]["asset"], "bg/posada");
        assert_eq!(layers[0]["width"], 1280);
        // Sprites by speaker id: Ana in the first slot, the captain (an alias) in the second.
        assert_eq!(
            (layers[1]["asset"].as_str(), layers[1]["x"].as_i64()),
            (Some("ana/feliz"), Some(100))
        );
        assert_eq!(
            (
                layers[2]["asset"].as_str(),
                layers[2]["y"].as_i64(),
                layers[2]["height"].as_u64()
            ),
            (Some("capitana/seria"), Some(10), Some(700))
        );
        assert_eq!(
            vn.nodes[1]["visual"]["layers"].as_array().unwrap().len(),
            2,
            "@quitar takes her sprite off"
        );
        assert_eq!(vn.nodes[0]["tags"], json!(["mio"]));
    }

    #[test]
    fn the_front_matter_keeps_its_other_keys_as_metadata_in_file_order() {
        let text = "---\nid: escena\nzeta: último\ncategoria: escena\nparticipantes: Lira, Nyxara, Nemi\nilustracion: lira_bano\nsala: posada\nTitulo: La escena\n---\n## inicio\n**Ana:** —Hola.\n";
        let (g, problems) = parse(text, Path::new("escena.md"), &Cast::new());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(g.id, "escena");
        assert_eq!(g.sala.as_deref(), Some("posada"));
        assert_eq!(g.title, "La escena");
        assert_eq!(
            g.meta,
            vec![
                ("zeta".to_owned(), "último".to_owned()),
                ("categoria".to_owned(), "escena".to_owned()),
                ("participantes".to_owned(), "Lira, Nyxara, Nemi".to_owned()),
                ("ilustracion".to_owned(), "lira_bano".to_owned()),
            ]
        );
        assert_eq!(g.meta_value("participantes"), Some("Lira, Nyxara, Nemi"));
        assert_eq!(
            g.meta_value("sala"),
            None,
            "a known key is a field, not metadata"
        );
        assert_eq!(g.meta_value("nada"), None);
    }

    #[test]
    fn a_metadata_value_keeps_its_colons_and_may_be_empty() {
        let text = "---\nhora: 10:30\nvacio:\n---\n## inicio\n**Ana:** —Hola.\n";
        let (g, problems) = parse(text, Path::new("hora.md"), &Cast::new());
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(g.meta_value("hora"), Some("10:30"));
        assert_eq!(g.meta_value("vacio"), Some(""));
        let (plain, _) = parse(
            "## inicio\n**Ana:** —Hola.\n",
            Path::new("sin.md"),
            &Cast::new(),
        );
        assert!(plain.meta.is_empty(), "no front matter, no metadata");
    }

    #[test]
    fn metadata_keys_follow_the_name_rule_and_are_given_once() {
        let text = "---\nCategoria: x\n2da: x\nmi clave: x\n: x\nok_1: uno\nok_1: dos\nsala: a\nsala: b\ntitulo: uno\ntitle: dos\n---\n## inicio\n**Ana:** —Hola.\n";
        let (g, problems) = parse(text, Path::new("cabeceras.md"), &Cast::new());
        let lines: Vec<usize> = problems.iter().map(|p| p.line).collect();
        assert_eq!(lines, vec![2, 3, 4, 5, 7, 9, 11], "{problems:?}");
        assert!(problems[0]
            .message
            .starts_with("clave no válida en la cabecera: Categoria"));
        assert_eq!(problems[4].message, "clave repetida en la cabecera: ok_1");
        assert_eq!(problems[6].message, "clave repetida en la cabecera: title");
        assert_eq!(g.meta, vec![("ok_1".to_owned(), "uno".to_owned())]);
        assert_eq!(
            g.sala.as_deref(),
            Some("a"),
            "the first value of a key is kept"
        );
        assert_eq!(g.title, "uno");
        assert_eq!(g.scenes.len(), 1, "the body still plays");
    }
}
