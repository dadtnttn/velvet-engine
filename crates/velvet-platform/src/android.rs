//! Android glue: the activity handle, the entry the [`android_main!`](crate::android_main) macro
//! expands to, and the few calls that need the Java side (they run on the activity's UI thread).
//!
//! The activity is `android.app.NativeActivity`, driven by winit's NativeActivity backend
//! (`android-activity`). No Java code ships in the APK.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use jni::objects::{JObject, JValue};
use jni::strings::JNIStr;
use jni::{jni_sig, jni_str};

/// The handle of the running activity, as `android_main` receives it.
pub use winit::platform::android::activity::AndroidApp;
use winit::platform::android::activity::{InputStatus, MainEvent, PollEvent};

use crate::screen::{Insets, Orientation};

static APP: Mutex<Option<AndroidApp>> = Mutex::new(None);
static RUNS: AtomicU32 = AtomicU32::new(0);
static SAFE_AREA: Mutex<Insets> = Mutex::new(Insets::ZERO);

/// The running activity, once the game was entered through `android_main!`.
pub fn app() -> Option<AndroidApp> {
    APP.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The device's Android API level (`Build.VERSION.SDK_INT`).
pub fn sdk_version() -> i32 {
    std::panic::catch_unwind(AndroidApp::sdk_version).unwrap_or(0)
}

/// The entry `android_main!` expands to: keeps the activity handle for the rest of the platform
/// layer, runs the game, then finishes the activity and ends the process.
#[doc(hidden)]
pub fn run_main<R: crate::MainResult>(app: AndroidApp, main: impl FnOnce() -> R) {
    if RUNS.fetch_add(1, Ordering::SeqCst) > 0 {
        // A second activity in a live process: winit cannot build another event loop here, so
        // the process ends and the system starts the activity again in a fresh one.
        eprintln!(
            "[velvet-platform] the activity was created again in a live process; restarting it"
        );
        exit_process();
    }
    *APP.lock().unwrap_or_else(|e| e.into_inner()) = Some(app.clone());
    eprintln!(
        "[velvet-platform] android_main: API level {}, data in {:?}",
        sdk_version(),
        app.internal_data_path()
    );
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(main)) {
        Ok(result) => result.report(),
        Err(_) => eprintln!("[velvet-platform] the game panicked (the message is above)"),
    }
    finish_and_exit(&app);
}

/// Finishes the activity, waits (briefly) for Android to destroy it, and ends the process.
fn finish_and_exit(app: &AndroidApp) -> ! {
    on_ui_thread(app, "finishing the activity", |env, activity| {
        env.call_method(activity, jni_str!("finish"), jni_sig!("()V"), &[])?;
        Ok(())
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut destroyed = false;
    while !destroyed && Instant::now() < deadline {
        app.poll_events(Some(Duration::from_millis(50)), |event| {
            if let PollEvent::Main(MainEvent::Destroy) = event {
                destroyed = true;
            }
        });
        // Input nobody reads would make the system report the app as not responding.
        if let Ok(mut events) = app.input_events_iter() {
            while events.next(|_| InputStatus::Unhandled) {}
        }
    }
    *APP.lock().unwrap_or_else(|e| e.into_inner()) = None;
    exit_process()
}

fn exit_process() -> ! {
    // `_exit`: no atexit handlers or static destructors racing the threads Android still runs.
    unsafe { libc::_exit(0) }
}

/// Runs a JNI call on the activity's Java UI thread (where Android wants view and window calls).
/// Errors and Java exceptions are logged, never propagated.
///
/// The activity and VM pointers are read here, on the calling thread, so the UI thread never
/// takes android-activity's `AndroidApp` lock: if it waited there behind a writer, Android would
/// report the app as not responding.
pub fn on_ui_thread<F>(app: &AndroidApp, what: &'static str, call: F)
where
    F: FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> jni::errors::Result<()> + Send + 'static,
{
    // Addresses travel as integers: raw pointers are not `Send`.
    let vm_addr = app.vm_as_ptr() as usize;
    let activity_addr = app.activity_as_ptr() as usize;
    // Keeps the activity's global reference alive until the call has run.
    let handle = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let _alive = &handle;
        // SAFETY: the pointer is android-activity's JavaVM, valid for the whole process.
        let vm = unsafe { jni::vm::JavaVM::from_raw(vm_addr as *mut _) };
        let raw = activity_addr as jni::sys::jobject;
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            // SAFETY: `raw` is the activity's global reference, valid while `handle` lives.
            let activity = unsafe { env.as_cast_raw::<JObject>(&raw)? };
            let outcome = call(env, &activity);
            if outcome.is_err() && env.exception_check() {
                env.exception_clear();
            }
            outcome
        });
        if let Err(error) = result {
            eprintln!("[velvet-platform] {what} failed: {error}");
        }
    }));
}

