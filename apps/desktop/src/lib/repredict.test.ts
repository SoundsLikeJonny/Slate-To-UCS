import { describe, expect, it, vi } from "vitest";

import {
  PREDICT_BATCH,
  applyPredictions,
  rePredictChanged,
  type EditableRow,
  type PredictClient,
} from "./repredict.ts";
import type { Prediction } from "./schema.ts";

/** A prediction with the given CatID. */
function prediction(catId: string): Prediction {
  return {
    cat_id: catId,
    category: "DOOR",
    subcategory: "Creak",
    cat_short: catId,
    category_full: "Doors",
    confidence: 0.9,
    decision: "confident",
    filename: `${catId}_Door Creak_NONE_NONE`,
  };
}

/** A queue of `count` rows, each with a distinct id and transcript. */
function queue(count: number): EditableRow[] {
  return Array.from({ length: count }, (_, index) => ({
    id: `row-${index}`,
    transcript: `transcript ${index}`,
    prediction: prediction(`CAT${index}`),
  }));
}

/** A client that answers every text with a prediction derived from it. */
function stubClient(): PredictClient & { readonly calls: readonly (readonly string[])[] } {
  const calls: string[][] = [];
  return {
    calls,
    predict: (texts: readonly string[]) => {
      calls.push([...texts]);
      return Promise.resolve(texts.map((text) => prediction(`NEW${text.length}`)));
    },
  };
}

describe("rePredictChanged", () => {
  it("makes one request for one changed row in a 200-row queue", async () => {
    const client = stubClient();
    const rows = queue(200);

    const result = await rePredictChanged(client, rows, ["row-7"], () => 0);

    expect(result.requests).toBe(1);
    expect(client.calls).toHaveLength(1);
    expect(client.calls[0]).toEqual(["transcript 7"]);
  });

  it("re-predicts only the changed rows", async () => {
    const client = stubClient();
    const rows = queue(200);

    const result = await rePredictChanged(client, rows, ["row-7"], () => 0);

    expect([...result.predictions.keys()]).toEqual(["row-7"]);
  });

  it("makes no request when nothing changed", async () => {
    const client = stubClient();

    const result = await rePredictChanged(client, queue(200), [], () => 0);

    expect(result.requests).toBe(0);
    expect(client.calls).toHaveLength(0);
    expect(result.predictions.size).toBe(0);
  });

  it("ignores an id that is not in the queue", async () => {
    const client = stubClient();

    const result = await rePredictChanged(client, queue(3), ["missing"], () => 0);

    expect(result.requests).toBe(0);
    expect(result.predictions.size).toBe(0);
  });

  it("sends a batch of exactly the batch size in one request", async () => {
    const client = stubClient();
    const rows = queue(PREDICT_BATCH);
    const ids = rows.map((row) => row.id);

    const result = await rePredictChanged(client, rows, ids, () => 0);

    expect(result.requests).toBe(1);
    expect(client.calls[0]).toHaveLength(PREDICT_BATCH);
  });

  it("splits one row over the batch size into two requests", async () => {
    const client = stubClient();
    const rows = queue(PREDICT_BATCH + 1);
    const ids = rows.map((row) => row.id);

    const result = await rePredictChanged(client, rows, ids, () => 0);

    expect(result.requests).toBe(2);
    expect(client.calls[0]).toHaveLength(PREDICT_BATCH);
    expect(client.calls[1]).toHaveLength(1);
  });

  it("keeps the queue order within a batch", async () => {
    const client = stubClient();
    const rows = queue(4);

    await rePredictChanged(client, rows, ["row-3", "row-1"], () => 0);

    expect(client.calls[0]).toEqual(["transcript 1", "transcript 3"]);
  });

  it("measures elapsed time with the injected clock", async () => {
    const ticks = [100, 137];
    const now = (): number => ticks.shift() ?? 137;

    const result = await rePredictChanged(stubClient(), queue(200), ["row-1"], now);

    expect(result.elapsedMs).toBe(37);
  });

  it("re-predicts one row in under 200 ms on a 200-row queue", async () => {
    const rows = queue(200);
    const started = performance.now();

    const result = await rePredictChanged(stubClient(), rows, ["row-42"], () => 0);

    const wall = performance.now() - started;

    expect(result.requests).toBe(1);
    expect(wall).toBeLessThan(200);
  });

  it("throws when the engine returns the wrong number of predictions", async () => {
    const client: PredictClient = { predict: () => Promise.resolve([]) };

    await expect(rePredictChanged(client, queue(2), ["row-0"], () => 0)).rejects.toThrow(
      /engine returned 0 predictions for 1 texts/,
    );
  });

  it("propagates a client failure", async () => {
    const client: PredictClient = { predict: () => Promise.reject(new Error("transport")) };

    await expect(rePredictChanged(client, queue(2), ["row-0"], () => 0)).rejects.toThrow(
      /transport/,
    );
  });

  it("does not mutate the queue", async () => {
    const rows = queue(3);
    const before = rows.map((row) => row.prediction?.cat_id);

    await rePredictChanged(stubClient(), rows, ["row-1"], () => 0);

    expect(rows.map((row) => row.prediction?.cat_id)).toEqual(before);
  });
});

describe("applyPredictions", () => {
  it("replaces the prediction of a named row", () => {
    const rows = queue(2);
    const predictions = new Map([["row-1", prediction("NEW")]]);

    const next = applyPredictions(rows, predictions);

    expect(next[1]?.prediction?.cat_id).toBe("NEW");
  });

  it("leaves an unnamed row alone", () => {
    const rows = queue(2);

    const next = applyPredictions(rows, new Map([["row-1", prediction("NEW")]]));

    expect(next[0]).toBe(rows[0]);
  });

  it("returns a new array", () => {
    const rows = queue(2);

    expect(applyPredictions(rows, new Map())).not.toBe(rows);
  });

  it("keeps the length and order", () => {
    const rows = queue(3);

    const next = applyPredictions(rows, new Map([["row-0", prediction("NEW")]]));

    expect(next.map((row) => row.id)).toEqual(["row-0", "row-1", "row-2"]);
  });

  it("does not mutate the input row", () => {
    const rows = queue(1);

    applyPredictions(rows, new Map([["row-0", prediction("NEW")]]));

    expect(rows[0]?.prediction?.cat_id).toBe("CAT0");
  });

  it("is a no-op for an empty map", () => {
    const rows = queue(2);

    const next = applyPredictions(rows, new Map());

    expect(next).toEqual(rows);
  });
});

describe("PREDICT_BATCH", () => {
  it("matches the contract's batch limit", () => {
    expect(PREDICT_BATCH).toBe(32);
  });
});

describe("the client is called with plain texts", () => {
  it("never sends a row id or a filename", async () => {
    const client = stubClient();

    await rePredictChanged(client, queue(2), ["row-0"], () => 0);

    expect(client.calls[0]).toEqual(["transcript 0"]);
    expect(vi.isMockFunction(client.predict)).toBe(false);
  });
});
