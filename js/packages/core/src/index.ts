// @velvet/core: the runtime that every Velvet JS game uses.
//
// Import from the package root, for example:
//   import { GameLoop, ActionMap, createSaveStore } from "@velvet/core";

export { GameLoop, createFrameScheduler } from "./loop";
export type {
  FrameScheduler,
  FrameStart,
  FrameTiming,
  GameLoopOptions,
  LoopHooks,
  LoopPhase,
  RenderEnd,
  StepEnd,
  StepStart,
} from "./loop";
export { GameClock } from "./clock";
export type { GameClockEvents, GameClockOptions, GameClockState, GameClockStatus } from "./clock";
export { EventEmitter } from "./events";
export type { EmitArgs, Listener, TapEvent, TapListener, Unsubscribe } from "./events";
export { ActionMap, markHandled } from "./input";
export type {
  ActionMapOptions,
  Binding,
  BindingDisplay,
  CaptureOptions,
  CapturedInput,
  Control,
  ConflictGroup,
  ConflictPolicy,
  GamepadChange,
  GamepadInfo,
  InputDevice,
  KeyCode,
  MouseButtonName,
  PadAxisName,
  PadButtonName,
  PadControl,
  PadDirection,
  PadStickName,
  RebindConflict,
  RebindOutcome,
  RebindStatus,
  RumbleOptions,
  Vector2,
  WheelDirection,
} from "./input";
export { describeBinding } from "./input-controls";
export {
  DEFAULT_DEADZONE,
  DEFAULT_PAD_THRESHOLD,
  PAD_AXIS_NAMES,
  PAD_BUTTON_NAMES,
  PAD_DIRECTION_NAMES,
  PAD_STICK_NAMES,
} from "./input-pads";
export { createRebindPort } from "./input-port";
export type { RebindCaptureRequest, RebindPort, RebindPortOutcome, RebindPortSlot } from "./input-port";
export { startPadMenuKeys } from "./pad-keys";
export type { PadKeyMapping, PadMenuKeysOptions } from "./pad-keys";
export { BINDINGS_FORMAT, BINDINGS_KEY, BINDINGS_VERSION, applyBindings, createBindingsStore, parseBindings, serializeBindings } from "./input-store";
export type {
  ApplyBindingsResult,
  BindingsBackendKind,
  BindingsFile,
  BindingsFileAction,
  BindingsFileSlot,
  BindingsStore,
  BindingsStoreOptions,
  ParseBindingsOptions,
} from "./input-store";
export {
  BridgeUnavailableError,
  SLOT_PATTERN,
  captureScreenshot,
  getBridge,
  isValidSlot,
  openExternalLink,
} from "./bridge";
export type { BridgeUnavailableCode, VelvetBridge } from "./bridge";
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
export { createSettingsStore, SETTINGS_FORMAT, SETTINGS_STORAGE_PREFIX } from "./settings-store";
export type { SettingsStore, SettingsStoreOptions } from "./settings-store";
export { chooseSettingsBackend } from "./settings-backend";
export type {
  ChosenSettingsBackend,
  SettingsBackend,
  SettingsBackendChoice,
  SettingsBackendKind,
} from "./settings-backend";
export {
  LOG_LEVELS,
  createBridgeSink,
  createConsoleSink,
  createLogger,
  isLogLevel,
} from "./log";
export type {
  ConsoleLike,
  JsonObject,
  JsonValue,
  LogEntry,
  LogFields,
  LogLevel,
  LogSink,
  LogTarget,
  Logger,
  LoggerOptions,
} from "./log";
export { captureError, formatReport, installErrorReporter } from "./error-report";
export type {
  ErrorReport,
  ErrorReportKind,
  ErrorReporter,
  ErrorReporterOptions,
  ReportGame,
} from "./error-report";
