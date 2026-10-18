// @velvet/tools: the command-line tools of Velvet JS, run as `pnpm velvet-js <command>` from js/.
// Commands: `new` scaffolds a game from the template, `check` validates a game's data against every
// format and the links between its files, and `i18n` reports the coverage of its languages (and writes a
// pseudo-locale for layout tests). The package is not a runtime library, so its API is for other tools
// and for tests. See README.md for the commands and the rules that discovery follows.

export { COMMANDS, main } from "./cli";
export { checkGame } from "./check/check";
export type { CheckCounts, CheckResult } from "./check/check";
export { scaffoldGame } from "./new/scaffold";
export type { ScaffoldOptions, ScaffoldResult } from "./new/scaffold";
export { PSEUDO_GROWTH, PSEUDO_NOTE, pseudoText, pseudoValue } from "./i18n/pseudo";
export { discoverGame } from "./game/discover";
export type { DocumentKind, GameDiscovery, GameDocument } from "./game/discover";
export { countIssues, formatIssue, makeIssue } from "./issues";
export type { Issue, Severity } from "./issues";
