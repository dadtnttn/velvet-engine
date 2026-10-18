# velvet-desktop

Velvet Desktop runs a JavaScript game as a desktop program. A native window hosts the system
webview (WebView2 on Windows). The game's folder is served through a custom protocol, and a small
native bridge gives the game `window.velvet`: saves, settings, window control, logs, pictures of
the page and links to the player's own programs.

## Run the example

From the repository root:

```sh
cargo run -p velvet-desktop -- crates/velvet-desktop/examples/hello-game
```

The window shows the example page. Its buttons print the bridge info, save, load, list and remove
a save, toggle fullscreen, and write log lines (those go to the terminal). Windows needs the
WebView2 runtime, which Windows 11 and Microsoft Edge already include.

## Game folder

```text
my-game/
  velvet-game.json   manifest (required)
  index.html         the entry page, and everything else the game loads
```

`velvet-game.json`: only `id` and `title` are required. Unknown keys are an error.

```json
{
  "id": "hello-game",
  "title": "Hello Velvet",
  "version": "0.1.0",
  "entry": "index.html",
  "window": {
    "width": 1280, "height": 720, "min_width": 960, "min_height": 540,
    "resizable": true, "fullscreen": false
  }
}
```

| Field | Default | Rule |
|---|---|---|
| `id` | required | `^[a-z0-9][a-z0-9_-]{0,63}$`, not a Windows device name (`con`, `com1`...) |
| `title` | required | 1 to 120 characters, no control characters |
| `version` | `"0.1.0"` | 1 to 64 characters |
| `entry` | `"index.html"` | relative path of letters, digits, `.`, `_` and `-`, separated by `/`; no `..`, no leading `/` |
| `window.width`, `window.height` | 1280, 720 | 1 to 16384 |
| `window.min_width`, `window.min_height` | 960, 540 | not larger than the size |
| `window.resizable` | `true` | |
| `window.fullscreen` | `false` | starts in borderless fullscreen |

A leading byte order mark is accepted, since Windows editors often write one.

## Command line

```text
velvet-desktop <game-folder> [--dev <url>] [--devtools] [--size <W>x<H>]
                             [--capture <file.png> [--after <ms>]]
velvet-desktop                      (no arguments: runs the packaged game, see below)
```

- `--dev <url>` loads a dev server, for example `http://localhost:5173`, instead of the folder's
  files. The manifest is still read from the folder, and the bridge is available on that page too.
- `--devtools` opens the web inspector. Debug builds always have it.
- `--size <W>x<H>` sets the window's client size in device pixels, for example `1600x900`, and
  replaces the manifest's size. Each side is from 64 to 7680. Such a window is never full screen.
  On a screen scaled to 150%, the picture can be one pixel wider than `W`, because WebView2 rounds
  the page to whole logical pixels. The log line gives the picture's real size.
- `--capture <file.png>` takes a picture of the page into that PNG file and then exits. The run
  waits for the page to finish loading, then for `--after`. It exits 0 when the file is written,
  and 1 with a message (in the terminal and in the log file) when it is not. Missing folders above
  the file are created. A release folder is captured with its `game` folder, for example
  `hello-game.exe --capture shot.png game`.
- `--after <ms>` is the wait after the page loads, before the picture: from 0 to 600000, default
  2000. It needs `--capture`.
- The window does not bind Escape. The game decides what it does.

Pictures and the links below work on Windows for now. Elsewhere they fail with the code
`unsupported`.

For QA, a screenshot of a game folder at a fixed size:

```sh
velvet-desktop --capture shot.png --after 3000 --size 1600x900 <game dir>
```

## Release folders

`velvet build --runtime js` makes a release folder, `<game>/release/<id>-<version>/`:

```text
hello-game-0.1.0/
  hello-game.exe     the host (no extension on Linux and macOS)
  game/
    velvet-game.json
    index.html ...   the web files
```

Started with no arguments, the host looks for `game/velvet-game.json` next to its own executable
and runs that folder, exactly as `velvet-desktop <folder>/game` would. Without that folder it
prints the usage. The host's file name does not matter: the data folder and the log file come from
the manifest's `id`. Run the example as a release with:

```sh
cargo run -p velvet-cli -- build --runtime js --path crates/velvet-desktop/examples/hello-game
```

## The bridge: `window.velvet`

The bridge is defined before the page's own scripts run, on the game's own origin only. The
contract, as TypeScript:

```ts
interface VelvetBridge {
  platform: "desktop";
  game: { id: string; title: string; version: string };
  saves: { list(): Promise<string[]>; read(slot: string): Promise<string | null>; write(slot: string, text: string): Promise<void>; remove(slot: string): Promise<void> };
  settings: { get(key: string): Promise<unknown>; set(key: string, value: unknown): Promise<void> };
  window: {
    setFullscreen(on: boolean): Promise<void>; isFullscreen(): Promise<boolean>; close(): Promise<void>;
    capture?(): Promise<string>;  // a PNG data URL of the page; a host from before it lacks it
  };
  shell?: { openExternal(url: string): Promise<void> };  // a host from before it lacks it
  log(level: "debug" | "info" | "warn" | "error", message: string): void;
}
```

