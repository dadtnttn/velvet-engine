//! The desktop window, the webview and the event loop.
//!
//! [`run`] opens the window from the manifest, creates the webview with the bridge script, and
//! runs the event loop until the window closes. The webview is only used on the event loop
//! thread. The IPC handler forwards each message to the loop through tao's `EventLoopProxy`, and
//! the loop runs the bridge, sends the reply and performs the window actions.

use std::path::PathBuf;

use tao::dpi::LogicalSize;
use tao::error::OsError;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::{Fullscreen, Window, WindowBuilder};
use wry::http::Request;
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;
use wry::{NewWindowResponse, WebContext, WebView, WebViewBuilder};

use crate::bridge::{self, BridgeCtx, GameInfo, WindowAction};
use crate::manifest::Manifest;
use crate::serve::{Site, SiteError};
use crate::{log, storage};

/// The custom protocol that serves the game folder. Its URLs are `velvet://game/...`.
///
/// On Windows wry turns the same protocol into `https://velvet.game/...` (https: a secure context,
/// as `crypto.subtle` and other web APIs need).
pub const GAME_SCHEME: &str = "velvet";

/// The host name in the game's custom protocol URLs.
pub const GAME_HOST: &str = "game";

/// What to run: the game folder, its manifest and the options from the command line.
#[derive(Debug, Clone)]
pub struct Launch {
    /// The game folder, which holds the manifest and the files.
    pub game_root: PathBuf,
    /// The manifest of that folder.
    pub manifest: Manifest,
    /// A dev server address to load instead of the folder's files.
    pub dev_url: Option<String>,
    /// Whether to open the web inspector. Debug builds always have it.
    pub devtools: bool,
}

/// Why the window could not run.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The game folder cannot be served.
    #[error(transparent)]
    Site(#[from] SiteError),
    /// The window cannot be created.
    #[error("cannot open the window: {0}")]
    Window(#[from] OsError),
    /// The webview cannot be created. On Windows it needs the WebView2 runtime.
    #[error(
        "cannot create the web view (on Windows it needs the Microsoft Edge WebView2 runtime): {0}"
    )]
    WebView(#[from] wry::Error),
    /// The dev server address has no origin.
    #[error("`{0}` is not a valid dev server address")]
    BadDevUrl(String),
    /// The event loop ended with an error code.
    #[error("the event loop stopped with code {0}")]
    EventLoop(i32),
}

/// Events that other code sends to the event loop.
#[derive(Debug)]
enum UserEvent {
    /// A raw message from `window.ipc.postMessage`.
    Ipc(String),
}