fn window<'local>(
    env: &mut jni::Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        activity,
        jni_str!("getWindow"),
        jni_sig!("()Landroid/view/Window;"),
        &[],
    )?
    .l()
}

fn decor_view<'local>(
    env: &mut jni::Env<'local>,
    window: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        window,
        jni_str!("getDecorView"),
        jni_sig!("()Landroid/view/View;"),
        &[],
    )?
    .l()
}

/// `WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE`.
const BARS_BY_SWIPE: i32 = 2;
/// `View.SYSTEM_UI_FLAG_*` for sticky immersive mode before API 30: LAYOUT_STABLE |
/// LAYOUT_HIDE_NAVIGATION | LAYOUT_FULLSCREEN | HIDE_NAVIGATION | FULLSCREEN | IMMERSIVE_STICKY.
const LEGACY_IMMERSIVE: i32 = 0x100 | 0x200 | 0x400 | 0x2 | 0x4 | 0x1000;

/// Hides (or shows again) the status and navigation bars; a swipe from the edge shows them for a
/// moment. Android brings the bars back after some events, so this is applied again on resume and
/// focus (see [`crate::Lifecycle`]).
pub fn set_immersive(app: &AndroidApp, immersive: bool) {
    let sdk = sdk_version();
    on_ui_thread(app, "immersive mode", move |env, activity| {
        let window = window(env, activity)?;
        if sdk >= 30 {
            let controller = env
                .call_method(
                    &window,
                    jni_str!("getInsetsController"),
                    jni_sig!("()Landroid/view/WindowInsetsController;"),
                    &[],
                )?
                .l()?;
            if !controller.is_null() {
                let bars = env
                    .call_static_method(
                        jni_str!("android/view/WindowInsets$Type"),
                        jni_str!("systemBars"),
                        jni_sig!("()I"),
                        &[],
                    )?
                    .i()?;
                if immersive {
                    env.call_method(
                        &controller,
                        jni_str!("setSystemBarsBehavior"),
                        jni_sig!("(I)V"),
                        &[JValue::Int(BARS_BY_SWIPE)],
                    )?;
                    env.call_method(
                        &controller,
                        jni_str!("hide"),
                        jni_sig!("(I)V"),
                        &[JValue::Int(bars)],
                    )?;
                } else {
                    env.call_method(
                        &controller,
                        jni_str!("show"),
                        jni_sig!("(I)V"),
                        &[JValue::Int(bars)],
                    )?;
                }
                return Ok(());
            }
        }
        let decor = decor_view(env, &window)?;
        env.call_method(
            &decor,
            jni_str!("setSystemUiVisibility"),
            jni_sig!("(I)V"),
            &[JValue::Int(if immersive { LEGACY_IMMERSIVE } else { 0 })],
        )?;
        Ok(())
    });
}

/// `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON`.
const FLAG_KEEP_SCREEN_ON: i32 = 0x80;

/// Keeps the screen on while the game is shown (or lets it sleep again).
///
/// Set through `Window.addFlags` on the UI thread, not `AndroidApp::set_window_flags`: in
/// android-activity 0.6 that takes the app lock for writing while `poll_events` holds it for
/// reading, so calling it from an event handler (where games do) deadlocks the game thread, and
/// then the UI thread with it.
pub fn set_keep_screen_on(app: &AndroidApp, on: bool) {
    on_ui_thread(app, "keeping the screen on", move |env, activity| {
        let window = window(env, activity)?;
        let method = if on {
            jni_str!("addFlags")
        } else {
            jni_str!("clearFlags")
        };
        env.call_method(
            &window,
            method,
            jni_sig!("(I)V"),
            &[JValue::Int(FLAG_KEEP_SCREEN_ON)],
        )?;
        Ok(())
    });
}

