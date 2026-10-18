//! The native side of `window.velvet`.
//!
//! The game sends a JSON request, `{ "id", "cmd", "args" }`, through `window.ipc.postMessage`.
//! [`parse_request`] reads it, [`handle`] runs the command, and [`reply_script`] builds the
//! JavaScript that settles the game's promise. Window commands do not touch the window: they
//! queue a [`WindowAction`] in the [`BridgeCtx`], and the event loop performs it after the reply.
//! A picture of the page ([`CAPTURE_CMD`]) finishes later, so the event loop answers it with
//! [`capture_reply`]. Nothing here needs a window, so the logic is tested with temporary folders.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::capture::{self, CaptureError};
use crate::manifest::Manifest;
use crate::shell::{self, ShellError};
use crate::storage::{self, StorageError};

/// The bridge script. It has one `__CONFIG__` placeholder for the game settings JSON.
const INIT_TEMPLATE: &str = include_str!("bridge_init.js");

/// The command that takes a picture of the page (`window.capture()`). The event loop answers it
/// once WebView2 has the picture, so [`handle`] refuses to answer it.
pub const CAPTURE_CMD: &str = "window.capture";

/// The log levels that `window.velvet.log` accepts.
const LOG_LEVELS: [&str; 4] = ["debug", "info", "warn", "error"];

/// Longest log message kept, in bytes. Longer messages are cut at a character boundary.
const MAX_LOG_BYTES: usize = 8 * 1024;

/// The game identity that the bridge exposes as `window.velvet.game`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInfo {
    /// The game id from the manifest.
    pub id: String,
    /// The game title from the manifest.
    pub title: String,
    /// The game version from the manifest.
    pub version: String,
}

impl From<&Manifest> for GameInfo {
    fn from(manifest: &Manifest) -> Self {
        Self {
            id: manifest.id.clone(),
            title: manifest.title.clone(),
            version: manifest.version.clone(),
        }
    }
}

/// One request from the game, as it arrives on `window.ipc`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BridgeRequest {
    /// The reply id. `None` means the game wants no reply (`log` sends none).
    pub id: Option<u64>,
    /// The command name, such as `saves.read`.
    pub cmd: String,
    /// The command arguments. `null` when the game sent none.
    #[serde(default)]
    pub args: Value,
}

/// A window change that the event loop must perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    /// Enter (`true`) or leave (`false`) borderless fullscreen.
    SetFullscreen(bool),
    /// Close the window, which ends the program.
    Close,
}

/// What the commands can see and change: the game's data folder and the window state.
#[derive(Debug)]
pub struct BridgeCtx {
    game_id: String,
    data_dir: Option<PathBuf>,
    fullscreen: bool,
    actions: Vec<WindowAction>,
}

impl BridgeCtx {
    /// A context for one game. `data_dir` is `None` when the machine has no data folder.
    pub fn new(game_id: impl Into<String>, data_dir: Option<PathBuf>) -> Self {
        Self {
            game_id: game_id.into(),
            data_dir,
            fullscreen: false,
            actions: Vec::new(),
        }
    }

    /// Records whether the window is fullscreen now. The event loop calls this before a command.
    pub fn set_fullscreen_state(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
    }

    /// Returns the window actions queued so far, in order, and empties the queue.
    pub fn take_actions(&mut self) -> Vec<WindowAction> {
        std::mem::take(&mut self.actions)
    }

    fn data_dir(&self) -> Result<&Path, BridgeError> {
        self.data_dir.as_deref().ok_or(BridgeError::NoDataFolder)
    }
}

