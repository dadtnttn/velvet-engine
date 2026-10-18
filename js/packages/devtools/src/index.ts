// @velvet/devtools: developer tools for dev builds of a Velvet JS game. An overlay (F3 by default) with a
// performance panel, game time controls, an event log, a state inspector, a console with typed commands,
// and watches pinned to the screen. Create them only in development: see the README.
//
// Import from the package root, for example:
//   import { createDevtools } from "@velvet/devtools";

export { createDevtools } from "./devtools";
export type { FrameStats } from "./stats";
export type {
  ArgSpec,
  ArgType,
  CommandSpec,
  CommandValue,
  CommandValues,
  Devtools,
  DevtoolsOptions,
  DevtoolsStats,
  ErrorReportSource,
  HudCorner,
  InspectOptions,
} from "./types";
