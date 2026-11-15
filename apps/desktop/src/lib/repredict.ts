/**
 * Re-predicting after an edit.
 *
 * The exit criterion is that editing one transcript re-predicts that row in under
 * 200 ms on a 200-row queue. The property that makes that possible is that an edit
 * issues one request for the changed rows, never a request per row and never a full
 * re-run. The clock is injected so the timing assertion is deterministic.
 */
import type { Prediction } from "./schema.ts";

/** The largest number of texts sent in one `/predict` call. */
export const PREDICT_BATCH = 32;

/** The part of a queue row this module needs. */
export interface EditableRow {
  /** A stable identifier, unique within the queue. */
  readonly id: string;
  /** The transcript the classifier reads. */
  readonly transcript: string;
  /** The prediction already held, or `null` before the first predict. */
  readonly prediction: Prediction | null;
}

/** The part of the engine client this module needs. */
export interface PredictClient {
  /** `POST /predict`. */
  predict(texts: readonly string[], topK?: number): Promise<Prediction[]>;
}

/** What a re-predict produced. */
export interface RePredictResult {
  /** Predictions by row id, for the rows that were re-predicted. */
  readonly predictions: ReadonlyMap<string, Prediction>;
  /** How many `/predict` calls were made. */
  readonly requests: number;
  /** How long the whole operation took, from the injected clock. */
  readonly elapsedMs: number;
}

/** Split a list into chunks of at most `size`. */
function chunk<T>(items: readonly T[], size: number): T[][] {
  const chunks: T[][] = [];
  for (let index = 0; index < items.length; index += size) {
    chunks.push(items.slice(index, index + size));
  }
  return chunks;
}

/**
 * Re-predict the rows whose transcripts changed.
 *
 * Rows not named in `changedIds` keep their existing prediction and cost nothing. The
 * changed rows are sent in batches of {@link PREDICT_BATCH}, and the responses are
 * matched back to row ids by position within each batch.
 *
 * # Errors
 *
 * Throws when the engine returns a different number of predictions than texts sent,
 * because a silent misalignment would attach one row's prediction to another.
 */
export async function rePredictChanged(
  client: PredictClient,
  rows: readonly EditableRow[],
  changedIds: readonly string[],
  now: () => number,
): Promise<RePredictResult> {
  const started = now();
  const changed = new Set(changedIds);
  const targets = rows.filter((row) => changed.has(row.id));
  const predictions = new Map<string, Prediction>();
  let requests = 0;

  for (const batch of chunk(targets, PREDICT_BATCH)) {
    const texts = batch.map((row) => row.transcript);
    const returned = await client.predict(texts);
    requests += 1;
    if (returned.length !== texts.length) {
      throw new Error(
        `engine returned ${returned.length} predictions for ${texts.length} texts`,
      );
    }
    batch.forEach((row, index) => {
      const prediction = returned[index];
      if (prediction !== undefined) {
        predictions.set(row.id, prediction);
      }
    });
  }

  return { predictions, requests, elapsedMs: now() - started };
}

/**
 * Apply a re-predict result to the queue, leaving untouched rows alone.
 *
 * Returns a new array; the input is not mutated, so a caller can keep the previous
 * queue for an undo.
 */
export function applyPredictions(
  rows: readonly EditableRow[],
  predictions: ReadonlyMap<string, Prediction>,
): EditableRow[] {
  return rows.map((row) => {
    const prediction = predictions.get(row.id);
    return prediction === undefined ? row : { ...row, prediction };
  });
}
