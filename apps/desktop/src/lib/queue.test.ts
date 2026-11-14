import { describe, expect, it } from "vitest";

import {
  PREDICT_BATCH_SIZE,
  QUEUE_FILTERS,
  applyOverride,
  chunk,
  clearOverride,
  effectiveCatId,
  effectiveConfidence,
  effectiveDecision,
  filterCounts,
  filterLabel,
  filterRows,
  isOverridden,
  matchesFilter,
  pendingTexts,
  sortByConfidenceAscending,
  summarise,
  type ReviewRow,
} from "./queue.ts";
import type { Decision, Prediction, TaxonomyEntry } from "./schema.ts";

function prediction(overrides: Partial<Prediction> = {}): Prediction {
  return {
    cat_id: "DOORCreak",
    category: "DOORS",
    subcategory: "Creak",
    cat_short: "DOORCreak",
    category_full: "Doors",
    confidence: 0.5,
    decision: "confident",
    filename: "DOORCreak_Door Creak_NONE_NONE",
    ...overrides,
  };
}

function row(overrides: Partial<ReviewRow> = {}): ReviewRow {
  return {
    id: "/tmp/a.wav",
    fileName: "a.wav",
    transcript: "door creak",
    prediction: prediction(),
    override: null,
    status: "ready",
    error: null,
    ...overrides,
  };
}

function entry(catId: string): TaxonomyEntry {
  return {
    cat_id: catId,
    category: "DOORS",
    subcategory: "Creak",
    cat_short: catId,
    category_full: "Doors",
  };
}

describe("effectiveDecision", () => {
  it("returns the classifier decision when there is no override", () => {
    expect(effectiveDecision(row())).toBe("confident");
  });

  it("returns null before a prediction exists", () => {
    expect(effectiveDecision(row({ prediction: null }))).toBeNull();
  });

  it("treats an override as confident", () => {
    expect(effectiveDecision(row({ override: "DOORSlid" }))).toBe("confident");
  });

  it("lets the override win over an unknown decision", () => {
    const unknown = row({ prediction: prediction({ decision: "unknown" }), override: "DOORSlid" });
    expect(effectiveDecision(unknown)).toBe("confident");
  });
});

describe("effectiveCatId", () => {
  it("prefers the override", () => {
    expect(effectiveCatId(row({ override: "DOORSlid" }))).toBe("DOORSlid");
  });

  it("falls back to the prediction", () => {
    expect(effectiveCatId(row())).toBe("DOORCreak");
  });

  it("is null with neither", () => {
    expect(effectiveCatId(row({ prediction: null }))).toBeNull();
  });
});

describe("effectiveConfidence", () => {
  it("returns the prediction confidence", () => {
    expect(effectiveConfidence(row({ prediction: prediction({ confidence: 0.42 }) }))).toBeCloseTo(0.42);
  });

  it("returns 1 for an override", () => {
    expect(effectiveConfidence(row({ override: "DOORSlid" }))).toBe(1);
  });

  it("returns 0 without a prediction", () => {
    expect(effectiveConfidence(row({ prediction: null }))).toBe(0);
  });
});

describe("isOverridden", () => {
  it("is false without an override", () => {
    expect(isOverridden(row())).toBe(false);
  });

  it("is true when the override differs from the guess", () => {
    expect(isOverridden(row({ override: "DOORSlid" }))).toBe(true);
  });

  it("is false when the override equals the guess", () => {
    expect(isOverridden(row({ override: "DOORCreak" }))).toBe(false);
  });
});

