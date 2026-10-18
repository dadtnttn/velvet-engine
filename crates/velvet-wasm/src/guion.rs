//! Guiones for JavaScript: velvet-guion's Markdown dialogue scenes, read, listed and played
//! through JSON.
//!
//! The exports, as JavaScript sees them:
//!
//! - `parseGuion(text, file, castJson)` reads one guion and returns its id, title, room, scenes,
//!   problems and metadata.
//! - `GuionLibrary(castJson)` holds the guiones of one game, read with one cast. Its methods:
//!   `add(file, text)`, `problems()`, `scenes()`, `entry(who, place, varsJson)` (the scene to play
//!   on meeting someone or using a place) and `start(scene, varsJson)`.
//! - `GuionPortraits(json)` holds the faces of the dialogue box (a `retratos.json`). Its methods:
//!   `find(who, mood)` and `entries()`.
//! - `GuionConversation`, from `start`, is a conversation in progress with the story's variables.
//!   Its methods: `current()`, `next()` (the method is `advance` in Rust), `choose(index)`,
//!   `vars()` and `setVars(varsJson)`.
//!
//! The JSON shapes:
//!
//! - **Cast:** `{"characters": {"<id>": ["<name>", ...]}, "thinker": {"id": "<id>", "name":
//!   "<name>"}}`. Both keys are optional. `{}`, or an empty string, is the default cast, where
//!   `narrador` and `narrator` name the narrator.
//! - **Vars:** `{"values": {"<var>": <integer>}, "seen": ["<file id>.<label>", ...]}`, the shape
//!   velvet-guion's `Vars` serializes to. Both keys are optional.
//! - **File:** the summary that `parseGuion` and `add` return: `{"file", "id", "title", "sala",
//!   "scenes", "problems", "meta"}`. `meta` is an object of the front matter's other keys, written
//!   in the order of the file, with each value as written (a list stays `a, b, c`).
//! - **Problem:** `{"file", "line", "message"}`. Lines start at 1.
//! - **Scene:** `{"id", "label", "title", "line", "targeted", "attrs": {"quien", "en", "si",
//!   "unaVez", "vn"}}`. `si` lists conditions `{"var", "op", "value"}`, where `op` is one of `eq`,
//!   `ne`, `gt`, `gte`, `lt` and `lte`.
//! - **Box:** what the dialogue box shows. `type` is `line`, `choice`, `vn` or `end`. Every box has
//!   `scene` (where it is) and `stage`, the stage directives (`{"cmd", "arg"}`) run since the box
//!   before it, in order. A line adds `who`, `name`, `text`, `kind` (`dialogue`, `narration`,
//!   `sound` or `thought`) and `mood`. A choice adds `prompt` and `options` (`{"label",
//!   "available"}`). A visual-novel hand-over adds `to`, the scene that takes over.
//!
//! Failures use the codes `bad-input`, `unknown-scene`, `not-a-choice`, `no-such-option`,
//! `unavailable` and `internal` (see the crate's docs for the convention).

use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use velvet_guion::{
    parse, Cast, Cond, Conversation, Guion, Library, LineKind, Op, Portrait, Portraits, Problem,
    Scene, Shown, Vars,
};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{json, respond, Failure};

/// A condition as JSON: `{"var", "op", "value"}`, with `op` spelled as velvet-guion's story
/// format spells it.
#[derive(Serialize)]
struct CondJson {
    var: String,
    op: &'static str,
    value: i32,
}

impl From<&Cond> for CondJson {
    fn from(cond: &Cond) -> Self {
        let op = match cond.op {
            Op::Eq => "eq",
            Op::Ne => "ne",
            Op::Gt => "gt",
            Op::Gte => "gte",
            Op::Lt => "lt",
            Op::Lte => "lte",
        };
        Self {
            var: cond.var.clone(),
            op,
            value: cond.value,
        }
    }
}