/// Opens the window, loads the game, and runs until the window closes.
pub fn run(launch: Launch) -> Result<(), AppError> {
    let Launch {
        game_root,
        manifest,
        dev_url,
        devtools,
    } = launch;

    // Where the page comes from, and which origin may use the bridge.
    let (start_url, origin, site) = match dev_url {
        Some(url) => {
            let origin = origin_of(&url).ok_or_else(|| AppError::BadDevUrl(url.clone()))?;
            (url, origin, None)
        }
        None => {
            let site = Site::new(&game_root, &manifest.entry)?;
            let start = game_url(cfg!(windows), &manifest.entry);
            (start, game_origin(cfg!(windows)), Some(site))
        }
    };

    let data_dir = storage::data_dir_for(&manifest.id);
    match &data_dir {
        Some(dir) => {
            log::open(dir);
        }
        None => log::line(
            "velvet-desktop: no data folder on this machine; saves and settings are unavailable",
        ),
    }
    // The webview's own profile (cache, local storage) lives in the data folder too, not next to
    // the program, which may be installed where it cannot write.
    let mut web_context = WebContext::new(data_dir.as_ref().map(|dir| dir.join("webview")));

    let mut event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let window = build_window(&event_loop, &manifest)?;
    let proxy = event_loop.create_proxy();

    let init = bridge::init_script(&GameInfo::from(&manifest), &origin);
    let navigation_origin = origin.clone();
    let mut builder = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_initialization_script(init)
        .with_devtools(devtools || cfg!(debug_assertions))
        .with_ipc_handler(move |request: Request<String>| {
            // Called on the UI thread. The loop does the work, so the message only travels here.
            let _ = proxy.send_event(UserEvent::Ipc(request.into_body()));
        })
        // Keep the game inside its own origin, so an outside page cannot reach the bridge.
        .with_navigation_handler(move |url: String| {
            url == "about:blank" || is_same_origin(&navigation_origin, &url)
        })
        .with_new_window_req_handler(|_url, _features| NewWindowResponse::Deny)
        .with_url(start_url);
    #[cfg(windows)]
    {
        builder = builder.with_https_scheme(true);
    }
    if let Some(site) = site {
        builder = builder.with_custom_protocol(GAME_SCHEME.to_string(), move |_id, request| {
            site.respond(request.method(), request.uri().path())
        });
    }
    let webview = builder.build(&window)?;

    let mut ctx = BridgeCtx::new(manifest.id.clone(), data_dir);
    let code = event_loop.run_return(move |event, _target, control_flow| {
        // The profile must outlive the webview.
        let _ = &web_context;
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Ipc(message)) => {
                handle_message(&message, &window, &webview, &mut ctx, control_flow);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
    if code == 0 {
        Ok(())
    } else {
        Err(AppError::EventLoop(code))
    }
}

/// Runs one bridge message on the event loop: the command, its reply, then any window action.
fn handle_message(
    message: &str,
    window: &Window,
    webview: &WebView,
    ctx: &mut BridgeCtx,
    control_flow: &mut ControlFlow,
) {
    let request = match bridge::parse_request(message) {
        Ok(request) => request,
        Err(error) => {
            log::line(&format!("velvet-desktop: {error}"));
            return;
        }
    };
    ctx.set_fullscreen_state(window.fullscreen().is_some());
    let result = bridge::handle(&request.cmd, &request.args, ctx);
    match request.id {
        Some(id) => {
            let script = bridge::reply_script(id, &result);
            if let Err(error) = webview.evaluate_script(&script) {
                log::line(&format!(
                    "velvet-desktop: cannot reply to `{}`: {error}",
                    request.cmd
                ));
            }
        }
        None => {
            if let Err(error) = &result {
                log::line(&format!(
                    "velvet-desktop: `{}` failed: {error}",
                    request.cmd
                ));
            }
        }
    }
    for action in ctx.take_actions() {
        match action {
            WindowAction::SetFullscreen(on) => {
                window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
            }
            WindowAction::Close => *control_flow = ControlFlow::Exit,
        }
    }
}

fn build_window(event_loop: &EventLoop<UserEvent>, manifest: &Manifest) -> Result<Window, OsError> {
    let config = manifest.window;
    WindowBuilder::new()
        .with_title(manifest.title.as_str())
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_min_inner_size(LogicalSize::new(config.min_width, config.min_height))
        .with_resizable(config.resizable)
        .with_fullscreen(config.fullscreen.then_some(Fullscreen::Borderless(None)))
        .build(event_loop)
}

/// The origin of the game's pages as the webview sees it: `https://velvet.game` on Windows and
/// `velvet://game` elsewhere.
pub fn game_origin(windows: bool) -> String {
    if windows {
        format!("https://{GAME_SCHEME}.{GAME_HOST}")
    } else {
        format!("{GAME_SCHEME}://{GAME_HOST}")
    }
}

/// The address of a page in the game folder, as the webview loads it.
pub fn game_url(windows: bool, entry: &str) -> String {
    format!("{}/{entry}", game_origin(windows))
}

/// The origin (`scheme://host[:port]`) of an address, or `None` if it has no scheme or host.
pub fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme_ok = !scheme.is_empty()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'));
    let authority = rest.split(['/', '?', '#']).next()?;
    if !scheme_ok || authority.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

/// Whether `url` is on `origin` (the same scheme, host and port), and not on a longer host
/// that merely starts with the same text.
pub fn is_same_origin(origin: &str, url: &str) -> bool {
    url.strip_prefix(origin)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_addresses_differ_by_platform() {
        assert_eq!(game_origin(true), "https://velvet.game");
        assert_eq!(game_origin(false), "velvet://game");
        assert_eq!(
            game_url(true, "index.html"),
            "https://velvet.game/index.html"
        );
        assert_eq!(
            game_url(false, "pages/start.html"),
            "velvet://game/pages/start.html"
        );
    }

    #[test]
    fn origins_are_read_from_addresses() {
        assert_eq!(
            origin_of("http://localhost:5173/"),
            Some("http://localhost:5173".into())
        );
        assert_eq!(
            origin_of("https://example.com/a?b#c"),
            Some("https://example.com".into())
        );
        assert_eq!(
            origin_of("http://[::1]:8080/x"),
            Some("http://[::1]:8080".into())
        );
        assert_eq!(origin_of("http://"), None);
        assert_eq!(origin_of("http:///path"), None);
        assert_eq!(origin_of("localhost:5173"), None);
        assert_eq!(origin_of("://host"), None);
        assert_eq!(origin_of("bad scheme://host"), None);
    }

    #[test]
    fn same_origin_checks_the_whole_host() {
        assert!(is_same_origin(
            "http://velvet.game",
            "http://velvet.game/index.html"
        ));
        assert!(is_same_origin("http://velvet.game", "http://velvet.game"));
        assert!(is_same_origin(
            "http://velvet.game",
            "http://velvet.game?x=1"
        ));
        assert!(is_same_origin("velvet://game", "velvet://game/main.js"));
        assert!(!is_same_origin(
            "http://velvet.game",
            "http://velvet.game.evil.example/"
        ));
        assert!(!is_same_origin(
            "http://velvet.game",
            "http://velvet.gamex/"
        ));
        assert!(!is_same_origin(
            "http://localhost:5173",
            "http://localhost:51730/"
        ));
        assert!(!is_same_origin(
            "http://velvet.game",
            "https://velvet.game/"
        ));
        assert!(!is_same_origin("velvet://game", "https://example.com/"));
    }
}
