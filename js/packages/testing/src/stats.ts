// Summaries of measured values: the smallest, the largest, the mean and the 50th, 90th and 99th
// percentiles. Soak runs use them for the chosen values and for the time per frame.

/** The summary of a list of numbers. Every field is null when the list is empty. */
export interface Summary {
  /** How many values there were, not counting NaN. */
  readonly count: number;
  readonly min: number | null;
  readonly max: number | null;
  readonly mean: number | null;
  readonly p50: number | null;
  readonly p90: number | null;
  readonly p99: number | null;
}

/**
 * The value at percentile `p` of a sorted list, by the nearest-rank rule: the smallest value that at
 * least `p` percent of the list does not exceed. `p` runs from 0 to 100.
 */
export function percentile(sorted: readonly number[], p: number): number | null {
  if (sorted.length === 0) return null;
  const rank = Math.ceil((p / 100) * sorted.length);
  const index = Math.min(sorted.length - 1, Math.max(0, rank - 1));
  return sorted[index] ?? null;
}

/** Summarises a list of numbers. NaN values are left out, since they carry no measurement. */
export function summarize(values: readonly number[]): Summary {
  const sorted = values.filter((value) => !Number.isNaN(value)).sort((left, right) => left - right);
  if (sorted.length === 0) {
    return { count: 0, min: null, max: null, mean: null, p50: null, p90: null, p99: null };
  }
  let total = 0;
  for (const value of sorted) total += value;
  return {
    count: sorted.length,
    min: sorted[0] ?? null,
    max: sorted[sorted.length - 1] ?? null,
    mean: total / sorted.length,
    p50: percentile(sorted, 50),
    p90: percentile(sorted, 90),
    p99: percentile(sorted, 99),
  };
}