describe("sortByConfidenceAscending", () => {
  it("puts the least certain row first", () => {
    const rows = [
      row({ id: "c", fileName: "c.wav", prediction: prediction({ confidence: 0.9 }) }),
      row({ id: "a", fileName: "a.wav", prediction: prediction({ confidence: 0.2 }) }),
      row({ id: "b", fileName: "b.wav", prediction: prediction({ confidence: 0.5 }) }),
    ];

    expect(sortByConfidenceAscending(rows).map((item) => item.id)).toEqual(["a", "b", "c"]);
  });

  it("puts rows without a prediction before every predicted row", () => {
    const rows = [
      row({ id: "predicted", prediction: prediction({ confidence: 0.01 }) }),
      row({ id: "waiting", prediction: null, status: "transcribing" }),
    ];

    expect(sortByConfidenceAscending(rows).map((item) => item.id)).toEqual(["waiting", "predicted"]);
  });

  it("breaks ties on file name then id, so the order is total", () => {
    const rows = [
      row({ id: "z", fileName: "same.wav" }),
      row({ id: "a", fileName: "same.wav" }),
      row({ id: "m", fileName: "alpha.wav" }),
    ];

    expect(sortByConfidenceAscending(rows).map((item) => item.id)).toEqual(["m", "a", "z"]);
  });

  it("does not mutate its input", () => {
    const rows = [
      row({ id: "b", prediction: prediction({ confidence: 0.9 }) }),
      row({ id: "a", prediction: prediction({ confidence: 0.1 }) }),
    ];
    const before = rows.map((item) => item.id);

    sortByConfidenceAscending(rows);

    expect(rows.map((item) => item.id)).toEqual(before);
  });

  it("sorts an override as certain", () => {
    const rows = [
      row({ id: "overridden", override: "DOORSlid", prediction: prediction({ confidence: 0.1 }) }),
      row({ id: "guessed", prediction: prediction({ confidence: 0.5 }) }),
    ];

    expect(sortByConfidenceAscending(rows).map((item) => item.id)).toEqual(["guessed", "overridden"]);
  });

  it("returns an empty array for an empty queue", () => {
    expect(sortByConfidenceAscending([])).toEqual([]);
  });
});

describe("matchesFilter", () => {
  const cases: Array<[Decision, string]> = [
    ["confident", "confident"],
    ["category_fallback", "fallback"],
    ["unknown", "unknown"],
  ];

  it.each(cases)("matches %s rows under the %s chip", (decision, filter) => {
    expect(matchesFilter(row({ prediction: prediction({ decision }) }), filter as never)).toBe(true);
  });

  it("matches every row under all", () => {
    expect(matchesFilter(row(), "all")).toBe(true);
  });

  it("matches only overridden rows under overridden", () => {
    expect(matchesFilter(row(), "overridden")).toBe(false);
    expect(matchesFilter(row({ override: "DOORSlid" }), "overridden")).toBe(true);
  });

  it("matches only failed rows under failed", () => {
    expect(matchesFilter(row(), "failed")).toBe(false);
    expect(matchesFilter(row({ status: "failed" }), "failed")).toBe(true);
  });

  it("does not match a confident row under fallback or unknown", () => {
    expect(matchesFilter(row(), "fallback")).toBe(false);
    expect(matchesFilter(row(), "unknown")).toBe(false);
  });
});

describe("filterRows", () => {
  it("preserves the incoming order", () => {
    const rows = [
      row({ id: "1", prediction: prediction({ decision: "unknown" }) }),
      row({ id: "2" }),
      row({ id: "3", prediction: prediction({ decision: "unknown" }) }),
    ];

    expect(filterRows(rows, "unknown").map((item) => item.id)).toEqual(["1", "3"]);
  });

  it("returns everything under all", () => {
    const rows = [row({ id: "1" }), row({ id: "2", status: "failed" })];
    expect(filterRows(rows, "all")).toHaveLength(2);
  });
});

describe("filterCounts", () => {
  it("counts every chip", () => {
    const rows = [
      row({ id: "1" }),
      row({ id: "2", prediction: prediction({ decision: "unknown" }) }),
      row({ id: "3", override: "DOORSlid" }),
      row({ id: "4", status: "failed", prediction: null, transcript: null }),
    ];

    expect(filterCounts(rows)).toEqual({
      all: 4,
      confident: 2,
      fallback: 0,
      unknown: 1,
      overridden: 1,
      failed: 1,
    });
  });

  it("counts an overridden row as confident, not as its original decision", () => {
    const rows = [row({ prediction: prediction({ decision: "unknown" }), override: "DOORSlid" })];

    const counts = filterCounts(rows);

    expect(counts.unknown).toBe(0);
    expect(counts.overridden).toBe(1);
    expect(counts.confident).toBe(1);
  });

  it("returns zeroes for an empty queue", () => {
    expect(filterCounts([])).toEqual({
      all: 0,
      confident: 0,
      fallback: 0,
      unknown: 0,
      overridden: 0,
      failed: 0,
    });
  });
});