/// A scene's attributes as JSON.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttrsJson {
    quien: Option<String>,
    en: Option<String>,
    si: Vec<CondJson>,
    una_vez: bool,
    vn: bool,
}

/// A scene as JSON, without its steps.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SceneJson {
    id: String,
    label: String,
    title: String,
    line: usize,
    targeted: bool,
    attrs: AttrsJson,
}

impl From<&Scene> for SceneJson {
    fn from(scene: &Scene) -> Self {
        Self {
            id: scene.id.clone(),
            label: scene.label.clone(),
            title: scene.title.clone(),
            line: scene.line,
            targeted: scene.targeted,
            attrs: AttrsJson {
                quien: scene.attrs.quien.clone(),
                en: scene.attrs.en.clone(),
                si: scene.attrs.si.iter().map(CondJson::from).collect(),
                una_vez: scene.attrs.una_vez,
                vn: scene.attrs.vn,
            },
        }
    }
}

/// A problem as JSON.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemJson {
    file: String,
    line: usize,
    message: String,
}

impl From<&Problem> for ProblemJson {
    fn from(problem: &Problem) -> Self {
        Self {
            file: problem.file.clone(),
            line: problem.line,
            message: problem.message.clone(),
        }
    }
}

/// The summary of one guion file, as `parseGuion` and `add` return it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileJson {
    file: String,
    id: String,
    title: String,
    sala: Option<String>,
    scenes: Vec<SceneJson>,
    problems: Vec<ProblemJson>,
    #[serde(serialize_with = "serialize_meta")]
    meta: Vec<(String, String)>,
}

/// The open metadata of a file as a JSON object, its keys in the order the file gives them. The
/// entries are written one by one, in that order, so no map on the way sorts them.
fn serialize_meta<S: serde::Serializer>(
    meta: &[(String, String)],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_map(
        meta.iter()
            .map(|(key, value)| (key.as_str(), value.as_str())),
    )
}

/// The summary of `guion`, read from `file`, with the problems found in it.
fn file_json(file: &str, guion: &Guion, problems: &[Problem]) -> FileJson {
    FileJson {
        file: file.to_owned(),
        id: guion.id.clone(),
        title: guion.title.clone(),
        sala: guion.sala.clone(),
        scenes: guion.scenes.iter().map(SceneJson::from).collect(),
        problems: problems.iter().map(ProblemJson::from).collect(),
        meta: guion.meta.clone(),
    }
}

/// One stage directive as JSON.
#[derive(Serialize)]
struct StageJson {
    cmd: String,
    arg: String,
}

/// One option of a choice as JSON.
#[derive(Serialize)]
struct OptionJson {
    label: String,
    available: bool,
}

/// What the dialogue box shows, as JSON.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum BoxJson {
    Line {
        scene: String,
        stage: Vec<StageJson>,
        who: String,
        name: String,
        text: String,
        kind: &'static str,
        mood: Option<String>,
    },
    Choice {
        scene: String,
        stage: Vec<StageJson>,
        prompt: Option<String>,
        options: Vec<OptionJson>,
    },
    Vn {
        scene: String,
        stage: Vec<StageJson>,
        to: String,
    },
    End {
        scene: String,
        stage: Vec<StageJson>,
    },
}

/// The cast as JSON: the names of each character and the thinker of the bracketed lines.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CastJson {
    #[serde(default)]
    characters: BTreeMap<String, Vec<String>>,
    thinker: Option<ThinkerJson>,
}

/// The thinker of the cast as JSON.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThinkerJson {
    id: String,
    name: String,
}

/// `text`, or `fallback` when `text` is blank (the JSON arguments may be left empty).
fn defaulted<'a>(text: &'a str, fallback: &'a str) -> &'a str {
    if text.trim().is_empty() {
        fallback
    } else {
        text
    }
}

