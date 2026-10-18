//! The desktop window, the webview and the event loop.
//!
//! [`run`] opens the window from the manifest, creates the webview with the bridge script, and
//! runs the event loop until the window closes. The webview is only used on the event loop
//! thread. The IPC handler forwards each message to the loop through tao's `EventLoopProxy`, and
//! the loop runs the bridge, sends the reply and performs the window actions.
//!
//! A picture of the page (`window.capture()`, or a `--capture` run) also goes through the loop.
//! WebView2 finishes a picture later, on this thread, and reports it as a loop event. Every
//! picture has a time limit, so one that never comes back is still answered.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use tao::dpi::{LogicalSize, PhysicalSize};
use tao::error::OsError;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::{Fullscreen, Icon, Window, WindowBuilder};
use wry::http::Request;
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;
use wry::{NewWindowResponse, PageLoadEvent, WebContext, WebView, WebViewBuilder};

use crate::bridge::{self, BridgeCtx, GameInfo, WindowAction};
use crate::capture::{self, CaptureError};
use crate::cli::{CaptureOptions, WindowSize};
use crate::manifest::Manifest;
use crate::origin::{is_same_origin, origin_of};
use crate::serve::{Site, SiteError};
use crate::{icon, log, storage, webview_capture};

/// The custom protocol that serves the game folder. Its URLs are `velvet://game/...`.
///
/// On Windows wry turns the same protocol into `https://velvet.game/...` (https: a secure context,
/// as `crypto.subtle` and other web APIs need).
pub const GAME_SCHEME: &str = "velvet";

/// The host name in the game's custom protocol URLs.
pub const GAME_HOST: &str = "game";

/// How long a `--capture` run waits for the page to finish loading before it gives up.
const PAGE_LOAD_LIMIT: Duration = Duration::from_secs(60);

/// How long a picture may take before it is given up. This covers `--capture` and
/// `window.capture()` alike.
const PICTURE_LIMIT: Duration = Duration::from_secs(30);

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
    /// The window's client size in device pixels, from `--size`. Such a window is never full screen.
    pub size: Option<WindowSize>,
    /// The picture to take, from `--capture`. The run ends once the file is written.
    pub capture: Option<CaptureOptions>,
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
    /// A `--capture` run did not write its picture. The text says why.
    #[error("{0}")]
    Capture(String),
}

/// Events that other code sends to the event loop.
#[derive(Debug)]
enum UserEvent {
    /// A raw message from `window.ipc.postMessage`.
    Ipc(String),
    /// The page's navigation has ended. It may have failed, but the page is then ready to show.
    PageFinished,
    /// The `--capture` wait is over: take the picture now.
    TakePicture,
    /// WebView2 has the picture, or has refused it. `token` says which picture.
    Picture {
        token: u64,
        result: Result<Vec<u8>, CaptureError>,
    },
    /// A picture has waited longer than [`PICTURE_LIMIT`].
    PictureTimedOut(u64),
    /// The page has not finished loading within [`PAGE_LOAD_LIMIT`].
    PageTimedOut,
}

/// What a finished picture is for.
#[derive(Debug)]
enum Purpose {
    /// Settle the game's `window.capture()` call with this reply id.
    Reply(u64),
    /// Write the picture to this file, for `--capture`.
    File(PathBuf),
}

/// The pictures on their way, by token. Each one ends in one of three ways: WebView2 answers, its
/// time limit runs out, or the run ends first.
struct Pictures {
    proxy: EventLoopProxy<UserEvent>,
    next_token: u64,
    waiting: HashMap<u64, Purpose>,
}