- A failed call rejects with an `Error` whose `code` is one of `invalid_name`, `too_large`,
  `corrupt_settings`, `io`, `no_data_folder`, `bad_args`, `unknown_command`, `bad_message`,
  `invalid_url`, `open_failed`, `unsupported`, `capture_failed`, `timeout` or `internal`.
- `saves.read` of an empty slot and `settings.get` of a missing key resolve to `null`.
- `saves.remove` of an empty slot is not an error.
- `window.close()` ends the program.
- `window.capture()` resolves to a PNG of the page as a data URL, `data:image/png;base64,...`. A
  picture over 8 MiB of PNG is refused with `too_large`. A picture that does not come back within
  30 seconds is refused with `timeout`. A failure in WebView2 is `capture_failed`.
- `shell.openExternal(url)` opens the link in the player's own browser or mail program. Only
  `https:`, `http:` and `mailto:` addresses are opened. The host refuses anything else with
  `invalid_url`: an address over 2048 bytes, a space, a control character, a non-ASCII character,
  a quote, `<`, `>`, `\`, `^`, a backtick, `{`, `|` or `}`, an address with no host name, and one
  with user information before the host (`user@host`). Nothing is opened for a refused address.
  When the system refuses the call, the code is `open_failed`.
- `log` writes `[game-id] level: message` to the terminal. Messages over 8 KiB are cut.
- Navigation to other sites and pop-up windows are refused, so no outside page gets the bridge.

## Data folder

| Platform | Folder |
|---|---|
| Windows | `%APPDATA%\Velvet\<game-id>` |
| macOS | `~/Library/Application Support/Velvet/<game-id>` |
| Linux and others | `$XDG_DATA_HOME/velvet/<game-id>`, or `~/.local/share/velvet/<game-id>` |

- Saves are `saves/<slot>.json`. Slots are 1 to 64 characters of `A-Z a-z 0-9 _ -`, and not a
  Windows device name. A save is at most 16 MiB.
- Settings are one JSON object in `settings.json`. Keys are 1 to 128 characters of
  `A-Z a-z 0-9 . _ -`. The file is at most 1 MiB, and a file that is not a JSON object is reported,
  never overwritten.
- Every write goes to a temporary file that then replaces the target. On Windows, slot names are
  case-insensitive, as the file system is.

## Serving

- Windows: pages load from `https://velvet.game/` (https, so the page is a secure context and
  `crypto.subtle` works). macOS and Linux: `velvet://game/`.
- A path is percent-decoded and must be a plain relative path. `..`, drive letters, backslashes and
  control characters are refused. A file that resolves outside the game folder, including through
  a symbolic link, is refused. Refused and missing files get a 404.
- MIME types come from a fixed table (html, js, mjs, css, json, the common image, audio and font
  types, wasm, glb and gltf). Other extensions are `application/octet-stream`.
- Files are read whole into memory and sent with `Cache-Control: no-store`.

## Windows processes

WebView2 starts its processes (the browser, the renderers, the helpers) as children of the host. At
start-up the host joins a job object that ends its processes when the host ends, however it ends
(`src/job.rs`, `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`). Without that, a host that is killed rather than
closed can leave the browser running (reproduced by ending the host with the msys `timeout` command),
and the browser keeps the game's profile in use, so the next start fails. If the job cannot be set
up, the host runs without it and logs why.

## Tests

```sh
cargo test -p velvet-desktop
cargo clippy -p velvet-desktop --all-targets -- -D warnings
```

The unit tests cover the manifest, the file server, the storage, the bridge commands, the command
line (including the `--size`, `--capture` and `--after` limits), the packaged-game lookup, the
picture checks (PNG size, the data URL and its size limit) and the link rules. No test opens a
link: refused addresses are the only ones the tests pass to the link call. The window and the
webview need a desktop, so they are checked by running the example, or a `--capture` run. The
symbolic-link test runs on Unix only.

## Known limits

- A release build on Windows has no console. Its messages and the game's `velvet.log` lines go to
  `velvet-desktop.log` in the data folder (rewritten on each run); debug builds also print them
  in the terminal. A start-up error (such as a missing WebView2 runtime) is only in that file.
  Each finished page load is noted there too (`page loaded: <url>`).
- The webview's profile (cache, local storage) is kept in `<data>/webview`, not next to the
  program.
- Pictures (`--capture`, `window.capture()`) and `shell.openExternal` work on Windows only, for now.
  The checks (the link rules, the PNG size and the size limit) are plain code and run everywhere.
  Only the calls into the system are Windows-only, and elsewhere they answer `unsupported`.
