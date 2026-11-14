import { describe, expect, it } from "vitest";

import { EXPORT_COLUMNS, csvField, previewColumns, toCsv, toJsonl } from "./export.ts";
import type { ReviewRow } from "./queue.ts";
import type { Prediction } from "./schema.ts";

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
    fx_name: "Door Creak",
    keywords: ["Door", "Creak"],
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

describe("csvField", () => {
  it("renders null as an empty field", () => {
    expect(csvField(null)).toBe("");
  });

  it("leaves a plain string alone", () => {
    expect(csvField("door creak")).toBe("door creak");
  });

  it("quotes a field containing a comma", () => {
    expect(csvField("a,b")).toBe('"a,b"');
  });

  it("doubles an embedded quote", () => {
    expect(csvField('say "hi"')).toBe('"say ""hi"""');
  });

  it("quotes a field containing a newline", () => {
    expect(csvField("a\nb")).toBe('"a\nb"');
  });

  it("quotes a field containing a carriage return", () => {
    expect(csvField("a\rb")).toBe('"a\rb"');
  });

  it("renders numbers without quotes", () => {
    expect(csvField(0.5)).toBe("0.5");
  });

  it("renders booleans without quotes", () => {
    expect(csvField(true)).toBe("true");
  });
});

describe("toCsv", () => {
  it("writes the header first", () => {
    const csv = toCsv([]);
    expect(csv.split("\n")[0]).toBe(EXPORT_COLUMNS.map((column) => column.header).join(","));
  });

  it("ends with a newline", () => {
    expect(toCsv([row()]).endsWith("\n")).toBe(true);
  });

  it("writes one line per row plus the header", () => {
    const csv = toCsv([row({ id: "1" }), row({ id: "2", fileName: "b.wav" })]);
    expect(csv.trimEnd().split("\n")).toHaveLength(3);
  });

  it("carries the effective CatID after an override", () => {
    const csv = toCsv([row({ override: "DOORSlid" })]);
    expect(csv).toContain("DOORSlid");
    expect(csv).toContain("DOORCreak");
  });

  it("marks an overridden row", () => {
    const csv = toCsv([row({ override: "DOORSlid" })]);
    expect(csv).toContain(",true,");
  });

  it("joins keywords with a semicolon", () => {
    expect(toCsv([row()])).toContain("Door; Creak");
  });

  it("quotes a transcript containing a comma", () => {
    expect(toCsv([row({ transcript: "door creak, loud" })])).toContain('"door creak, loud"');
  });

  it("renders a missing prediction as empty fields", () => {
    const csv = toCsv([row({ prediction: null, transcript: null })]);
    const line = csv.trimEnd().split("\n")[1] ?? "";
    expect(line.startsWith("a.wav,,,false,,,,")).toBe(true);
  });

  it("honours a custom column set", () => {
    const csv = toCsv([row()], [{ key: "file", header: "file", value: (item) => item.fileName }]);
    expect(csv).toBe("file\na.wav\n");
  });

  it("is deterministic for the same input", () => {
    const rows = [row()];
    expect(toCsv(rows)).toBe(toCsv(rows));
  });
});

describe("toJsonl", () => {
  it("returns an empty string for no rows", () => {
    expect(toJsonl([])).toBe("");
  });

  it("writes one object per line", () => {
    const jsonl = toJsonl([row({ id: "1" }), row({ id: "2", fileName: "b.wav" })]);
    expect(jsonl.trimEnd().split("\n")).toHaveLength(2);
  });

  it("parses back to the column keys", () => {
    const parsed = JSON.parse(toJsonl([row()]).trimEnd()) as Record<string, unknown>;
    expect(Object.keys(parsed)).toEqual(EXPORT_COLUMNS.map((column) => column.key));
  });

  it("carries the effective CatID and the original guess", () => {
    const parsed = JSON.parse(toJsonl([row({ override: "DOORSlid" })]).trimEnd()) as Record<string, unknown>;
    expect(parsed.cat_id).toBe("DOORSlid");
    expect(parsed.original_cat_id).toBe("DOORCreak");
    expect(parsed.overridden).toBe(true);
  });

  it("writes null rather than dropping a missing field", () => {
    const parsed = JSON.parse(toJsonl([row({ prediction: null })]).trimEnd()) as Record<string, unknown>;
    expect(parsed.cat_id).toBeNull();
    expect(parsed.category).toBeNull();
  });

  it("rounds the confidence to six places", () => {
    const parsed = JSON.parse(
      toJsonl([row({ prediction: prediction({ confidence: 0.123456789 }) })]).trimEnd(),
    ) as Record<string, unknown>;
    expect(parsed.confidence).toBe(0.123457);
  });

  it("ends with a newline", () => {
    expect(toJsonl([row()]).endsWith("\n")).toBe(true);
  });
});

describe("previewColumns", () => {
  it("shows the headers", () => {
    expect(previewColumns([]).headers).toEqual(EXPORT_COLUMNS.map((column) => column.header));
  });

  it("limits the rows shown", () => {
    const rows = Array.from({ length: 8 }, (_, index) => row({ id: String(index) }));
    expect(previewColumns(rows, 3).rows).toHaveLength(3);
  });

  it("reports how many rows were omitted", () => {
    const rows = Array.from({ length: 8 }, (_, index) => row({ id: String(index) }));
    expect(previewColumns(rows, 3).omitted).toBe(5);
  });

  it("omits nothing when the limit exceeds the queue", () => {
    expect(previewColumns([row()], 5).omitted).toBe(0);
  });

  it("renders a null cell as an empty string", () => {
    const preview = previewColumns([row({ prediction: null })], 1);
    expect(preview.rows[0]?.[1]).toBe("");
  });

  it("rejects a negative limit", () => {
    expect(() => previewColumns([], -1)).toThrow(RangeError);
  });

  it("rejects a fractional limit", () => {
    expect(() => previewColumns([], 1.5)).toThrow(RangeError);
  });

  it("accepts a zero limit", () => {
    expect(previewColumns([row()], 0).rows).toEqual([]);
  });
});
