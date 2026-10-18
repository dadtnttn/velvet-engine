# velvet-desktop

Velvet Desktop runs a JavaScript game as a desktop program. A native window hosts the system
webview (WebView2 on Windows). The game's folder is served through a custom protocol, and a small
native bridge gives the game `window.velvet`: saves, settings, window control and logs.

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
velvet-desktop <game-folder> [--dev <url>] [--devtools]
```

- `--dev <url>` loads a dev server, for example `http://localhost:5173`, instead of the folder's
  files. The manifest is still read from the folder, and the bridge is available on that page too.
- `--devtools` opens the web inspector. Debug builds always have it.
- The window does not bind Escape. The game decides what it does.

## The bridge: `window.velvet`

The bridge is defined before the page's own scripts run, on the game's own origin only. The
contract, as TypeScript:

```ts
interface VelvetBridge {
  platform: "desktop";
  game: { id: string; title: string; version: string };
  saves: { list(): Promise<string[]>; read(slot: string): Promise<string | null>; write(slot: string, text: string): Promise<void>; remove(slot: string): Promise<void> };
  settings: { get(key: string): Promise<unknown>; set(key: string, value: unknown): Promise<void> };
  window: { setFullscreen(on: boolean): Promise<void>; isFullscreen(): Promise<boolean>; close(): Promise<void> };
  log(level: "debug" | "info" | "warn" | "error", message: string): void;
}
```

- A failed call rejects with an `Error` whose `code` is one of `invalid_name`, `too_large`,
  `corrupt_settings`, `io`, `no_data_folder`, `bad_args`, `unknown_command` or `bad_message`.
- `saves.read` of an empty slot and `settings.get` of a missing key resolve to `null`.
- `saves.remove` of an empty slot is not an error.
- `window.close()` ends the program.
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

## Tests

```sh
cargo test -p velvet-desktop
cargo clippy -p velvet-desktop --all-targets -- -D warnings
```

The unit tests cover the manifest, the file server, the storage, the bridge commands and the
command line. The window and the webview need a desktop, so they are checked by running the example.
The symbolic-link test runs on Unix only.

## Known limits

- A release build on Windows has no console. Its messages and the game's `velvet.log` lines go to
  `velvet-desktop.log` in the data folder (rewritten on each run); debug builds also print them
  in the terminal. A start-up error (such as a missing WebView2 runtime) is only in that file.
- The webview's profile (cache, local storage) is kept in `<data>/webview`, not next to the
  program.
- Release packaging (a host executable next to the built game) is not part of this crate yet.