/// The failure for a JSON argument that does not have the expected shape.
fn bad_input(what: &str, error: &serde_json::Error) -> Failure {
    Failure::new(
        "bad-input",
        format!("the {what} is not valid JSON of the expected shape: {error}"),
    )
}

/// Reads a cast from its JSON text.
fn parse_cast(text: &str) -> Result<Cast, Failure> {
    let spec: CastJson =
        serde_json::from_str(defaulted(text, "{}")).map_err(|e| bad_input("cast", &e))?;
    let mut cast = Cast::new();
    for (id, names) in &spec.characters {
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        cast = cast.alias(id, &names);
    }
    if let Some(thinker) = &spec.thinker {
        cast = cast.thinker(&thinker.id, &thinker.name);
    }
    Ok(cast)
}

/// Reads the story's variables from their JSON text.
fn parse_vars(text: &str) -> Result<Vars, Failure> {
    serde_json::from_str(defaulted(text, "{}")).map_err(|e| bad_input("vars", &e))
}

/// The stage directives of a box, as JSON.
fn stage_json(stage: &[(String, String)]) -> Vec<StageJson> {
    stage
        .iter()
        .map(|(cmd, arg)| StageJson {
            cmd: cmd.clone(),
            arg: arg.clone(),
        })
        .collect()
}

/// The JSON name of a kind of line.
fn kind_name(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Dialogue => "dialogue",
        LineKind::Narration => "narration",
        LineKind::Sound => "sound",
        LineKind::Thought => "thought",
    }
}

/// Reads one guion (`text`, named `file`) with `castJson` and returns its summary: its id, title,
/// room, scenes and problems. A bad cast gives the error JSON.
#[wasm_bindgen(js_name = "parseGuion")]
pub fn parse_guion(text: &str, file: &str, cast_json: &str) -> String {
    respond(parse_cast(cast_json).map(|cast| {
        let (guion, problems) = parse(text, Path::new(file), &cast);
        file_json(file, &guion, &problems)
    }))
}

/// A conversation in progress: where it is, the story's variables, and the staging that led to
/// the box on screen. Made by [`GuionLibrary::start`].
#[wasm_bindgen]
pub struct GuionConversation {
    lib: Rc<Library>,
    conv: Conversation,
    vars: Vars,
    stage: Vec<(String, String)>,
}

impl GuionConversation {
    /// Starts scene `at` (file, scene) with `vars`, and runs it up to its first box.
    fn begin(lib: Rc<Library>, at: (usize, usize), mut vars: Vars) -> Self {
        let mut conv = Conversation::start(&lib, at, &mut vars);
        let stage = conv.take_staging();
        Self {
            lib,
            conv,
            vars,
            stage,
        }
    }

    /// The box on screen now.
    fn shown_box(&self) -> BoxJson {
        let scene = self.conv.scene_id(&self.lib).to_owned();
        let stage = stage_json(&self.stage);
        match self.conv.shown(&self.lib, &self.vars) {
            Shown::Line {
                who,
                name,
                text,
                kind,
                mood,
            } => BoxJson::Line {
                scene,
                stage,
                who,
                name,
                text,
                kind: kind_name(kind),
                mood,
            },
            Shown::Choice { prompt, options } => BoxJson::Choice {
                scene,
                stage,
                prompt,
                options: options
                    .into_iter()
                    .map(|(label, available)| OptionJson { label, available })
                    .collect(),
            },
            Shown::Vn(to) => BoxJson::Vn { scene, stage, to },
            Shown::End => BoxJson::End { scene, stage },
        }
    }