/// Why a bridge request failed. The game receives [`BridgeError::code`] and the message.
#[derive(Debug, Error)]
pub enum BridgeError {
    /// The message is not a JSON request.
    #[error("invalid bridge message: {0}")]
    BadMessage(String),
    /// No command has this name.
    #[error("unknown command `{0}`")]
    UnknownCommand(String),
    /// An argument is missing or has the wrong type or value.
    #[error("bad arguments: {0}")]
    BadArgs(String),
    /// This machine has no data folder for the game.
    #[error("this machine has no data folder for the game, so saves and settings are unavailable")]
    NoDataFolder,
    /// A save or settings operation failed.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// The link was not opened (see [`crate::shell`]).
    #[error(transparent)]
    Shell(#[from] ShellError),
    /// The picture was not taken, or is too large to give (see [`crate::capture`]).
    #[error(transparent)]
    Capture(#[from] CaptureError),
    /// A command reached the wrong place. This is a fault in the host, not in the game's call.
    #[error("internal error: {0}")]
    Internal(&'static str),
}

impl BridgeError {
    /// A short machine-readable code, which the game receives as `error.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadMessage(_) => "bad_message",
            Self::UnknownCommand(_) => "unknown_command",
            Self::BadArgs(_) => "bad_args",
            Self::NoDataFolder => "no_data_folder",
            Self::Storage(error) => error.code(),
            Self::Shell(error) => error.code(),
            Self::Capture(error) => error.code(),
            Self::Internal(_) => "internal",
        }
    }
}

/// Reads one request from the game's message text.
pub fn parse_request(message: &str) -> Result<BridgeRequest, BridgeError> {
    serde_json::from_str(message).map_err(|error| BridgeError::BadMessage(error.to_string()))
}

/// Runs one command. `args` is the request's `args` object (`Value::Null` when absent).
///
/// Returns the JSON value that settles the game's promise. Window commands queue a
/// [`WindowAction`] in `ctx`, for the event loop to perform.
pub fn handle(cmd: &str, args: &Value, ctx: &mut BridgeCtx) -> Result<Value, BridgeError> {
    match cmd {
        "saves.list" => Ok(json!(storage::list_saves(ctx.data_dir()?)?)),
        "saves.read" => {
            let text = storage::read_save(ctx.data_dir()?, str_arg(args, "slot")?)?;
            Ok(text.map_or(Value::Null, Value::String))
        }
        "saves.write" => {
            let slot = str_arg(args, "slot")?;
            let text = str_arg(args, "text")?;
            storage::write_save(ctx.data_dir()?, slot, text)?;
            Ok(Value::Null)
        }
        "saves.remove" => {
            storage::remove_save(ctx.data_dir()?, str_arg(args, "slot")?)?;
            Ok(Value::Null)
        }
        "settings.get" => Ok(storage::get_setting(
            ctx.data_dir()?,
            str_arg(args, "key")?,
        )?),
        "settings.set" => {
            let key = str_arg(args, "key")?;
            let value = args.get("value").cloned().unwrap_or(Value::Null);
            storage::set_setting(ctx.data_dir()?, key, value)?;
            Ok(Value::Null)
        }
        "window.setFullscreen" => {
            ctx.actions
                .push(WindowAction::SetFullscreen(bool_arg(args, "on")?));
            Ok(Value::Null)
        }
        "window.isFullscreen" => Ok(Value::Bool(ctx.fullscreen)),
        "window.close" => {
            ctx.actions.push(WindowAction::Close);
            Ok(Value::Null)
        }
        "shell.openExternal" => {
            shell::open_external(str_arg(args, "url")?)?;
            Ok(Value::Null)
        }
        CAPTURE_CMD => Err(BridgeError::Internal(
            "window.capture is answered by the event loop, not by handle",
        )),
        "log" => {
            let level = str_arg(args, "level")?;
            if !LOG_LEVELS.contains(&level) {
                return Err(BridgeError::BadArgs(format!(
                    "`level` must be debug, info, warn or error, not `{level}`"
                )));
            }
            let message = str_arg(args, "message")?;
            crate::log::line(&format_log_line(&ctx.game_id, level, message));
            Ok(Value::Null)
        }
        other => Err(BridgeError::UnknownCommand(other.to_string())),
    }
}

/// The line that a `log` call writes: `[game-id] level: message`.
///
/// A message longer than 8 KiB is cut at a character boundary and marked `[truncated]`.
pub fn format_log_line(game_id: &str, level: &str, message: &str) -> String {
    if message.len() <= MAX_LOG_BYTES {
        return format!("[{game_id}] {level}: {message}");
    }
    let mut end = MAX_LOG_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    format!("[{game_id}] {level}: {} [truncated]", &message[..end])
}

/// The JavaScript that settles the game's call `id`: `window.__velvetReply(id, ok, valueJson)`.
///
/// `valueJson` is the JSON text of the value, or of `{ code, message }` for a failure. It is
/// embedded as a string literal, so no quote or line break in the value can break the script.
pub fn reply_script(id: u64, result: &Result<Value, BridgeError>) -> String {
    let (ok, payload) = match result {
        Ok(value) => (true, value.to_string()),
        Err(error) => (
            false,
            json!({ "code": error.code(), "message": error.to_string() }).to_string(),
        ),
    };
    let literal = Value::String(payload).to_string();
    format!("if (window.__velvetReply) {{ window.__velvetReply({id}, {ok}, {literal}); }}")
}

/// The value that settles `window.capture()`: the picture as a PNG data URL, or the error that
/// stopped it (a picture over the size limit is `too_large`).
pub fn capture_reply(picture: Result<Vec<u8>, CaptureError>) -> Result<Value, BridgeError> {
    let png = picture?;
    Ok(Value::String(capture::png_data_url(&png)?))
}

/// The script that defines `window.velvet`. The host injects it into every page before the
/// page's own scripts run. `origin` is the only origin that gets the bridge.
pub fn init_script(game: &GameInfo, origin: &str) -> String {
    let config = json!({
        "game": { "id": game.id, "title": game.title, "version": game.version },
        "origin": origin,
    });
    INIT_TEMPLATE.replace("__CONFIG__", &config.to_string())
}

fn str_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, BridgeError> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| BridgeError::BadArgs(format!("`{name}` must be a string")))
}

