// @velvet/cards: cards on a board, stacks, recipes and timed jobs, from velvet-stacks
// (docs/formats/STACKS.md). The rules are velvet-stacks' own, in Rust; this package calls them
// through WebAssembly (@velvet/wasm), so that the Rust and the JavaScript runtimes give the same
// results.
//
// Import from the package root:
//   import { Board, loadCatalog } from "@velvet/cards";

export { loadCatalog } from "./catalog";
export { Board } from "./board";
export { CardsError } from "./error";
export type { CardsErrorCode } from "./error";
export type {
  BoardEvent,
  BoardSave,
  Catalog,
  CardDef,
  CardView,
  Ingredient,
  JobSave,
  Recipe,
  RecipeOutput,
  Seed,
  Selector,
  StackJob,
  StackRule,
  StackSave,
  StackView,
} from "./types";
