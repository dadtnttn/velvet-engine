// @velvet/core: the runtime that every Velvet JS game uses.
//
// Import from the package root, for example:
//   import { GameLoop, ActionMap, createSaveStore } from "@velvet/core";

export { GameLoop, createFrameScheduler } from "./loop";
export type { FrameScheduler, GameLoopOptions } from "./loop";
export { GameClock } from "./clock";
export type { GameClockOptions, GameClockState } from "./clock";
export { EventEmitter } from "./events";
export type { EmitArgs, Listener, Unsubscribe } from "./events";
export { ActionMap } from "./input";
export type {
  ActionMapOptions,
  Binding,
  Control,
  KeyCode,
  MouseButtonName,
  Vector2,
  WheelDirection,
} from "./input";
export { SLOT_PATTERN, getBridge, isValidSlot } from "./bridge";
export type { VelvetBridge } from "./bridge";
export { createSaveStore } from "./saves";
export type { LoadedSave, SaveBackendKind, SaveStore, SaveStoreOptions } from "./saves";
export { SaveError } from "./save-error";
export type { SaveErrorCode } from "./save-error";
export { SAVE_FORMAT } from "./save-format";
export type { Migration, MigrationMap, SaveEnvelope } from "./save-format";
export {
  createBridgeBackend,
  createLocalStorageBackend,
  createMemoryBackend,
  getLocalStorage,
} from "./save-backends";
export type { SaveBackend, StorageLike } from "./save-backends";
