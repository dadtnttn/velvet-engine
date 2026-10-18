// The shape of a command, and the parsing of its command line. Every command takes the same two
// built-in options: `--json` (print the result as JSON) and `--help` (show the options).

import type { Output } from "./output";

/** An option of a command: a flag with no value, or an option that takes one. */
export interface OptionSpec {
  /** The name without the dashes, as typed after `--`. */
  readonly name: string;
  /** True for an option with a value: `--id hello` or `--id=hello`. */
  readonly takesValue: boolean;
  /** One line for the help text. */
  readonly help: string;
  /** How the value is named in the help text, such as `<id>`. */
  readonly valueName?: string;
}

/** The arguments of a command line, after they were checked against the options. */
export interface ParsedArgs {
  /** The arguments that are not options, in order. */
  readonly positional: readonly string[];
  /** The names of the flags that were given. */
  readonly flags: ReadonlySet<string>;
  /** The values of the options that were given. A repeated option keeps its last value. */
  readonly values: ReadonlyMap<string, string>;
}

/** A command line that does not fit the command's options. Its message says what is wrong. */
export class UsageError extends Error {
  /** @param message - what is wrong with the command line, for the user. */
  constructor(message: string) {
    super(message);
    this.name = "UsageError";
  }
}

/** A command: its name, what it does, its options, and the function that runs it. */
export interface CommandSpec {
  /** The name typed after `velvet-js`. */
  readonly name: string;
  /** One line that says what the command does. */
  readonly summary: string;
  /** The arguments after the name, as the help text shows them: `<dir> --id <id>`. */
  readonly usage: string;
  /** The command's own options. `--json` and `--help` are added to them. */
  readonly options: readonly OptionSpec[];
  /** Extra lines for the help text, such as examples and rules. */
  readonly details: readonly string[];
  /**
   * Runs the command.
   * @param args - the parsed command line.
   * @param output - where the command writes its text.
   * @returns the exit code: 0 for success, 1 for a failure.
   */
  readonly run: (args: ParsedArgs, output: Output) => Promise<number>;
}

/** The options that every command has. */
export const BUILT_IN_OPTIONS: readonly OptionSpec[] = [
  { name: "json", takesValue: false, help: "print the result as JSON, for scripts" },
  { name: "help", takesValue: false, help: "show this help (also -h)" },
];

/**
 * Reads a command line against its options.
 * @param args - the arguments after the command name. `-h` means `--help`.
 * @param options - the command's options, without the built-in ones.
 * @throws UsageError for an unknown option, a value that is missing, or a value given to a flag.
 */
export function parseArgs(args: readonly string[], options: readonly OptionSpec[]): ParsedArgs {
  const known = new Map<string, OptionSpec>();
  for (const option of [...options, ...BUILT_IN_OPTIONS]) known.set(option.name, option);

  const positional: string[] = [];
  const flags = new Set<string>();
  const values = new Map<string, string>();
  let onlyPositional = false;

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index] ?? "";
    if (onlyPositional || arg === "-" || !arg.startsWith("-")) {
      positional.push(arg);
      continue;
    }
    if (arg === "--") {
      onlyPositional = true;
      continue;
    }
    if (arg === "-h") {
      flags.add("help");
      continue;
    }
    if (!arg.startsWith("--")) {
      throw new UsageError(`unknown option "${arg}"`);
    }

    const equals = arg.indexOf("=");
    const name = equals < 0 ? arg.slice(2) : arg.slice(2, equals);
    const spec = known.get(name);
    if (spec === undefined) {
      throw new UsageError(`unknown option "--${name}"`);
    }
    if (!spec.takesValue) {
      if (equals >= 0) throw new UsageError(`--${name} is a flag and takes no value`);
      flags.add(name);
      continue;
    }
    if (equals >= 0) {
      values.set(name, arg.slice(equals + 1));
      continue;
    }
    const next = args[index + 1];
    if (next === undefined || next.startsWith("--")) {
      throw new UsageError(`--${name} needs a value`);
    }
    values.set(name, next);
    index += 1;
  }
  return { positional, flags, values };
}

/**
 * The help text of a command: its usage, its summary, its options and its details.
 * @param spec - the command.
 * @returns the lines to print.
 */
export function helpText(spec: CommandSpec): string[] {
  const options = [...spec.options, ...BUILT_IN_OPTIONS];
  const labels = options.map((option) => {
    const value = option.takesValue ? ` ${option.valueName ?? "<value>"}` : "";
    return `  --${option.name}${value}`;
  });
  const width = Math.max(...labels.map((label) => label.length)) + 2;
  const lines = [
    `Usage: velvet-js ${spec.name} ${spec.usage}`.trimEnd(),
    "",
    spec.summary,
    "",
    "Options:",
    ...options.map((option, index) => `${(labels[index] ?? "").padEnd(width)}${option.help}`),
  ];
  if (spec.details.length > 0) lines.push("", ...spec.details);
  return lines;
}
