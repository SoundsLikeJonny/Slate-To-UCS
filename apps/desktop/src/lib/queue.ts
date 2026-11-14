/**
 * The review queue: the rows the user works through after a batch predict.
 *
 * Everything here is a pure function over plain data, so the ordering, filtering and
 * override rules are testable without rendering a component. The components in
 * `app/Queue.tsx` and `app/Inspector.tsx` only call into this module.
 */
import type { Decision, Prediction, TaxonomyEntry } from "./schema.ts";

/** How far through the pipeline one file is. */
export type RowStatus = "pending" | "transcribing" | "predicting" | "ready" | "failed";

/** One file in the queue, with its transcript and the classifier's guess. */
export interface ReviewRow {
  /** Stable identity: the absolute path, which is unique per drop. */
  readonly id: string;
  /** The file name shown in the table. */
  readonly fileName: string;
  /** The transcript, once transcription has run. */
  readonly transcript: string | null;
  /** The classifier's answer, once prediction has run. */
  readonly prediction: Prediction | null;
  /** The CatID the user picked, or `null` while the guess stands. */
  readonly override: string | null;
  /** Where the row is in the pipeline. */
  readonly status: RowStatus;
  /** Why the row failed, when `status` is `failed`. */
  readonly error: string | null;
}

/** The filter chips above the table. */
export type QueueFilter = "all" | "confident" | "fallback" | "unknown" | "overridden" | "failed";

/** Every filter, in the order the chips are rendered. */
export const QUEUE_FILTERS: readonly QueueFilter[] = [
  "all",
  "confident",
  "fallback",
  "unknown",
  "overridden",
  "failed",
];

/** The label a chip shows. */
export function filterLabel(filter: QueueFilter): string {
  switch (filter) {
    case "all":
      return "All";
    case "confident":
      return "Confident";
    case "fallback":
      return "Category only";
    case "unknown":
      return "Unknown";
    case "overridden":
      return "Overridden";
    case "failed":
      return "Failed";
  }
}

/** The decision a row currently stands on, honouring an override. */
export function effectiveDecision(row: ReviewRow): Decision | null {
  if (row.override !== null) {
    return "confident";
  }
  return row.prediction?.decision ?? null;
}

/** The CatID a row currently stands on, honouring an override. */
export function effectiveCatId(row: ReviewRow): string | null {
  return row.override ?? row.prediction?.cat_id ?? null;
}

/** The confidence a row currently stands on; an override is certain by definition. */
export function effectiveConfidence(row: ReviewRow): number {
  if (row.override !== null) {
    return 1;
  }
  return row.prediction?.confidence ?? 0;
}

/** Whether the user has replaced the classifier's guess. */
export function isOverridden(row: ReviewRow): boolean {
  return row.override !== null && row.override !== row.prediction?.cat_id;
}

/**
 * Sort by confidence ascending, so the least certain rows are reviewed first.
 *
 * Ties break on file name and then on id, so the order is total and a re-render never
 * reshuffles equal rows. Rows without a prediction sort before every predicted row,
 * because "no answer yet" is the least certain state there is.
 */
export function sortByConfidenceAscending(rows: readonly ReviewRow[]): ReviewRow[] {
  return [...rows].sort((left, right) => {
    const leftReady = left.prediction !== null || left.override !== null;
    const rightReady = right.prediction !== null || right.override !== null;
    if (leftReady !== rightReady) {
      return leftReady ? 1 : -1;
    }
    const byConfidence = effectiveConfidence(left) - effectiveConfidence(right);
    if (byConfidence !== 0) {
      return byConfidence;
    }
    const byName = left.fileName.localeCompare(right.fileName);
    if (byName !== 0) {
      return byName;
    }
    return left.id.localeCompare(right.id);
  });
}

/** Whether a row passes a filter chip. */
export function matchesFilter(row: ReviewRow, filter: QueueFilter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "confident":
      return effectiveDecision(row) === "confident";
    case "fallback":
      return effectiveDecision(row) === "category_fallback";
    case "unknown":
      return effectiveDecision(row) === "unknown";
    case "overridden":
      return isOverridden(row);
    case "failed":
      return row.status === "failed";
  }
}

/** Apply a filter chip, preserving the incoming order. */
export function filterRows(rows: readonly ReviewRow[], filter: QueueFilter): ReviewRow[] {
  return rows.filter((row) => matchesFilter(row, filter));
}

/** How many rows each chip would show, for the count beside each label. */
export function filterCounts(rows: readonly ReviewRow[]): Record<QueueFilter, number> {
  const counts: Record<QueueFilter, number> = {
    all: rows.length,
    confident: 0,
    fallback: 0,
    unknown: 0,
    overridden: 0,
    failed: 0,
  };
  for (const row of rows) {
    for (const filter of QUEUE_FILTERS) {
      if (filter !== "all" && matchesFilter(row, filter)) {
        counts[filter] += 1;
      }
    }
  }
  return counts;
}

/**
 * Replace a row's CatID with the user's choice.
 *
 * The original guess stays on `prediction`, so the export can carry both and the UI can
 * show what the classifier said. Overriding with the guess itself clears the override
 * rather than recording a no-op.
 */
export function applyOverride(row: ReviewRow, entry: TaxonomyEntry): ReviewRow {
  const override = entry.cat_id === row.prediction?.cat_id ? null : entry.cat_id;
  return { ...row, override };
}

/** Clear an override, returning the row to the classifier's guess. */
export function clearOverride(row: ReviewRow): ReviewRow {
  return row.override === null ? row : { ...row, override: null };
}

/** Split items into fixed-size batches; the engine caps a request at 512 texts. */
export function chunk<T>(items: readonly T[], size: number): T[][] {
  if (!Number.isInteger(size) || size < 1) {
    throw new RangeError(`batch size must be a positive integer, got ${size}`);
  }
  const batches: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    batches.push(items.slice(index, index + size));
  }
  return batches;
}

/** The batch size the plan fixes for predict calls. */
export const PREDICT_BATCH_SIZE = 32;

/** Texts to predict, in queue order, skipping rows that already have an answer. */
export function pendingTexts(rows: readonly ReviewRow[]): string[] {
  return rows
    .filter((row) => row.status === "ready" && row.prediction === null && row.transcript !== null)
    .map((row) => row.transcript as string);
}

/** Counts for the status bar. */
export interface QueueSummary {
  /** Every row. */
  readonly total: number;
  /** Rows with a prediction or an override. */
  readonly reviewed: number;
  /** Rows the user overrode. */
  readonly overridden: number;
  /** Rows that failed. */
  readonly failed: number;
  /** Rows still waiting on the pipeline. */
  readonly pending: number;
}

/** Summarise the queue for the status bar. */
export function summarise(rows: readonly ReviewRow[]): QueueSummary {
  let reviewed = 0;
  let overridden = 0;
  let failed = 0;
  let pending = 0;
  for (const row of rows) {
    if (row.status === "failed") {
      failed += 1;
    }
    if (row.status !== "ready") {
      pending += 1;
    }
    if (row.prediction !== null || row.override !== null) {
      reviewed += 1;
    }
    if (isOverridden(row)) {
      overridden += 1;
    }
  }
  return { total: rows.length, reviewed, overridden, failed, pending };
}
