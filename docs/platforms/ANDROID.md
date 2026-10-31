# Android

Velvet Engine games run on Android with the same Rust code as on desktop. The game's program
lives in its library crate; the desktop `main` calls it, and on Android the system starts it
through `android.app.NativeActivity`. `velvet android` builds the library with the NDK and packs
it into a signed APK or an Android App Bundle, without Gradle or Java code.

| Piece | What it does |
|-------|--------------|
| `velvet-platform` | Entry point, event loop, GPU surface lifecycle, packaged files, save folders, touch, back button, screen (insets, immersive, orientation), lifecycle events |
| `velvet-assets` | `ApkSource` (the APK's assets through `AAssetManager`), `AssetReader` (streaming), `AssetIndex` |
| `velvet-host2d` | 2D games: Android handled by the host (touch is the pointer, back is Escape, suspend and resume) |
| `velvet-render` | `GltfModel::load_with`: models read through the packaged files |
| `velvet-build` / `velvet android` | NDK build, manifest and resources, APK/AAB assembly, zipalign, signing, verification |

Samples: `demos/third-person` (3D, `velvet-render`, a scene file, saves) and
`examples/hello-mobile` (2D, `velvet-host2d`, sound).

Status: the libraries build for `aarch64-linux-android` and `x86_64-linux-android`, the samples
package into signed and verified APKs and an app bundle; running them on devices is still to be
done (see [Known limits](#known-limits)).

## Setup

1. **Android Studio** (or the command-line tools) for the SDK. Set `ANDROID_HOME` to it
   (`%LOCALAPPDATA%\Android\Sdk` on Windows, `~/Android/Sdk` on Linux).
2. In the SDK manager (or `sdkmanager`):
   - **NDK r28 or newer** (`ndk;28.2.13676358`): it links 16 KB pages by default.
   - **Build-tools 35 or newer** (`build-tools;36.0.0`): `zipalign -P 16` and apksigner's
     16 KB alignment.
   - **Platform 36** (`platforms;android-36`): the target SDK Google Play requires.
   - Platform-tools (`adb`).
3. **A JDK** for apksigner, keytool, jarsigner and bundletool: set `JAVA_HOME` (Android Studio's
   `jbr` folder works).
4. **Rust targets**: `rustup target add aarch64-linux-android x86_64-linux-android`
   (`armv7-linux-androideabi` too for old 32-bit phones).
5. **bundletool**, for app bundles only: download `bundletool-all-<version>.jar` from
   <https://github.com/google/bundletool/releases> into `%LOCALAPPDATA%\velvet\tools` (Windows)
   or `~/.local/share/velvet/tools`, or point `BUNDLETOOL_JAR` at it.

`velvet android doctor` shows what was found and what is missing, and lists connected devices.

| Variable | Use |
|----------|-----|
| `ANDROID_HOME` / `ANDROID_SDK_ROOT` | The SDK |
| `ANDROID_NDK_HOME` / `ANDROID_NDK_ROOT` | An NDK other than the newest in `<sdk>/ndk` |
| `JAVA_HOME` | The JDK |
| `BUNDLETOOL_JAR` / `VELVET_TOOLS_DIR` | Where bundletool is |
| `VELVET_ANDROID_KEYSTORE`, `VELVET_ANDROID_KEY_ALIAS` | Release keystore and alias |
| `VELVET_KEYSTORE_PASS`, `VELVET_KEY_PASS` | Release passwords (handed to the signers by name) |

## Make a game Android-ready

### 1. The program in a library

```rust
// src/lib.rs
pub fn run() -> anyhow::Result<()> {
    let event_loop = velvet_platform::event_loop()?;   // attached to the activity on Android
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(&mut Game::default())?;
    Ok(())
}

velvet_platform::android_main!(run);                  // nothing on desktop

// src/main.rs
fn main() -> anyhow::Result<()> {
    my_game::run()
}
```

The crate needs no `crate-type`: `velvet android` builds the library as a `cdylib` itself
(`cargo rustc --crate-type cdylib`), so desktop builds and CI do not link a shared library.

When `run` returns, the activity is finished and the process ends once Android has destroyed it:
winit 0.30 builds one event loop per process, so the next launch must start fresh.

### 2. Settings in Cargo.toml

`velvet android init -p <crate>` prints a template.

```toml
[package.metadata.velvet.android]
package = "com.studio.game"          # application id; can never change once published
label = "My Game"                    # launcher name (default: the crate name)
version_code = 3                     # raise for every upload (default: from the version)
# version_name = "1.2.0"             # default: the crate's version
min_sdk = 26                         # AAudio, which rodio/cpal use, starts at 26
target_sdk = 36                      # Google Play requirement since 2026-08-31
# compile_sdk = 36                   # default: the lowest installed platform >= target_sdk
abis = ["arm64-v8a", "x86_64"]       # x86_64: emulators and Chromebooks
orientation = "sensorLandscape"      # or landscape, sensorPortrait, portrait, fullSensor, user...
fullscreen = true                    # status bar hidden by the theme
cutout = "shortEdges"                # draw into display cutouts (read the safe area)
assets = ["assets", { from = "../data", to = "data", exclude = ["**/*.psd"] }]
icon = "android/icon.png"            # square PNG, 512 px; a plain icon is drawn without it
# icon_background = "#241633"        # adaptive icon background (default: the icon's corner)
# permissions = ["android.permission.VIBRATE"]
# uses_features = ["android.hardware.gamepad"]
# cargo_features = ["mobile"]        # features for the Android build
# abi_cargo_features = { "arm64-v8a" = ["neon"] }  # more features for one ABI only
# no_default_features = false
# store_uncompressed = ["ogg", "png"]  # default: already-compressed media types
# strip = true                       # strip debug info from the packaged library
# asset_packs = [{ name = "media", assets = [{ from = "../media", to = "media" }] }]  # AAB only
```

Paths are relative to the crate. Asset folders keep their name under `assets/` unless `to`
says otherwise (`to = ""` packs the folder's contents at the root). Hidden files (`.git`,
`.DS_Store`) are left out.

## Build, sign, install, debug

| Command | What it does |
|---------|--------------|
| `velvet android doctor` | Toolchain check and connected devices (installs nothing) |
| `velvet android build -p game [--abi arm64-v8a] [--debug] [-j 6]` | The native library per ABI, checked (ELF machine, 16 KB pages, needed libraries) |
| `velvet android apk -p game [--sign debug\|release\|none] [--out x.apk]` | Build, package, `zipalign -P 16`, sign with apksigner, then verify and print the contents |
| `velvet android aab -p game [--sign release] [--check]` | An app bundle via bundletool, signed with jarsigner; `--check` also makes the universal APK Play would serve and verifies it |
| `velvet android sign <file> --sign release` | Sign an existing APK or AAB again |
| `velvet android verify <file>` | `apksigner verify --print-certs` / `jarsigner -verify` + `bundletool validate`, alignment, ABIs, assets, manifest (`aapt2 dump badging`) |
| `velvet android install <apk> [--device SERIAL]` | `adb install -r`; refuses when no device is connected. No other command installs anything |
| `velvet android logcat -p game [--all] [--dump]` | The game's log: its stdout/stderr (tag `RustStdoutStderr`), panics and native crashes |
| `velvet android keygen-help` | How to make a release key (nothing is created) |

`--profile` picks a cargo profile (`release` by default; `--debug` is `dev`, which also makes the
app debuggable). Outputs go to `target/velvet-android/<package>/<profile>/`.

### What the pipeline does

1. `cargo rustc --lib --crate-type cdylib --target <triple>` with the NDK's clang as linker and C/C++
   compiler (for crates with C code), `llvm-ar`, a static C++ runtime, and
   `-z max-page-size=16384` on the link line.
2. The library is stripped of debug info (the unstripped one stays in
   `target/<triple>/<profile>/` for symbolising crashes) and checked: machine, 16 KB `PT_LOAD`
   alignment on 64-bit ABIs, and every `DT_NEEDED` library either an Android system library
   available at `min_sdk` or packaged (`libc++_shared.so` is copied from the NDK if needed).
3. `AndroidManifest.xml` (NativeActivity, `hasCode="false"`, `appCategory="game"`, the
   configuration changes the activity handles itself, `enableOnBackInvokedCallback="false"`),
   the theme, label and launcher icons (legacy and adaptive) are written and linked with `aapt2`.
4. The APK is written: aapt2's entries, `lib/<abi>/*.so` stored, assets (media stored, everything
   else deflated), and `assets/velvet-asset-index.txt`, the list of packaged files.
5. `zipalign -f -P 16 4`, then `apksigner sign` (v2 and v3 schemes for API 24 and up), then
   `zipalign -c -P 16 4`, `apksigner verify` and `aapt2 dump badging`.
6. App bundles: aapt2's protobuf output becomes the base module (with the libraries and assets),
   each install-time asset pack its own module, `bundletool build-bundle` with a configuration that
   keeps native libraries and media uncompressed, then `jarsigner`.

### Signing

- **Debug** (default): the standard Android debug key, `~/.android/debug.keystore` (alias
  `androiddebugkey`, password `android`, as published by Android). Made with keytool if missing.
  Installable for testing; stores refuse it.
- **Release**: your own key, which the tools never create. Make it once:

  ```text
  keytool -genkeypair -v -keystore my-upload-key.keystore -alias upload -keyalg RSA -keysize 4096 -validity 10000
  ```

  Then give it to `velvet android` by any of:
  1. `--keystore <path> --alias <alias>`;
  2. `VELVET_ANDROID_KEYSTORE`, `VELVET_ANDROID_KEY_ALIAS`, `VELVET_KEYSTORE_PASS` and optionally
     `VELVET_KEY_PASS`;
  3. a file `android-signing.local.properties` beside the game's Cargo.toml or the workspace root,
     with Gradle's keys `storeFile` (relative to the file), `keyAlias`, and optionally
     `storePassword` and `keyPassword`.

  Passwords reach apksigner and jarsigner through environment variables named on their command
  lines (`--ks-pass env:VELVET_KEYSTORE_PASS`), never as text. Without a password, the signer asks
  for it on the terminal. Keystores (`*.keystore`, `*.jks`), the signing file and packages are
  git-ignored. Keep the key and its passwords backed up outside the repository.
- **Google Play**: enrol in Play App Signing; your key is then the *upload* key and Google holds
  the app signing key. Upload `.aab` files.

### Debugging

- `velvet android logcat -p game` follows the game's process: everything it prints to stdout and
  stderr (android-activity redirects both to logcat), panics, wgpu messages, crash dumps.
- `velvet android install` reinstalls over an existing install (`-r`), keeping saves. A different
  signing key needs an uninstall first (`adb uninstall <package>`), which deletes the saves.
- Emulators run the `x86_64` ABI; their GPU support varies (wgpu falls back to OpenGL ES).

## The API a game uses

### Event loop and lifecycle

```rust
use velvet_platform::{gpu::SurfaceSlot, Lifecycle, PlatformEvent};

impl ApplicationHandler for Game {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.lifecycle.resumed();          // re-applies immersive mode, reads the safe area
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.resume(&gpu.device).ok();   // back from the background: a new surface
            return;
        }
        // First start: create the window and the GPU objects (see below).
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.lifecycle.suspended();
        if let Some(gpu) = &mut self.gpu {
            gpu.surface.suspend();         // Android destroys the window right after this
        }
        self.save();                        // the process may be killed from now on
        self.pause_audio();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(PlatformEvent::Back) = self.lifecycle.window_event(&event) {
            self.go_back_or_quit(el);       // on the first screen: el.exit()
        }
        // ...
    }

    fn memory_warning(&mut self, _: &ActiveEventLoop) {
        self.drop_caches();
    }
}
```

- `Lifecycle::new().with_immersive(true).with_keep_screen_on(true)` keeps the system bars hidden
  and the screen on. `state()` says `Running`, `Unfocused` or `Background`.
- winit 0.30 does not forward Android's pause, stop, destroy or saved-instance-state callbacks:
  treat `suspended` as the last chance to save.
- Use `ControlFlow::Wait` (or `WaitUntil`) and `request_redraw`: with `ControlFlow::Poll` the loop
  keeps spinning while the app is in the background.
- On Android `RedrawRequested` only arrives while the app is resumed.

### GPU

```rust
let instance = wgpu::Instance::new(&velvet_platform::gpu::instance_descriptor());
let mut surface = SurfaceSlot::new(instance, window.clone())?;
let adapter = velvet_platform::gpu::request_adapter(
    surface.instance(), surface.surface().unwrap(), wgpu::PowerPreference::HighPerformance,
).await.unwrap();
let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
    required_limits: velvet_platform::gpu::supported_limits(&adapter),
    ..Default::default()
}, None).await?;
let caps = surface.get_capabilities(&adapter);
let config = velvet_platform::gpu::surface_config(&caps, size.width, size.height).unwrap();
surface.configure(&device, &config);
```

- `instance_descriptor()`: on Android one backend, Vulkan when the device has a hardware Vulkan
  adapter and OpenGL ES otherwise (software Vulkan, as on emulators, counts as none); wgpu's
  defaults elsewhere; `WGPU_BACKEND` still overrides it. Never enable both on Android: each
  makes its own surface, the Vulkan loader connects the window first, and EGL then fails with
  "already connected to another API" (wgpu: "Invalid surface"). `request_adapter()` takes the
  instance's first adapter that can present to the surface.
- Window flags (`FLAG_KEEP_SCREEN_ON`) go through `Window.addFlags` on the UI thread, never
  `AndroidApp::set_window_flags`: android-activity 0.6 takes its app lock for writing there
  while `poll_events` holds it for reading, so calling it from an event handler deadlocks.
- The C++ runtime: dependencies with C++ code are linked against the NDK's static libc++
  (`-lc++_static -lc++abi`), and `velvet android build` refuses a library that still imports
  C++ runtime symbols no system library has (Android's `libstdc++.so` is an empty stub).
- `SurfaceSlot` stands in for `wgpu::Surface`: `configure` and `get_current_texture` work the
  same; while suspended it answers `SurfaceError::Lost`, so the usual "lost: reconfigure and skip
  the frame" path is right. Device, pipelines and textures survive suspend and resume.
- `supported_limits()`: `Limits::default()` asks more than many phone GPUs give (and far more than
  OpenGL ES); it picks the best tier the adapter supports.
- Mobile GPUs have no BC texture compression (`TEXTURE_COMPRESSION_BC`): ship ETC2 or ASTC
  textures or expect RGBA8 (four times the memory of BC7). Compute shaders need Vulkan or OpenGL
  ES 3.1.
- OpenGL ES cannot read a depth texture texel by texel (GLSL has no `texelFetch` for shadow
  samplers, so naga refuses WGSL's `textureLoad` on `texture_depth_2d`). `velvet-render`'s passes
  that do (the ink outline, the bounce light, the cached shadow map's restore) bind the depth as
  an unfilterable float texture there instead; your own shaders need the same on GL.
- DX12 and DXC are Windows-only: keep backend and shader-compiler choices behind
  `cfg!(target_os = "windows")`.

### Packaged files and saves

```rust
use velvet_platform::files;

files::set_assets_root(game_dir);          // desktop only: the folder that becomes assets/
let assets = files::assets();
let scene = assets.read_to_string("scenes/arena.json")?;
let music = assets.open("music/title.ogg")?;     // streams (Read + Seek + Send + Sync)
for entry in assets.list_dir("levels")? { /* entry.name, entry.is_dir */ }
assert!(assets.is_dir("levels") && assets.exists("levels/1.json"));
let model = velvet_render::GltfModel::load_with("models/hero.gltf", &|p| assets.read(p))?;

let saves = files::data_dir("Studio/Game")?;      // per user; app storage on Android
files::write_atomic(&saves.join("slot1.json"), &bytes)?;
let cache = files::cache_dir("Studio/Game")?;     // the system may clear it
let big = files::external_data_dir("Studio/Game")?; // large downloaded content
```

- Paths are relative, with `/`. On desktop the root is `set_assets_root`, else `VELVET_ASSETS_DIR`,
  else `assets/` beside the executable, else `./assets`. On Android it is the APK's `assets/`
  (including install-time asset packs, which the system merges into the same asset manager).
- `list_dir` and `is_dir` use the index the packager writes (the NDK cannot list sub-folders).
- `open` streams entries stored uncompressed straight from the APK (the packager stores media
  so); compressed entries are read into memory.
- `assets().source()` is a velvet-assets `Source` for the `Assets` registry and `AssetStreamer`.
- `data_dir`: `%APPDATA%\<app>`, `~/Library/Application Support/<app>`, `$XDG_DATA_HOME/<app>`;
  Android `/data/user/0/<package>/files` (private, removed with the app, included in Android's
  backup). `VELVET_DATA_DIR` overrides it on desktop.
- `std::env::current_exe()` and paths beside the executable mean nothing on Android.

### Input

```rust
use velvet_platform::touch::{TouchMouse, Touches, Gestures, GestureConfig, Gesture};

// A game written for the mouse: the first finger moves the cursor and holds the left button.
for mouse_event in self.touch_mouse.translate(&event) {
    self.handle(mouse_event);
}

// Fingers and gestures.
if let WindowEvent::Touch(touch) = &event {
    if let Some(change) = self.touches.handle(touch) {
        for gesture in self.gestures.on_change(&change, &self.touches) {
            match gesture {
                Gesture::Tap { fingers: 2, .. } => self.back(),
                Gesture::Pinch { scale, .. } => self.zoom *= scale,
                Gesture::Drag { delta, fingers: 2 } => self.scroll(delta),
                _ => {}
            }
        }
    }
}
if let Some(Gesture::LongPress { position }) = self.gestures.poll(&self.touches, Instant::now()) {}
```

- winit delivers `WindowEvent::Touch` and never mouse events for fingers. `TouchMouse` makes
  `CursorMoved`/`MouseInput` from the first finger, `CursorLeft` after a release (no stuck hover)
  and before the release of a cancelled touch (so click-on-release UIs ignore it). Call
  `reset()` on suspend.
- `GestureConfig::for_scale_factor(window.scale_factor())` sets thresholds in dp.
- The back button: `velvet_platform::input::is_back_key(&key_event)` (its physical key is
  `Unidentified(Android(4))`, not `Escape`), or `PlatformEvent::Back` from `Lifecycle`.
  `velvet-host2d` turns it into Escape.
- Gamepads: gilrs has no Android backend (it builds and finds none). Buttons arrive as key events
  (`velvet_platform::input::gamepad_button`), the D-pad as arrow keys; winit 0.30 forwards no
  analog sticks or triggers.
- Text: NativeActivity has no input connection. `screen::show_keyboard(true)` (or
  `Window::set_ime_allowed(true)`) shows the soft keyboard, whose key events carry `text` for
  plain characters; composing input methods (CJK, swipe typing) and autocorrection do not work.
  Prefer an in-game keyboard for names.

### Screen

- `screen::safe_area()`: insets in window pixels (visible system bars and display cutouts), read
  on resume, focus and resize by `Lifecycle`. Apps targeting API 35+ are drawn edge to edge, so
  keep buttons and text inside `Insets::safe_rect(width, height)`.
- `screen::set_immersive(true)`: hides status and navigation bars (a swipe from the edge shows
  them for a moment).
- `screen::set_orientation(Orientation::SensorLandscape)`; the manifest's `orientation` is the
  default. Android 16 ignores orientation on large screens except for apps declared as games,
  which the Velvet manifest does.
- `screen::set_keep_screen_on(true)`.
- `Window::scale_factor()` is the density / 160 (2.6 to 3.5 on phones): size touch targets in dp
  (48 dp at least).

### Language

`velvet_platform::system_locale()` gives the user's language as a BCP 47 tag (`es-ES`, `pt-BR`)
on every platform: Android's configuration, Windows' preferred UI language, `LC_ALL`/`LANG`
elsewhere. Android sets no `LANG` variable.

### Audio

rodio 0.21 plays through cpal's AAudio backend (`min_sdk` 26). No extra feature or library is
needed (`libaaudio.so` is a system library). AAudio streams keep running in the background: pause
your sinks on `Suspended` and resume them on `Resumed`. When the output device changes
(headphones), cpal reports a stream error and a new `OutputStream` must be opened.

### 2D games on velvet-host2d

`velvet_host2d::run` does all of the above: the first finger is the pointer, back is Escape,
the surface follows suspend and resume, the screen is kept full and on, and
`Game2D::platform_event` reports `Suspended`, `Resumed`, focus, low memory and `Back`. See
`examples/hello-mobile`.

## Porting checklist for a big game

Roughly in order of effort, smallest first:

1. **Library and entry point.** Move `main` into `lib.rs` as `run()`, add `android_main!`, add
   `[package.metadata.velvet.android]`. Replace `EventLoop::new()` with
   `velvet_platform::event_loop()`.
2. **Windows-only code.** Gate `windows`/`windows-sys` dependencies and their call sites
   (`[target.'cfg(windows)'.dependencies]`, `#[cfg(windows)]`): message boxes, DPI awareness,
   process and memory queries, window icons. Give Android a fallback (logcat, no-op). DX12 and
   DXC selection must stay behind `cfg!(target_os = "windows")`. `build.rs` steps that embed
   Windows resources must check `CARGO_CFG_TARGET_OS`.
3. **Window assumptions.** No window size, position, title, cursor, maximise or fullscreen toggle
   on Android: the window is the screen. Handle `Resized` (rotation, split screen, foldables) and
   the safe area.
4. **Saves and settings.** Replace `%APPDATA%`/executable-relative folders with
   `files::data_dir`; write with `files::write_atomic`; save on `Suspended`. Take the first-run
   language from `velvet_platform::system_locale()` instead of Win32 calls or `LANG`.
5. **Packaged files.** Replace `std::fs::read`/`read_dir`/`is_file` on shipped files with
   `files::assets()`; models through `GltfModel::load_with`; large media through `open()`
   (streamed). Every desktop path built from `current_exe()` or `CARGO_MANIFEST_DIR` must become
   a relative asset path.
6. **Surface lifecycle.** Keep the instance, device and resources; drop and re-create only the
   surface (`SurfaceSlot`). Stop rendering and simulation while suspended; don't busy-loop.
7. **Audio.** Pause on `Suspended`, resume on `Resumed`; re-open the output after a device change.
8. **Input and UI for touch.** Mouse-driven UIs work through `TouchMouse` for clicks and drags, but
   there is no hover and no right button: give right-click actions a long press, a two-finger tap
   or a button; give every keyboard shortcut (menus, quick save, fullscreen, debug keys) an
   on-screen equivalent; map Escape/back to the back button; make targets at least 48 dp; check
   text sizes at phone scale.
9. **Render presets for mobile GPUs.** A mobile preset: lower internal resolution (render at 0.6
   to 0.75 and upscale), fewer or smaller shadow maps, no or cheaper MSAA, lighter post-processing
   (bloom, HDR targets cost bandwidth on tiled GPUs), capped frame rate (30/60), smaller texture
   sizes; `supported_limits` instead of `Limits::default()`; no BC textures (ETC2/ASTC or RGBA8);
   watch memory: phones share 4 to 12 GB between CPU and GPU and kill apps that take too much.
   Test on Adreno, Mali and PowerVR GPUs; thermal throttling lowers clocks after minutes.
10. **Video.** Platform video decoders (Media Foundation, AVFoundation) do not exist on Android:
    use the NDK's `AMediaCodec`/`AMediaExtractor` (the `ndk` crate's `media` feature; hardware
    H.264/HEVC everywhere, AV1 on recent chips) or a software decoder at a lower resolution.
11. **Size and delivery.** Measure the packaged size. Google Play (as of 2026-10): base module up to
    500 MB compressed, each asset pack up to 1.5 GB, 4 GB of install-time content in total; larger
    content needs fast-follow or on-demand asset packs (Play Asset Delivery library, not shipped
    by this pipeline). Install-time asset packs (`asset_packs`) are read like any packaged file.
    A sideloaded APK holds everything in one file: under 4 GB and 65,535 entries (no ZIP64), and the
    install needs about twice its size free. Store media uncompressed (the default) so it streams;
    pack many small files into archives.
12. **Store requirements.** Target SDK 36, 64-bit libraries (arm64-v8a), 16 KB page alignment
    (checked by the pipeline), app bundles for Google Play, the content and data-safety
    declarations of each store your game goes to.

## Known limits

- **Not run on a phone yet.** Everything above builds, packages, signs and verifies, and a large
  3D game on `velvet-render` has run on the x86_64 emulator over OpenGL ES 3.1 (lifecycle, back
  button, audio pause, saves); behaviour on phones (insets, input, Vulkan and mobile GPU drivers,
  performance) still has to be checked on real devices.
- **winit 0.30 lifecycle.** No pause/stop/destroy/save-state callbacks; one event loop per process
  (the process ends after the game returns). An activity destroyed by the system while the game
  runs (an unhandled configuration change, "Don't keep activities" in developer options) cannot
  be survived; the manifest lists every configuration change to avoid that.
- **Predictive back.** Apps targeting API 36 get no back key events unless they opt out; the
  manifest sets `android:enableOnBackInvokedCallback="false"`, which Android 16 still honours. A
  future target SDK will require handling back through `OnBackInvokedCallback` (Java side).
- **NativeActivity** has no text input connection (see Input). GameActivity would bring one, but
  needs its Java library (AAR) and Gradle.
- **rav1d without assembly on Android.** The ARM assembly does not build from the crates.io
  package (1.1.0 leaves out its `.h` files) and the x86 assembly does not link into a shared
  library, so `velvet-avif` decodes AV1 (AVIF pictures, IVF clips) with Rust code only on Android:
  several times slower than with assembly. Desktop x86 keeps the assembly.
- **Fast-follow and on-demand asset packs** need the Play Asset Delivery library and Gradle;
  only install-time packs are supported.
- **No x86 32-bit or RISC-V** testing; armeabi-v7a builds but is not part of the samples.