    /// Takes option `index` of the choice on screen.
    fn choose_option(&mut self, index: u32) -> Result<(), Failure> {
        let options = match self.conv.shown(&self.lib, &self.vars) {
            Shown::Choice { options, .. } => options,
            _ => {
                return Err(Failure::new(
                    "not-a-choice",
                    "the conversation is not waiting for a choice",
                ))
            }
        };
        let Some(&(_, available)) = options.get(index as usize) else {
            return Err(Failure::new(
                "no-such-option",
                format!(
                    "this choice has {} options, so there is no option {index}",
                    options.len()
                ),
            ));
        };
        if !available {
            return Err(Failure::new(
                "unavailable",
                format!("option {index} is closed: its condition does not hold"),
            ));
        }
        if !self.conv.choose(&self.lib, &mut self.vars, index as usize) {
            return Err(Failure::new(
                "unavailable",
                format!("option {index} cannot be taken"),
            ));
        }
        self.stage = self.conv.take_staging();
        Ok(())
    }
}

#[wasm_bindgen]
impl GuionConversation {
    /// The box on screen now. Changes nothing.
    pub fn current(&self) -> String {
        json(&self.shown_box())
    }

    /// Moves past the line on screen and returns the box that follows. On a choice, at the end
    /// or at a visual-novel hand-over it changes nothing and returns the same box.
    #[wasm_bindgen(js_name = "next")]
    pub fn advance(&mut self) -> String {
        if matches!(self.conv.shown(&self.lib, &self.vars), Shown::Line { .. }) {
            self.conv.advance(&self.lib, &mut self.vars);
            self.stage = self.conv.take_staging();
        }
        json(&self.shown_box())
    }

    /// Takes option `index` of the choice on screen and returns the box after it. A closed
    /// option, an index out of range or a box that is not a choice fails with its code.
    pub fn choose(&mut self, index: u32) -> String {
        respond(self.choose_option(index).map(|()| self.shown_box()))
    }

    /// The story's variables (`values` and `seen`), as JSON.
    pub fn vars(&self) -> String {
        json(&self.vars)
    }

    /// Replaces the story's variables with `varsJson` and returns them as they now are. The box
    /// on screen does not change.
    #[wasm_bindgen(js_name = "setVars")]
    pub fn set_vars(&mut self, vars_json: &str) -> String {
        match parse_vars(vars_json) {
            Ok(vars) => {
                self.vars = vars;
                json(&self.vars)
            }
            Err(failure) => failure.to_json(),
        }
    }
}

/// The guiones of one game, read with one cast. Make one for each game, add its files with `add`,
/// then find the scene to play with `entry` and start conversations with `start`. A conversation
/// keeps the guiones as they were when it started.
#[wasm_bindgen]
pub struct GuionLibrary {
    lib: Rc<Library>,
}

impl GuionLibrary {
    /// Starts scene `scene` with the story's variables, or fails with the reason.
    fn start_scene(&self, scene: &str, vars_json: &str) -> Result<GuionConversation, Failure> {
        let vars = parse_vars(vars_json)?;
        let at = self.lib.find(scene, None).ok_or_else(|| {
            Failure::new(
                "unknown-scene",
                format!(
                    "there is no scene {scene:?}; scene ids are file.label, such as posadera.llegada"
                ),
            )
        })?;
        Ok(GuionConversation::begin(Rc::clone(&self.lib), at, vars))
    }
}

#[wasm_bindgen]
impl GuionLibrary {
    /// An empty library whose guiones are read with `castJson`. A bad cast fails with the error
    /// JSON.
    #[wasm_bindgen(constructor)]
    pub fn new(cast_json: &str) -> Result<GuionLibrary, String> {
        let cast = parse_cast(cast_json).map_err(|failure| failure.to_json())?;
        Ok(Self {
            lib: Rc::new(Library::new(cast)),
        })
    }

    /// Reads one more guion (`file` names it, as `parseGuion` does) and returns its summary.
    pub fn add(&mut self, file: &str, text: &str) -> String {
        let lib = Rc::make_mut(&mut self.lib);
        let (guion, problems) = parse(text, Path::new(file), &lib.cast);
        let summary = json(&file_json(file, &guion, &problems));
        lib.problems.extend(problems);
        lib.guiones.push(guion);
        summary
    }