impl Pictures {
    /// Starts a picture for `purpose`. Its answer arrives as [`UserEvent::Picture`], and a late
    /// one is cut off by [`UserEvent::PictureTimedOut`].
    fn start(&mut self, webview: &WebView, purpose: Purpose) {
        let token = self.next_token;
        self.next_token += 1;
        self.waiting.insert(token, purpose);
        let proxy = self.proxy.clone();
        webview_capture::take_png(webview, move |result| {
            // When the loop has already ended, the answer is simply dropped.
            let _ = proxy.send_event(UserEvent::Picture { token, result });
        });
        later(
            &self.proxy,
            PICTURE_LIMIT,
            UserEvent::PictureTimedOut(token),
        );
    }

    /// Takes the purpose of a picture out of the list. `None` when that picture has already ended.
    fn finish(&mut self, token: u64) -> Option<Purpose> {
        self.waiting.remove(&token)
    }
}

/// The state of one run: the webview and its window, the bridge, the pictures in progress, and
/// the `--capture` state. The event loop passes every event to [`Session::handle`].
struct Session {
    // The webview comes first, so that it is dropped before the window it lives in.
    webview: WebView,
    window: Window,
    ctx: BridgeCtx,
    pictures: Pictures,
    /// The `--capture` request, when this run takes a picture.
    capture: Option<CaptureOptions>,
    /// Whether the page's navigation has ended.
    page_loaded: bool,
    /// Whether the `--capture` wait has started. It starts once.
    wait_started: bool,
    /// Whether the `--capture` file has been written.
    picture_written: bool,
    /// Why the run failed, when it did.
    failure: Option<String>,
    /// The web profile, which must outlive the webview. Nothing reads it.
    _web_context: WebContext,
}

impl Session {
    fn handle(&mut self, event: Event<'_, UserEvent>, control_flow: &mut ControlFlow) {
        // Wait for the next event, unless a handler below asks to end the run.
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(event) => self.user_event(event, control_flow),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    }

    fn user_event(&mut self, event: UserEvent, control_flow: &mut ControlFlow) {
        match event {
            UserEvent::Ipc(message) => handle_message(
                &message,
                &self.window,
                &self.webview,
                &mut self.ctx,
                &mut self.pictures,
                control_flow,
            ),
            UserEvent::PageFinished => {
                self.page_loaded = true;
                // A `--capture` run starts its wait when the page first finishes loading.
                if let Some(capture) = &self.capture {
                    if !self.wait_started {
                        self.wait_started = true;
                        let wait = Duration::from_millis(capture.after_ms);
                        later(&self.pictures.proxy, wait, UserEvent::TakePicture);
                    }
                }
            }
            UserEvent::TakePicture => {
                if let Some(capture) = &self.capture {
                    let purpose = Purpose::File(capture.path.clone());
                    self.pictures.start(&self.webview, purpose);
                }
            }
            UserEvent::Picture { token, result } => {
                if let Some(purpose) = self.pictures.finish(token) {
                    self.finish_picture(purpose, result, control_flow);
                }
            }
            UserEvent::PictureTimedOut(token) => {
                if let Some(purpose) = self.pictures.finish(token) {
                    let late = CaptureError::TimedOut(PICTURE_LIMIT.as_secs());
                    self.finish_picture(purpose, Err(late), control_flow);
                }
            }
            UserEvent::PageTimedOut => {
                if self.capture.is_some() && !self.page_loaded {
                    self.failure = Some(format!(
                        "the page did not finish loading within {} seconds",
                        PAGE_LOAD_LIMIT.as_secs()
                    ));
                    *control_flow = ControlFlow::Exit;
                }
            }
        }
    }

    /// Settles a finished picture: the game's reply, or the `--capture` file.
    fn finish_picture(
        &mut self,
        purpose: Purpose,
        result: Result<Vec<u8>, CaptureError>,
        control_flow: &mut ControlFlow,
    ) {
        match purpose {
            Purpose::Reply(id) => {
                let script = bridge::reply_script(id, &bridge::capture_reply(result));
                evaluate(&self.webview, &script, "`window.capture`");
            }
            Purpose::File(path) => {
                let saved = result
                    .map_err(|error| format!("cannot take the picture: {error}"))
                    .and_then(|png| write_picture(&path, &png));
                match saved {
                    Ok(()) => self.picture_written = true,
                    Err(message) => self.failure = Some(message),
                }
                *control_flow = ControlFlow::Exit;
            }
        }
    }
}

