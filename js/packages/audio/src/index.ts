// @velvet/audio: the sounds of a Velvet JS game, in the velvet-audio format (docs/formats/AUDIO.md).
// A game asks for a sound by id, never by file path. The manifest says which files play, on which
// bus, at what volume, and with which limits. The engine loads the files on first use, keeps the
// mix (buses, mutes and ducking), runs the music channel, and follows the game clock's pause, the
// page's visibility and the browser's rule for starting audio.
//
// Import from the package root, for example:
//   import { createAudio, parseAudioManifest } from "@velvet/audio";

export { AUDIO_FORMAT, AUDIO_FORMATS, AUDIO_MIME, AUDIO_VERSION, BUILT_IN_BUSES, MASTER_BUS, isBuiltInBus } from "./types";
export type { AudioFormat, AudioLoopPoints, AudioManifest, AudioSound, AudioVariant, BuiltInBus } from "./types";
export { AudioError } from "./error";
export type { AudioErrorCode, AudioIssue } from "./error";
export { parseAudioManifest, parseAudioManifestLenient } from "./manifest";
export type { LenientAudioManifest } from "./manifest";
export { decibelsToGain, volumeToDecibels, volumeToGain } from "./volume";
export { createAudio } from "./engine";
export { defaultSettingsStore } from "./settings";
export type {
  AudioEngine,
  AudioEvents,
  AudioHost,
  AudioOptions,
  AudioSettingsStore,
  DuckingOptions,
  EndReason,
  FetchLike,
  FetchResponseLike,
  MusicOptions,
  PlaybackHandle,
  PlaybackStatus,
  PlayOptions,
  PreloadProgress,
  PreloadResult,
} from "./api";
export type {
  AudioBufferLike,
  AudioBufferSourceLike,
  AudioContextLike,
  AudioNodeLike,
  AudioParamLike,
  GainNodeLike,
  StereoPannerLike,
} from "./context";