    /// Every problem of every guion added, in the order they were found.
    pub fn problems(&self) -> String {
        let problems: Vec<ProblemJson> = self.lib.problems.iter().map(ProblemJson::from).collect();
        json(&problems)
    }

    /// The scenes of every guion added, in file order, with their attributes.
    pub fn scenes(&self) -> String {
        let scenes: Vec<SceneJson> = self
            .lib
            .guiones
            .iter()
            .flat_map(|guion| guion.scenes.iter())
            .map(SceneJson::from)
            .collect();
        json(&scenes)
    }

    /// The scene to play on meeting `who` or using `place` (either may be null): the first, in
    /// file order, that matches, whose conditions hold and that is not a spent `una_vez`. Returns
    /// `{"scene": id}`, or `{"scene": null}` when none plays.
    pub fn entry(&self, who: Option<String>, place: Option<String>, vars_json: &str) -> String {
        respond(parse_vars(vars_json).map(|vars| {
            let found = self
                .lib
                .entry(who.as_deref(), place.as_deref(), &vars)
                .and_then(|(file, scene)| {
                    self.lib.guiones.get(file).and_then(|g| g.scenes.get(scene))
                })
                .map(|scene| scene.id.clone());
            serde_json::json!({ "scene": found })
        }))
    }

    /// Starts scene `scene` (an id such as `posadera.llegada`) with the story's `varsJson`, runs
    /// it up to its first box and returns the conversation. A bad vars text or an unknown scene
    /// throws the error JSON.
    pub fn start(&self, scene: &str, vars_json: &str) -> Result<GuionConversation, String> {
        self.start_scene(scene, vars_json)
            .map_err(|failure| failure.to_json())
    }
}

/// A portrait as JSON: the image's asset key, and the square around the head and shoulders.
#[derive(Serialize)]
struct PortraitJson {
    image: String,
    crop: [u32; 4],
}

impl From<Portrait> for PortraitJson {
    fn from(portrait: Portrait) -> Self {
        Self {
            image: portrait.image,
            crop: portrait.crop,
        }
    }
}

/// The faces of the dialogue box, read from a `retratos.json` (see docs/formats/GUION.md).
#[wasm_bindgen]
pub struct GuionPortraits {
    portraits: Portraits,
}

#[wasm_bindgen]
impl GuionPortraits {
    /// The portraits listed in `json`. A file that is not valid fails with the error JSON.
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<GuionPortraits, String> {
        let portraits = Portraits::parse(defaulted(json, "{}"))
            .map_err(|error| bad_input("portraits", &error).to_json())?;
        Ok(Self { portraits })
    }

    /// The portrait of `who` looking `mood` (or their normal one), as `{"image", "crop"}`, or null
    /// when `who` has none. A thought shows its thinker's portrait.
    pub fn find(&self, who: &str, mood: Option<String>) -> String {
        json(
            &self
                .portraits
                .find(who, mood.as_deref())
                .map(PortraitJson::from),
        )
    }