fn bool_arg(args: &Value, name: &str) -> Result<bool, BridgeError> {
    args.get(name)
        .and_then(Value::as_bool)
        .ok_or_else(|| BridgeError::BadArgs(format!("`{name}` must be true or false")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_in(dir: &Path) -> BridgeCtx {
        BridgeCtx::new("demo", Some(dir.to_path_buf()))
    }

    fn call(ctx: &mut BridgeCtx, cmd: &str, args: Value) -> Result<Value, BridgeError> {
        handle(cmd, &args, ctx)
    }

    #[test]
    fn a_request_is_read_with_its_id_command_and_args() {
        let request = parse_request(r#"{"id":3,"cmd":"saves.list","args":{}}"#).expect("parses");
        assert_eq!(request.id, Some(3));
        assert_eq!(request.cmd, "saves.list");
        assert_eq!(request.args, json!({}));
    }

    #[test]
    fn a_request_without_an_id_or_args_is_still_read() {
        let log =
            parse_request(r#"{"id":null,"cmd":"log","args":{"level":"info","message":"hi"}}"#)
                .expect("parses");
        assert_eq!(log.id, None);
        let bare = parse_request(r#"{"cmd":"window.isFullscreen"}"#).expect("parses");
        assert_eq!(bare.id, None);
        assert_eq!(bare.args, Value::Null);
    }

    #[test]
    fn a_message_that_is_not_json_is_a_bad_message() {
        let error = parse_request("{oops").expect_err("must fail");
        assert_eq!(error.code(), "bad_message");
    }

    #[test]
    fn a_reply_settles_the_call_with_its_value() {
        let script = reply_script(7, &Ok(json!({ "a": "x\"y" })));
        assert_eq!(
            script,
            r#"if (window.__velvetReply) { window.__velvetReply(7, true, "{\"a\":\"x\\\"y\"}"); }"#
        );
    }

    #[test]
    fn a_failed_reply_carries_the_code_and_message() {
        let script = reply_script(9, &Err(BridgeError::UnknownCommand("nope".into())));
        assert_eq!(
            script,
            r#"if (window.__velvetReply) { window.__velvetReply(9, false, "{\"code\":\"unknown_command\",\"message\":\"unknown command `nope`\"}"); }"#
        );
    }

    #[test]
    fn a_reply_with_line_breaks_stays_one_literal() {
        let script = reply_script(1, &Ok(Value::String("line\nbreak \u{2028}".into())));
        assert!(!script.contains('\n'));
        assert!(script.contains(r#"\n"#));
    }

    #[test]
    fn the_init_script_embeds_the_game_and_the_origin() {
        let game = GameInfo {
            id: "demo".into(),
            title: "He said \"hi\"".into(),
            version: "1.0".into(),
        };
        let script = init_script(&game, "http://velvet.game");
        assert!(!script.contains("__CONFIG__"));
        assert!(script.contains(
            r#"var config = {"game":{"id":"demo","title":"He said \"hi\"","version":"1.0"},"origin":"http://velvet.game"};"#
        ));
        assert!(script.contains(r#"platform: "desktop""#));
        assert!(script.contains(r#"window.velvet"#));
    }

    #[test]
    fn game_info_comes_from_the_manifest() {
        let manifest = Manifest::from_json(r#"{"id":"demo","title":"Demo","version":"2.0.0"}"#)
            .expect("valid manifest");
        assert_eq!(
            GameInfo::from(&manifest),
            GameInfo {
                id: "demo".into(),
                title: "Demo".into(),
                version: "2.0.0".into(),
            }
        );
    }

    #[test]
    fn a_save_can_be_written_read_listed_and_removed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut ctx = ctx_in(dir.path());
        assert_eq!(
            call(&mut ctx, "saves.read", json!({"slot": "one"})).expect("read"),
            Value::Null
        );
        call(
            &mut ctx,
            "saves.write",
            json!({"slot": "one", "text": "{\"gold\":5}"}),
        )
        .expect("write");
        assert_eq!(
            call(&mut ctx, "saves.read", json!({"slot": "one"})).expect("read"),
            json!("{\"gold\":5}")
        );
        assert_eq!(
            call(&mut ctx, "saves.list", Value::Null).expect("list"),
            json!(["one"])
        );
        call(&mut ctx, "saves.remove", json!({"slot": "one"})).expect("remove");
        assert_eq!(
            call(&mut ctx, "saves.list", Value::Null).expect("list"),
            json!([])
        );
    }

    #[test]
    fn bad_slot_names_are_reported_with_a_code() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut ctx = ctx_in(dir.path());
        let error =
            call(&mut ctx, "saves.read", json!({"slot": "../secret"})).expect_err("refused");
        assert_eq!(error.code(), "invalid_name");
    }

    #[test]
    fn missing_arguments_are_reported_with_a_code() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut ctx = ctx_in(dir.path());
        let error = call(&mut ctx, "saves.write", json!({"slot": "one"})).expect_err("no text");
        assert_eq!(error.code(), "bad_args");
        let error =
            call(&mut ctx, "window.setFullscreen", json!({"on": "yes"})).expect_err("not bool");
        assert_eq!(error.code(), "bad_args");
        let listed = call(&mut ctx, "saves.list", Value::Null).expect("no args needed");
        assert_eq!(listed, json!([]));
    }

    #[test]
    fn settings_round_trip_through_the_bridge() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut ctx = ctx_in(dir.path());
        assert_eq!(
            call(&mut ctx, "settings.get", json!({"key": "volume"})).expect("get"),
            Value::Null
        );
        call(
            &mut ctx,
            "settings.set",
            json!({"key": "volume", "value": 0.25}),
        )
        .expect("set");
        call(
            &mut ctx,
            "settings.set",
            json!({"key": "lang", "value": "es"}),
        )
        .expect("set");
        assert_eq!(
            call(&mut ctx, "settings.get", json!({"key": "volume"})).expect("get"),
            json!(0.25)
        );
        assert_eq!(
            call(&mut ctx, "settings.get", json!({"key": "lang"})).expect("get"),
            json!("es")
        );
    }

    #[test]
    fn without_a_data_folder_saves_and_settings_fail_clearly() {
        let mut ctx = BridgeCtx::new("demo", None);
        let error = call(&mut ctx, "saves.list", Value::Null).expect_err("no folder");
        assert_eq!(error.code(), "no_data_folder");
        let error = call(&mut ctx, "settings.get", json!({"key": "a"})).expect_err("no folder");
        assert_eq!(error.code(), "no_data_folder");
    }

    #[test]
    fn unknown_commands_are_reported() {
        let mut ctx = BridgeCtx::new("demo", None);
        let error = call(&mut ctx, "saves.delete", Value::Null).expect_err("unknown");
        assert_eq!(error.code(), "unknown_command");
        assert_eq!(error.to_string(), "unknown command `saves.delete`");
    }

    #[test]
    fn window_commands_queue_actions_for_the_event_loop() {
        let mut ctx = BridgeCtx::new("demo", None);
        assert_eq!(
            call(&mut ctx, "window.setFullscreen", json!({"on": true})).expect("fullscreen"),
            Value::Null
        );
        call(&mut ctx, "window.setFullscreen", json!({"on": false})).expect("windowed");
        call(&mut ctx, "window.close", Value::Null).expect("close");
        assert_eq!(
            ctx.take_actions(),
            vec![
                WindowAction::SetFullscreen(true),
                WindowAction::SetFullscreen(false),
                WindowAction::Close,
            ]
        );
        assert!(ctx.take_actions().is_empty());
    }

    #[test]
    fn is_fullscreen_reports_the_state_the_event_loop_set() {
        let mut ctx = BridgeCtx::new("demo", None);
        assert_eq!(
            call(&mut ctx, "window.isFullscreen", Value::Null).expect("query"),
            json!(false)
        );
        ctx.set_fullscreen_state(true);
        assert_eq!(
            call(&mut ctx, "window.isFullscreen", Value::Null).expect("query"),
            json!(true)
        );
    }

    #[test]
    fn log_levels_are_checked() {
        let mut ctx = BridgeCtx::new("demo", None);
        call(
            &mut ctx,
            "log",
            json!({"level": "warn", "message": "careful"}),
        )
        .expect("valid log");
        let error =
            call(&mut ctx, "log", json!({"level": "shout", "message": "x"})).expect_err("bad");
        assert_eq!(error.code(), "bad_args");
    }

    #[test]
    fn log_lines_are_prefixed_and_long_messages_are_cut() {
        assert_eq!(
            format_log_line("demo", "info", "hello"),
            "[demo] info: hello"
        );
        let long = "\u{e9}".repeat(6000);
        let line = format_log_line("demo", "error", &long);
        assert!(line.ends_with(" [truncated]"));
        let kept = line
            .strip_prefix("[demo] error: ")
            .and_then(|rest| rest.strip_suffix(" [truncated]"))
            .expect("prefix and marker");
        assert_eq!(kept.len(), MAX_LOG_BYTES);
        assert!(kept.chars().all(|character| character == '\u{e9}'));
    }

    /// A PNG header for a 2 by 2 picture, padded with zeros to `total` bytes.
    fn png_bytes(total: usize) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&2u32.to_be_bytes());
        bytes.extend_from_slice(&2u32.to_be_bytes());
        bytes.resize(total.max(bytes.len()), 0);
        bytes
    }

    #[test]
    fn a_link_that_is_not_web_or_mail_is_refused_before_anything_opens() {
        let mut ctx = BridgeCtx::new("demo", None);
        for url in ["javascript:alert(1)", "file:///C:/x", "https://a b.example"] {
            let error =
                call(&mut ctx, "shell.openExternal", json!({ "url": url })).expect_err("refused");
            assert_eq!(error.code(), "invalid_url", "{url}");
        }
        let missing = call(&mut ctx, "shell.openExternal", Value::Null).expect_err("no url");
        assert_eq!(missing.code(), "bad_args");
    }

    #[test]
    fn the_capture_command_is_answered_by_the_event_loop_not_by_handle() {
        let mut ctx = BridgeCtx::new("demo", None);
        let error = call(&mut ctx, CAPTURE_CMD, Value::Null).expect_err("handled elsewhere");
        assert_eq!(error.code(), "internal");
    }

    #[test]
    fn a_picture_becomes_a_png_data_url_reply() {
        let value = capture_reply(Ok(png_bytes(24))).expect("a PNG is returned");
        let text = value.as_str().expect("the reply is a string");
        assert!(text.starts_with("data:image/png;base64,iVBORw0KGgo"));
    }

    #[test]
    fn a_picture_that_failed_or_is_too_large_settles_with_its_code() {
        let timeout = capture_reply(Err(CaptureError::TimedOut(30))).expect_err("timed out");
        assert_eq!(timeout.code(), "timeout");
        let unsupported = capture_reply(Err(CaptureError::Unsupported)).expect_err("unsupported");
        assert_eq!(unsupported.code(), "unsupported");
        let too_big = capture_reply(Ok(png_bytes(capture::MAX_CAPTURE_BYTES + 1)))
            .expect_err("over the limit");
        assert_eq!(too_big.code(), "too_large");
        let script = reply_script(4, &Err(BridgeError::from(CaptureError::Unsupported)));
        assert!(script.contains(r#"\"code\":\"unsupported\""#));
    }
}