/// Opens the window, loads the game, and runs until the window closes.
///
/// A `--capture` run ends as soon as its picture file is written. When it cannot write the file,
/// the error says why.
pub fn run(launch: Launch) -> Result<(), AppError> {
    let Launch {
        game_root,
        manifest,
        dev_url,
        devtools,
        size,
        capture: capture_request,
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
    tie_webview_to_host();

    // The webview's own profile (cache, local storage) lives in the data folder too, not next to
    // the program, which may be installed where it cannot write.
    let mut web_context = WebContext::new(data_dir.as_ref().map(|dir| dir.join("webview")));

    let mut event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let window = build_window(
        &event_loop,
        &manifest,
        size,
        window_icon(&game_root, &manifest),
    )?;
    let ipc_proxy = event_loop.create_proxy();
    let page_proxy = event_loop.create_proxy();
    let pictures_proxy = event_loop.create_proxy();
    if capture_request.is_some() {
        // A `--capture` run must not wait forever for a page that never finishes loading.
        later(&pictures_proxy, PAGE_LOAD_LIMIT, UserEvent::PageTimedOut);
    }

    let init = bridge::init_script(&GameInfo::from(&manifest), &origin);
    let navigation_origin = origin.clone();
    let mut builder = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_initialization_script(init)
        .with_devtools(devtools || cfg!(debug_assertions))
        .with_ipc_handler(move |request: Request<String>| {
            // Called on the UI thread. The loop does the work, so the message only travels here.
            let _ = ipc_proxy.send_event(UserEvent::Ipc(request.into_body()));
        })
        // Keep the game inside its own origin, so an outside page cannot reach the bridge.
        .with_navigation_handler(move |url: String| {
            url == "about:blank" || is_same_origin(&navigation_origin, &url)
        })
        .with_new_window_req_handler(|_url, _features| NewWindowResponse::Deny)
        // A release on Windows has no console, so the log file is where a page load shows up.
        .with_on_page_load_handler(move |event, url| {
            if matches!(event, PageLoadEvent::Finished) {
                log::line(&format!("velvet-desktop: page loaded: {url}"));
                let _ = page_proxy.send_event(UserEvent::PageFinished);
            }
        })
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

    let mut session = Session {
        webview,
        window,
        ctx: BridgeCtx::new(manifest.id.clone(), data_dir),
        pictures: Pictures {
            proxy: pictures_proxy,
            next_token: 0,
            waiting: HashMap::new(),
        },
        capture: capture_request,
        page_loaded: false,
        wait_started: false,
        picture_written: false,
        failure: None,
        _web_context: web_context,
    };
    let code = event_loop.run_return(|event, _target, control_flow| {
        session.handle(event, control_flow);
    });

    // A picture run must end with its file. A run that ended before the picture leaves none.
    if session.capture.is_some() && !session.picture_written && session.failure.is_none() {
        session.failure = Some("the window closed before the picture was taken".to_string());
    }
    if let Some(message) = session.failure.take() {
        return Err(AppError::Capture(message));
    }
    if code == 0 {
        Ok(())
    } else {
        Err(AppError::EventLoop(code))
    }
}

/// Runs one bridge message on the event loop: the command, its reply, then any window action.
///
/// A picture of the page is started here, and its reply is sent later (see [`Pictures`]).
fn handle_message(
    message: &str,
    window: &Window,
    webview: &WebView,
    ctx: &mut BridgeCtx,
    pictures: &mut Pictures,
    control_flow: &mut ControlFlow,
) {
    let request = match bridge::parse_request(message) {
        Ok(request) => request,
        Err(error) => {
            log::line(&format!("velvet-desktop: {error}"));
            return;
        }
    };
    if request.cmd == bridge::CAPTURE_CMD {
        match request.id {
            Some(id) => pictures.start(webview, Purpose::Reply(id)),
            None => log::line(
                "velvet-desktop: `window.capture` needs a reply id, so no picture was taken",
            ),
        }
        return;
    }
    ctx.set_fullscreen_state(window.fullscreen().is_some());
    let result = bridge::handle(&request.cmd, &request.args, ctx);
    match request.id {
        Some(id) => {
            let script = bridge::reply_script(id, &result);
            evaluate(webview, &script, &format!("`{}`", request.cmd));
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

/// Runs a reply script on the page. A failure is logged, and the loop carries on.
fn evaluate(webview: &WebView, script: &str, what: &str) {
    if let Err(error) = webview.evaluate_script(script) {
        log::line(&format!("velvet-desktop: cannot reply to {what}: {error}"));
    }
}

/// Writes the `--capture` file, and logs where it went and how big the picture is.
fn write_picture(path: &Path, png: &[u8]) -> Result<(), String> {
    capture::write_png(path, png)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    let size = capture::png_size(png).map_or_else(String::new, |(width, height)| {
        format!(" ({width}x{height} pixels)")
    });
    log::line(&format!("velvet-desktop: wrote {}{size}", path.display()));
    Ok(())
}

/// Sends `event` to the loop after `delay`. A thread does the waiting, so the loop never blocks.
fn later(proxy: &EventLoopProxy<UserEvent>, delay: Duration, event: UserEvent) {
    let proxy = proxy.clone();
    thread::spawn(move || {
        thread::sleep(delay);
        // When the loop has already ended, the event is dropped.
        let _ = proxy.send_event(event);
    });
}

/// Ties the webview's processes to this host, so that they do not outlive a killed host (on Windows,
/// see `job.rs`). A failure is logged, and the window runs on without it.
fn tie_webview_to_host() {
    #[cfg(windows)]
    {
        if let Err(error) = crate::job::end_with_host() {
            log::line(&format!(
                "velvet-desktop: cannot tie the webview's processes to this host ({error}); \
                 if the host is killed, they may keep running"
            ));
        }
    }
}

/// The window's icon, from the manifest's `icon` in the game folder, or None when there is none.
///
/// A missing or broken icon is logged as a warning, and the game starts without one.
fn window_icon(game_root: &Path, manifest: &Manifest) -> Option<Icon> {
    let relative = manifest.icon.as_deref()?;
    let image = match icon::load(&game_root.join(relative)) {
        Ok(image) => image,
        Err(error) => {
            log::line(&format!(
                "velvet-desktop: warning: the window icon is not used: {error}"
            ));
            return None;
        }
    };
    match Icon::from_rgba(image.rgba, image.width, image.height) {
        Ok(built) => Some(built),
        Err(error) => {
            log::line(&format!(
                "velvet-desktop: warning: the window icon is not used: {error}"
            ));
            None
        }
    }
}

/// Opens the window. With `--size` it is exactly that many device pixels, and never full screen.
fn build_window(
    event_loop: &EventLoop<UserEvent>,
    manifest: &Manifest,
    size: Option<WindowSize>,
    icon: Option<Icon>,
) -> Result<Window, OsError> {
    let config = manifest.window;
    let builder = WindowBuilder::new()
        .with_title(manifest.title.as_str())
        .with_resizable(config.resizable)
        .with_window_icon(icon);
    match size {
        Some(size) => {
            let exact = PhysicalSize::new(size.width, size.height);
            builder
                .with_inner_size(exact)
                .with_min_inner_size(exact)
                .build(event_loop)
        }
        None => builder
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_min_inner_size(LogicalSize::new(config.min_width, config.min_height))
            .with_fullscreen(config.fullscreen.then_some(Fullscreen::Borderless(None)))
            .build(event_loop),
    }
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
}