    /// Every portrait listed, as `[who, mood]` pairs.
    pub fn entries(&self) -> String {
        json(&self.portraits.entries())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// Parses a reply of the boundary, which must be valid JSON.
    fn reply(text: &str) -> Value {
        serde_json::from_str(text).expect("the boundary returns JSON")
    }

    /// A library with one guion, read with the default cast.
    fn library(file: &str, text: &str) -> GuionLibrary {
        let mut lib = GuionLibrary::new("{}").expect("the default cast");
        lib.add(file, text);
        lib
    }

    const TAVERN: &str = "---\nid: taberna\n---\n## llegada {en=puerta}\n**Greta:** —Hola.\n? ¿Qué?\n- Caldo -> caldo {ayuda +1}\n- Cuarto [oro >= 5] -> fin\n\n## caldo\n**Greta:** —Aquí tienes.\n";

    #[test]
    fn parse_guion_reports_scenes_and_problems_with_their_lines() {
        let text = "## inicio {quien=greta, tiempo=noche}\n@nada hace algo\n";
        let file = reply(&parse_guion(text, "taberna.md", "{}"));
        assert_eq!(file["id"], "taberna");
        assert_eq!(file["scenes"][0]["id"], "taberna.inicio");
        assert_eq!(file["scenes"][0]["attrs"]["quien"], Value::Null);
        assert_eq!(
            file["problems"],
            json!([
                {
                    "file": "taberna.md",
                    "line": 1,
                    "message": "atributos ilegibles en: inicio {quien=greta, tiempo=noche}"
                },
                {
                    "file": "taberna.md",
                    "line": 2,
                    "message": "orden desconocida o ilegible: @nada hace algo"
                }
            ])
        );
    }

    #[test]
    fn a_library_plays_a_conversation_through_json() {
        let lib = library("taberna.md", TAVERN);
        let mut play = lib.start("taberna.llegada", "{}").expect("the scene");
        let first = reply(&play.current());
        assert_eq!(first["type"], "line");
        assert_eq!(first["who"], "greta");
        assert_eq!(first["text"], "Hola.");
        let choice = reply(&play.advance());
        assert_eq!(choice["type"], "choice");
        assert_eq!(choice["prompt"], "¿Qué?");
        assert_eq!(choice["options"][1]["available"], false);
    }

    #[test]
    fn choices_refuse_what_cannot_happen_with_a_code() {
        let lib = library("taberna.md", TAVERN);
        let mut conv = lib.start("taberna.llegada", "{}").expect("the scene");
        assert_eq!(reply(&conv.choose(0))["code"], "not-a-choice");
        conv.advance();
        assert_eq!(reply(&conv.choose(9))["code"], "no-such-option");
        assert_eq!(reply(&conv.choose(1))["code"], "unavailable");
        let taken = reply(&conv.choose(0));
        assert_eq!(taken["type"], "line");
        assert_eq!(taken["scene"], "taberna.caldo");
        assert_eq!(reply(&conv.vars())["values"]["ayuda"], 1);
    }

    #[test]
    fn bad_arguments_and_unknown_scenes_fail_with_codes() {
        let lib = library("taberna.md", TAVERN);
        let missing = lib
            .start("taberna.nada", "{}")
            .err()
            .expect("no such scene");
        assert_eq!(reply(&missing)["code"], "unknown-scene");
        let bad_vars = lib
            .start("taberna.llegada", "[1]")
            .err()
            .expect("vars are an object");
        assert_eq!(reply(&bad_vars)["code"], "bad-input");
        let bad_cast = GuionLibrary::new("{\"characters\": 3}")
            .err()
            .expect("bad cast");
        assert_eq!(reply(&bad_cast)["code"], "bad-input");
        let bad_entry = reply(&lib.entry(None, None, "{\"values\": 1}"));
        assert_eq!(bad_entry["code"], "bad-input");
    }

    #[test]
    fn vars_can_be_written_back_and_read() {
        let lib = library("taberna.md", TAVERN);
        let mut conv = lib
            .start("taberna.llegada", "{\"values\": {\"oro\": 7}}")
            .expect("the scene");
        assert_eq!(reply(&conv.vars())["values"]["oro"], 7);
        assert_eq!(reply(&conv.vars())["seen"], json!(["taberna.llegada"]));
        let set = reply(&conv.set_vars("{\"values\": {\"oro\": 1}, \"seen\": []}"));
        assert_eq!(set, json!({ "values": { "oro": 1 }, "seen": [] }));
        assert_eq!(reply(&conv.current())["type"], "line");
    }

    #[test]
    fn entry_picks_the_first_scene_whose_conditions_hold() {
        let text = "## fiesta {quien=greta, una_vez, si=noche}\n**Greta:** —Música.\n\n## saludo {quien=greta}\n**Greta:** —Hola.\n";
        let lib = library("fiesta.md", text);
        let plain = reply(&lib.entry(Some("greta".into()), None, "{}"));
        assert_eq!(plain["scene"], "fiesta.saludo");
        let night = reply(&lib.entry(Some("greta".into()), None, "{\"values\": {\"noche\": 1}}"));
        assert_eq!(night["scene"], "fiesta.fiesta");
        let spent = reply(&lib.entry(
            Some("greta".into()),
            None,
            "{\"values\": {\"noche\": 1}, \"seen\": [\"fiesta.fiesta\"]}",
        ));
        assert_eq!(spent["scene"], "fiesta.saludo");
        assert_eq!(
            reply(&lib.entry(None, Some("barra".into()), "{}"))["scene"],
            Value::Null
        );
    }

    #[test]
    fn portraits_find_a_mood_its_synonym_and_the_thinkers_face() {
        let listing = r#"{
            "ana": {
                "normal": { "imagen": "retratos/ana", "recorte": [0, 0, 10, 10] },
                "enfadada": { "imagen": "retratos/ana_enfadada", "recorte": [1, 1, 5, 5] },
                "enojada": "enfadada"
            },
            "capitana": { "normal": { "imagen": "retratos/capitana", "recorte": [0, 0, 8, 8] } }
        }"#;
        let portraits = GuionPortraits::new(listing).expect("a valid listing");
        let angry = reply(&portraits.find("ana", Some("Enojada".into())));
        assert_eq!(
            angry,
            json!({ "image": "retratos/ana_enfadada", "crop": [1, 1, 5, 5] })
        );
        assert_eq!(
            reply(&portraits.find("ana", Some("furiosa".into()))),
            reply(&portraits.find("ana", None)),
            "an unknown mood shows the normal face"
        );
        assert_eq!(
            reply(&portraits.find("capitana_thought", None))["crop"],
            json!([0, 0, 8, 8])
        );
        assert_eq!(portraits.find("teo", None), "null");
        assert_eq!(
            reply(&portraits.entries()).as_array().map(Vec::len),
            Some(4)
        );
        assert!(
            GuionPortraits::new("[1]").is_err(),
            "a list of portraits is an object"
        );
    }

