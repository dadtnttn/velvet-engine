# @velvet/audio

The sounds of a Velvet JS game, in the `velvet-audio` format (`docs/formats/AUDIO.md`). A game asks for a
**sound id** (`sfx.door.open`), never for a file. The manifest says which files play, on which bus, at what
volume, and with which limits. The engine loads each file on first use, keeps the mix, runs the music
channel, and follows the game clock's pause, the page's visibility and the browser's rule for starting
audio. Nothing in the package is tied to one game.

## A game

```ts
import { GameClock, GameLoop } from "@velvet/core";
import { createAudio, parseAudioManifest } from "@velvet/audio";

// The manifest, from the audio folder. The text can come from a fetch, a bundler or the desktop bridge.
const manifest = parseAudioManifest(await (await fetch("audio/manifest.json")).text());
const clock = new GameClock();
const loop = new GameLoop({ update: (dt) => clock.advance(dt) });
// The engine follows the clock's pause events and the loop's frame hook by itself: no call per frame is needed.
const audio = createAudio({ manifest, clock, loop });

// Decode what the first scene needs. The callback drives a loading bar.
const { failed } = await audio.preload(["ui.click", "music.village.day"], ({ done, total }) => setProgress(done / total));
if (failed.length > 0) console.warn("could not load:", failed);

audio.on("error", ({ id, message }) => console.warn(id, message));
audio.on("music-changed", ({ id }) => console.log("music is now", id));

// Music: the new track fades in while the old one fades out, and it loops as the manifest says.
audio.playMusic("music.village.day", { crossfadeMs: 1500 });

// Effects: each play returns a handle, which the game can stop, fade or wait on.
const step = audio.play("sfx.footstep.stone", { pan: -0.3, rate: 1.05 });
const reason = await step.ended; // "finished", "stopped", "stolen", "skipped" or "failed"

// The mix: volumes run from 0 to 1 on a decibel curve. Mutes and volumes are saved for the next start.
audio.setVolume("music", 0.6);
audio.setMuted("ui", false);

loop.start();
```

A game that runs in a browser calls `audio.unlock()` from its own "start" button, or it simply waits for the
first press: the engine unlocks on the first press, key or touch by itself.

Without a `loop` option the engine still follows the clock's events, and the page's visibility. A game that
drives its own frames can call `audio.update()` once per frame instead; it is optional, and it is cheap.

## API

| Call | What it does |
|---|---|
| `parseAudioManifest(json)` | Reads the text of the manifest. Throws `AudioError`; its `issues` list every problem, each with a path. |
| `parseAudioManifestLenient(json)` | Reads it leniently: the bad custom buses and sounds are left out and reported; the envelope is still checked. Returns `{ manifest, issues }`. |
| `createAudio(options)` | The engine. Only `manifest` is required. The options are listed below. |
| `play(id, options?)` | Plays a sound: `{ volume, rate, detune, pan, loop, delay }`. Returns a handle with `status`, `volume`, `ended`, `stop(fadeMs?)` and `fade(to, ms)`. |
| `playMusic(id, options?)` | Starts a track on the music channel, `{ crossfadeMs, startAt }`. The sound must be on the `music` bus. |
| `stopMusic(fadeMs?)`, `stopAll(fadeMs?)` | Stops the music, or every play. |
| `currentMusic` | The id of the music that plays or waits to play, or null. |
| `preload(ids?, onProgress?)` | Decodes sounds, and resolves to `{ failed }`. With no ids, the sounds that the manifest marks with `preload`. |
| `isLoaded(id)`, `unload(ids?)` | Whether a sound is decoded; drops decoded files, all of them with no ids. |
| `setVolume(bus, v)`, `getVolume(bus)` | The volume of a bus, or of `"master"`, from 0 to 1. |
| `setMuted(bus, muted)`, `isMuted(bus)` | Mutes a bus, or `"master"`. |
| `unlock()` | Asks the browser to start the audio. Resolves `true` once it runs. |
| `update()` | Optional. Checks the clock's pause, the page's visibility and the start of the audio. The engine already follows the clock's events and, with a `loop`, the loop's frame hook; calling it runs the same check by hand, and it is cheap when nothing changed. |
| `on`, `once`, `off` | The events `loaded`, `error` and `music-changed`, through the core event system. |
| `ready` | Resolves when the saved mix has been read. Never rejects. |
| `unlocked` | True once the browser has let the audio run. |
| `close()` | Stops every play, releases the page's listeners and closes the context. |
| `volumeToGain`, `volumeToDecibels`, `decibelsToGain` | The volume curve, for tools and tests. |
| `defaultSettingsStore()` | The backend that the engine's mix is saved in by default: the bridge's settings, else localStorage, else null. |

