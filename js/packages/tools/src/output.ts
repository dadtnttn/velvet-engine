// Where a command writes its text: the terminal in the tools, a buffer in the tests.

/** The two streams that a command writes to. Each call writes one line. */
export interface Output {
  /** Writes a line to standard output. */
  readonly out: (line: string) => void;
  /** Writes a line to standard error. */
  readonly err: (line: string) => void;
}

/** The terminal: standard output and standard error of this process. */
export const terminalOutput: Output = {
  out: (line) => {
    process.stdout.write(`${line}\n`);
  },
  err: (line) => {
    process.stderr.write(`${line}\n`);
  },
};

/** Lines collected in memory, for tests and for callers that want the text. */
export interface BufferedOutput {
  readonly output: Output;
  /** Every line written to standard output, in order. */
  readonly stdout: string[];
  /** Every line written to standard error, in order. */
  readonly stderr: string[];
}

/** An output that keeps its lines in two arrays. */
export function bufferedOutput(): BufferedOutput {
  const stdout: string[] = [];
  const stderr: string[] = [];
  return {
    output: {
      out: (line) => {
        stdout.push(line);
      },
      err: (line) => {
        stderr.push(line);
      },
    },
    stdout,
    stderr,
  };
}
