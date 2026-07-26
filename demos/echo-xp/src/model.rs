use anyhow::{anyhow, Context, Result};
use velvet_script_vs3::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppKind {
    Inbox,
    CaseFiles,
    PhotoViewer,
    TapePlayer,
    Classifier,
    RecycleBin,
    SystemDialog,
    OperatorRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w as i32 && py < self.y + self.h as i32
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventView {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct FrameView {
    pub phase: String,
    pub case_id: String,
    pub operator_name: String,
    pub boot_complete: bool,
    pub logged_in: bool,
    pub mail_read: bool,
    pub case_file_read: bool,
    pub photo_inspected: bool,
    pub tape_listened: bool,
    pub deleted_file_found: bool,
    pub classification: String,
    pub classification_attempts: i64,
    pub case_complete: bool,
    pub truth_unlocked: bool,
    pub anomaly_level: i64,
    pub system_corruption: f32,
    pub can_classify: bool,
    pub required_evidence: i64,
    pub collected_evidence: i64,
    pub current_message: String,
    pub message_kind: String,
    pub events: Vec<EventView>,
    pub save_revision: i64,
    pub clippy_state: String,
    pub clippy_dialog: String,
    pub name_swap: bool,
    pub time_glitch: bool,
    pub extra_icon: bool,
    pub subject_changed: bool,
    pub operator_compromised: bool,
}

impl FrameView {
    pub fn parse(value: &Value) -> Result<Self> {
        Ok(Self {
            phase: string(value, "phase")?,
            case_id: string(value, "case_id")?,
            operator_name: string(value, "operator_name")?,
            boot_complete: boolean(value, "boot_complete")?,
            logged_in: boolean(value, "logged_in")?,
            mail_read: boolean(value, "mail_read")?,
            case_file_read: boolean(value, "case_file_read")?,
            photo_inspected: boolean(value, "photo_inspected")?,
            tape_listened: boolean(value, "tape_listened")?,
            deleted_file_found: boolean(value, "deleted_file_found")?,
            classification: string(value, "classification")?,
            classification_attempts: integer(value, "classification_attempts")?,
            case_complete: boolean(value, "case_complete")?,
            truth_unlocked: boolean(value, "truth_unlocked")?,
            anomaly_level: integer(value, "anomaly_level")?,
            system_corruption: number(value, "system_corruption")?,
            can_classify: boolean(value, "can_classify")?,
            required_evidence: integer(value, "required_evidence")?,
            collected_evidence: integer(value, "collected_evidence")?,
            current_message: string(value, "current_message")?,
            message_kind: string(value, "message_kind")?,
            events: list(value, "events", parse_event)?,
            save_revision: integer(value, "save_revision")?,
            clippy_state: string(value, "clippy_state")?,
            clippy_dialog: string(value, "clippy_dialog")?,
            name_swap: boolean(value, "name_swap")?,
            time_glitch: boolean(value, "time_glitch")?,
            extra_icon: boolean(value, "extra_icon")?,
            subject_changed: boolean(value, "subject_changed")?,
            operator_compromised: boolean(value, "operator_compromised")?,
        })
    }
}

fn parse_event(value: &Value) -> Result<EventView> {
    Ok(EventView {
        kind: string(value, "kind")?,
        name: optional_string(value, "name"),
    })
}

fn list<T>(root: &Value, key: &str, parse: fn(&Value) -> Result<T>) -> Result<Vec<T>> {
    field(root, key)?
        .list_items()
        .map_err(|error| anyhow!(error))?
        .iter()
        .map(parse)
        .collect::<Result<Vec<_>>>()
        .with_context(|| format!("snapshot field `{key}`"))
}

fn field(value: &Value, key: &str) -> Result<Value> {
    value
        .map_get(key)
        .map_err(|error| anyhow!(error))?
        .ok_or_else(|| anyhow!("missing VS3 map field `{key}`"))
}

fn string(value: &Value, key: &str) -> Result<String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("`{key}` is not a string"))
}

fn optional_string(value: &Value, key: &str) -> String {
    field(value, key)
        .ok()
        .and_then(|item| item.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn integer(value: &Value, key: &str) -> Result<i64> {
    field(value, key)?
        .as_i64()
        .ok_or_else(|| anyhow!("`{key}` is not an integer"))
}

fn number(value: &Value, key: &str) -> Result<f32> {
    field(value, key)?
        .as_f64()
        .map(|n| n as f32)
        .ok_or_else(|| anyhow!("`{key}` is not numeric"))
}

fn boolean(value: &Value, key: &str) -> Result<bool> {
    let value = field(value, key)?;
    match value {
        Value::Bool(flag) => Ok(flag),
        _ => Err(anyhow!("`{key}` is not a boolean")),
    }
}