The options of `createAudio`: `baseUrl` (default `audio/`), `fetch`, `createContext` (the audio context
factory), `canPlay` (which formats the runtime plays), `clock` (a `GameClock`, or anything with a `paused`
flag; a clock with `on` is followed through its `paused` and `resumed` events), `loop` (a `GameLoop`, whose
`beforeFrame` hook checks the audio each frame), `pauseWith` (`"game-buses"` by default, or `"context"`),
`host` (the page: visibility and gestures), `settings` (where the mix is saved; `null` keeps it for the
session), `ducking` (or `false`), `now`, `random`, `queueLimit`, `staleAfterMs` and `suspendWhenHidden`. Every
option has a default, and the defaults are the browser's own.

The package also exports `AudioError` (with `code` and `issues`), the format constants (`AUDIO_FORMAT`,
`AUDIO_VERSION`, `AUDIO_FORMATS`, `AUDIO_MIME`, `BUILT_IN_BUSES`, `MASTER_BUS`, `isBuiltInBus`) and the types
of the manifest, the handles, the options, the clock and loop shapes (`AudioClock`, `AudioLoop`) and the
context that a custom host implements.

## Rules to know

- **Autoplay.** Plays made before the first gesture wait in a queue. The first press, key or touch starts them.
  The queue keeps the newest 32 plays, and a play that waits more than 3 seconds is skipped.
- **Pause.** While the clock is paused, the music, sfx and voice buses are muted over a short ramp, and `ui`
  keeps sounding. With `pauseWith: "context"` the whole context is suspended instead. The change follows the
  clock's `paused` and `resumed` events at once, and a clock that is already paused when the engine starts
  mutes the buses from the start.
- **Hidden page.** A hidden page suspends the context, and a visible one resumes it. Pass
  `suspendWhenHidden: false` to keep it running.
- **Ducking.** While a `voice` sound plays, the `music` bus drops by 12 dB, over 60 ms, and comes back over
  400 ms after the last voice ends. Pass `ducking: { trigger, target, decibels, attackMs, releaseMs }` to
  change it, or `ducking: false` to turn it off.
- **Volumes.** A volume from 0 to 1 is a gain of its cube, so 0.5 is about -18 dB and 0.1 is -60 dB. The
  levels of a sound, a play, a bus and the master multiply.
- **Cooldowns and voice limits** belong to a sound id. A play inside a cooldown ends at once as `skipped`.
  A sound with `max_voices` steals its oldest copy when a new one would pass the limit.
- **Variants** are picked at random, and the one that played last is never picked again.
- **Loop points** are clamped to the file's length. A start offset (`startAt`) is clamped the same way.
- **Settings.** The mix is read at start, and written after changes, through the core settings store under the
  namespace `audio`. A burst of changes is one write, made 250 ms after the last change; `close()` writes what is
  still pending. In Velvet Desktop the mix is kept in the bridge's `settings` object under `audio`, in a browser
  in localStorage under `velvet-settings:audio`. A mix that an older build left in the bridge is still read; the
  localStorage key changed, so a mix saved in a browser by an older build is not. Pass `settings: getBridge()?.settings` or your own store with the same `get` and `set` shape to choose, or
  `null` to keep the mix for the session only.
- **Loading.** A file is fetched once and decoded once. The candidates of a sound are tried in order, and a
  file that is missing or does not decode moves on to the next one. A sound that loads nowhere reports an
  `error` event, and its play ends as `failed`.
- **Events.** `loaded` is sent for each variant that is decoded. `error` names the sound and the reason.
  `music-changed` is sent when a track starts, and when it stops or ends on its own.

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. The tests run on a fake Web Audio context in
`src/test-support.ts`, which records the graph and every scheduled value, and on the conformance suite in
`docs/formats/fixtures/audio/`. Nothing needs a browser.