    #[test]
    fn the_default_cast_names_the_narrator() {
        let text = "**Narrador:** —Llueve.\n(Mmm.)\n";
        let file = reply(&parse_guion(text, "cuento.md", ""));
        assert!(file["problems"].as_array().is_some_and(Vec::is_empty));
        let mut lib = GuionLibrary::new("").expect("empty cast is the default");
        lib.add("cuento.md", text);
        let first = reply(&lib.start("cuento.inicio", "").expect("the scene").current());
        assert_eq!(first["who"], "narrator");
        assert_eq!(first["name"], "Narrador");
    }

    #[test]
    fn the_metadata_crosses_in_file_order_and_bad_keys_are_problems() {
        let text = "---\nid: escena\nzeta: 1\ncategoria: escena\nparticipantes: Lira, Nyxara\n---\n## inicio\n**Lira:** —Hola.\n";
        let raw = parse_guion(text, "escena.md", "{}");
        assert_eq!(
            reply(&raw)["meta"],
            json!({ "zeta": "1", "categoria": "escena", "participantes": "Lira, Nyxara" })
        );
        let zeta = raw.find("\"zeta\"").expect("the key is in the reply");
        let categoria = raw.find("\"categoria\"").expect("the key is in the reply");
        assert!(zeta < categoria, "the keys keep the file's order: {raw}");

        let mut lib = GuionLibrary::new("{}").expect("the default cast");
        let added = reply(&lib.add("escena.md", text));
        assert_eq!(added["meta"]["participantes"], "Lira, Nyxara");

        let bad = reply(&parse_guion("---\nCategoria: x\n---\n## a\n", "a.md", "{}"));
        assert_eq!(bad["meta"], json!({}));
        assert_eq!(bad["problems"][0]["line"], 2);
    }
}
