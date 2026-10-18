// @velvet/guion: the guion API that games use. A guion is a dialogue scene written as Markdown
// (the format is in docs/formats/GUION.md). The parser and the conversation rules are velvet-guion's,
// in Rust; this package calls them through WebAssembly (@velvet/wasm), so that the Rust and the
// JavaScript runtimes give the same results.
//
// Import from the package root:
//   import { GuionLibrary, parseGuion } from "@velvet/guion";

export { parseGuion } from "./parse";
export type { ParseOptions } from "./parse";
export { GuionLibrary } from "./library";
export type { EntryQuery } from "./library";
export { Conversation } from "./conversation";
export { Portraits } from "./portraits";
export type { Portrait } from "./portraits";
export { GuionError } from "./error";
export type { GuionErrorCode } from "./error";
export type {
  CastSpec,
  ChoiceOption,
  GuionCondition,
  GuionFile,
  GuionProblem,
  GuionVars,
  LineKind,
  SceneAttrs,
  SceneInfo,
  Shown,
  ShownBase,
  ShownChoice,
  ShownEnd,
  ShownLine,
  ShownVn,
  StageDirective,
} from "./types";