describe("filterLabel", () => {
  it("labels every filter", () => {
    const labels = QUEUE_FILTERS.map(filterLabel);
    expect(labels).toEqual(["All", "Confident", "Category only", "Unknown", "Overridden", "Failed"]);
  });

  it("has no duplicate labels", () => {
    expect(new Set(QUEUE_FILTERS.map(filterLabel)).size).toBe(QUEUE_FILTERS.length);
  });
});

describe("applyOverride", () => {
  it("records the chosen CatID", () => {
    expect(applyOverride(row(), entry("DOORSlid")).override).toBe("DOORSlid");
  });

  it("keeps the original guess on the prediction", () => {
    expect(applyOverride(row(), entry("DOORSlid")).prediction?.cat_id).toBe("DOORCreak");
  });

  it("clears the override when the choice equals the guess", () => {
    expect(applyOverride(row({ override: "DOORSlid" }), entry("DOORCreak")).override).toBeNull();
  });

  it("does not mutate the row", () => {
    const original = row();
    applyOverride(original, entry("DOORSlid"));
    expect(original.override).toBeNull();
  });
});

describe("clearOverride", () => {
  it("removes an override", () => {
    expect(clearOverride(row({ override: "DOORSlid" })).override).toBeNull();
  });

  it("returns the same object when there is nothing to clear", () => {
    const original = row();
    expect(clearOverride(original)).toBe(original);
  });
});

describe("chunk", () => {
  it("splits into fixed-size batches", () => {
    expect(chunk([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
  });

  it("returns one batch when the size exceeds the length", () => {
    expect(chunk([1, 2], 10)).toEqual([[1, 2]]);
  });

  it("returns no batches for an empty list", () => {
    expect(chunk([], 32)).toEqual([]);
  });

  it("rejects a zero size", () => {
    expect(() => chunk([1], 0)).toThrow(RangeError);
  });

  it("rejects a negative size", () => {
    expect(() => chunk([1], -1)).toThrow(RangeError);
  });

  it("rejects a fractional size", () => {
    expect(() => chunk([1], 1.5)).toThrow(RangeError);
  });

  it("keeps every item exactly once", () => {
    const items = Array.from({ length: 100 }, (_, index) => index);
    expect(chunk(items, PREDICT_BATCH_SIZE).flat()).toEqual(items);
  });

  it("uses 32 as the predict batch size", () => {
    expect(PREDICT_BATCH_SIZE).toBe(32);
  });
});

describe("pendingTexts", () => {
  it("returns transcripts for ready rows without a prediction", () => {
    const rows = [
      row({ id: "1", prediction: null, transcript: "one" }),
      row({ id: "2", transcript: "two" }),
    ];

    expect(pendingTexts(rows)).toEqual(["one"]);
  });

  it("skips rows that are not ready", () => {
    const rows = [row({ prediction: null, transcript: "one", status: "transcribing" })];
    expect(pendingTexts(rows)).toEqual([]);
  });

  it("skips rows without a transcript", () => {
    const rows = [row({ prediction: null, transcript: null })];
    expect(pendingTexts(rows)).toEqual([]);
  });

  it("preserves queue order", () => {
    const rows = [
      row({ id: "1", prediction: null, transcript: "first" }),
      row({ id: "2", prediction: null, transcript: "second" }),
    ];

    expect(pendingTexts(rows)).toEqual(["first", "second"]);
  });
});

describe("summarise", () => {
  it("counts a mixed queue", () => {
    const rows = [
      row({ id: "1" }),
      row({ id: "2", override: "DOORSlid" }),
      row({ id: "3", status: "failed", prediction: null, transcript: null, error: "boom" }),
      row({ id: "4", status: "transcribing", prediction: null }),
    ];

    expect(summarise(rows)).toEqual({ total: 4, reviewed: 2, overridden: 1, failed: 1, pending: 2 });
  });

  it("counts an empty queue as zeroes", () => {
    expect(summarise([])).toEqual({ total: 0, reviewed: 0, overridden: 0, failed: 0, pending: 0 });
  });

  it("does not count a no-op override as overridden", () => {
    const rows = [row({ override: "DOORCreak" })];
    expect(summarise(rows).overridden).toBe(0);
  });
});