/// Asks for an orientation at run time (the manifest's `screenOrientation` is the default).
pub fn set_orientation(app: &AndroidApp, orientation: Orientation) {
    let value = orientation.android_value();
    on_ui_thread(app, "setting the orientation", move |env, activity| {
        env.call_method(
            activity,
            jni_str!("setRequestedOrientation"),
            jni_sig!("(I)V"),
            &[JValue::Int(value)],
        )?;
        Ok(())
    });
}

/// The last safe area read by [`refresh_safe_area`] (window pixels).
pub fn safe_area() -> Insets {
    *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner())
}

/// Reads the window's insets again (system bars that are showing, and the display cutout) on the
/// UI thread; [`safe_area`] has them a moment later.
pub fn refresh_safe_area(app: &AndroidApp) {
    let sdk = sdk_version();
    on_ui_thread(app, "reading the window insets", move |env, activity| {
        let window = window(env, activity)?;
        let decor = decor_view(env, &window)?;
        let insets = env
            .call_method(
                &decor,
                jni_str!("getRootWindowInsets"),
                jni_sig!("()Landroid/view/WindowInsets;"),
                &[],
            )?
            .l()?;
        if insets.is_null() {
            return Ok(());
        }
        let read = if sdk >= 30 {
            let class = jni_str!("android/view/WindowInsets$Type");
            let bars = env
                .call_static_method(class, jni_str!("systemBars"), jni_sig!("()I"), &[])?
                .i()?;
            let cutout = env
                .call_static_method(class, jni_str!("displayCutout"), jni_sig!("()I"), &[])?
                .i()?;
            let sides = env
                .call_method(
                    &insets,
                    jni_str!("getInsets"),
                    jni_sig!("(I)Landroid/graphics/Insets;"),
                    &[JValue::Int(bars | cutout)],
                )?
                .l()?;
            let mut side = |name: &JNIStr| -> jni::errors::Result<u32> {
                Ok(env.get_field(&sides, name, jni_sig!("I"))?.i()?.max(0) as u32)
            };
            Insets {
                left: side(jni_str!("left"))?,
                top: side(jni_str!("top"))?,
                right: side(jni_str!("right"))?,
                bottom: side(jni_str!("bottom"))?,
            }
        } else {
            let mut get = |object: &JObject<'_>, name: &JNIStr| -> jni::errors::Result<u32> {
                Ok(env
                    .call_method(object, name, jni_sig!("()I"), &[])?
                    .i()?
                    .max(0) as u32)
            };
            let mut found = Insets {
                left: get(&insets, jni_str!("getSystemWindowInsetLeft"))?,
                top: get(&insets, jni_str!("getSystemWindowInsetTop"))?,
                right: get(&insets, jni_str!("getSystemWindowInsetRight"))?,
                bottom: get(&insets, jni_str!("getSystemWindowInsetBottom"))?,
            };
            if sdk >= 28 {
                let cutout = env
                    .call_method(
                        &insets,
                        jni_str!("getDisplayCutout"),
                        jni_sig!("()Landroid/view/DisplayCutout;"),
                        &[],
                    )?
                    .l()?;
                if !cutout.is_null() {
                    let mut get = |name: &JNIStr| -> jni::errors::Result<u32> {
                        Ok(env
                            .call_method(&cutout, name, jni_sig!("()I"), &[])?
                            .i()?
                            .max(0) as u32)
                    };
                    found = found.max(Insets {
                        left: get(jni_str!("getSafeInsetLeft"))?,
                        top: get(jni_str!("getSafeInsetTop"))?,
                        right: get(jni_str!("getSafeInsetRight"))?,
                        bottom: get(jni_str!("getSafeInsetBottom"))?,
                    });
                }
            }
            found
        };
        *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner()) = read;
        Ok(())
    });
}

/// Shows or hides the soft keyboard. NativeActivity has no text editor attached, so the keyboard
/// sends key events (winit `KeyboardInput` with `text`): plain characters work, composing input
/// methods (CJK, swipe typing) do not.
pub fn show_keyboard(app: &AndroidApp, show: bool) {
    if show {
        app.show_soft_input(true);
    } else {
        app.hide_soft_input(true);
    }
}
