// @velvet/testing: tools to test and debug any Velvet JS game. A game's loop runs headless on a driver
// with a manual clock; scripted input reaches its action maps the way a player's input does; sessions are
// recorded as replays and checked against state hashes; DOM and state snapshots compare stable text; and
// soak runs play many seeds with a bot strategy and report the metrics.
//
// Import from the package root, for example:
//   import { createDriver, InputScript, runSoak } from "@velvet/testing";

export { TestingError } from "./errors";
export type { TestingErrorCode } from "./errors";
export { hashState, hashText, stableStringify } from "./hash";
export type { StableJsonOptions } from "./hash";
export { createRandom } from "./random";
export type { Random } from "./random";
export { percentile, summarize } from "./stats";
export type { Summary } from "./stats";
export {
  DEFAULT_STAGE_HEIGHT,
  DEFAULT_STAGE_WIDTH,
  clientToLogical,
  createStageMap,
  logicalToClient,
  placementOf,
} from "./stage";
export type { StageMap, StageOptions, StagePlacement, StagePoint, StageRect } from "./stage";
export { FakeGamepads, STANDARD_AXIS_COUNT, STANDARD_BUTTON_COUNT, readNavigatorPads } from "./fake-pads";
export type { NavigatorPad, PadButtonState, PadInit, PadState } from "./fake-pads";
export { Driver, createDriver } from "./driver";
export type {
  AdvanceGameOptions,
  DriverListener,
  DriverOptions,
  InputOptions,
  ScriptRun,
  UntilOptions,
  WheelOptions,
} from "./driver";
export { InputScript, ScriptCursor } from "./script";
export type {
  ClickOptions,
  DragOptions,
  HoldOptions,
  PadAxisOptions,
  PadButtonOptions,
  PadConnectOptions,
  PadOptions,
  PadStickOptions,
  ScriptAction,
  WheelStepOptions,
} from "./script";
export { createRecorder } from "./recorder";
export type { Recorder, RecorderOptions } from "./recorder";
export {
  DEFAULT_CHECK_EVERY,
  REPLAY_FORMAT,
  REPLAY_VERSION,
  parseReplay,
  parseReplayText,
  serializeReplay,
} from "./replay-format";
export type { Replay, ReplayCheckpoint, ReplayEvent, ReplayInput, ReplaySeed } from "./replay-format";
export { checkReplay, saveEnvelopeOf, startReplay } from "./replay";
export type {
  ReplayGame,
  ReplayResult,
  ReplayRun,
  ReplayRunOptions,
  ReplayStart,
  ReplayStartContext,
} from "./replay";
export { VOLATILE_ATTRIBUTES, compareSnapshot, serializeDom } from "./snapshot";
export type { DomSnapshotOptions, SnapshotComparison } from "./snapshot";
export { runSoak } from "./soak";
export type {
  BotContext,
  BotStrategy,
  MetricSampler,
  SoakContext,
  SoakError,
  SoakGame,
  SoakOptions,
  SoakRange,
  SoakReport,
} from "./soak";
