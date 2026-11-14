/**
 * CSV and JSONL export of the review queue.
 *
 * The column set is fixed and ordered, so two exports of the same queue are byte-identical
 * and a diff between them means the data changed. CSV follows RFC 4180: a field is quoted
 * only when it contains a comma, a quote, or a line break, and an embedded quote is doubled.
 */
import { effectiveCatId, effectiveConfidence, effectiveDecision, isOverridden, type ReviewRow } from "./queue.ts";

/** One exported column. */
export interface ExportColumn {
  /** The key used in JSONL. */
  readonly key: string;
  /** The header used in CSV. */
  readonly header: string;
  /** Read the value for a row; `null` becomes an empty CSV field and a JSON `null`. */
  readonly value: (row: ReviewRow) => string | number | boolean | null;
}

/** The columns every export carries, in order. */
export const EXPORT_COLUMNS: readonly ExportColumn[] = [
  { key: "file", header: "file", value: (row) => row.fileName },
  { key: "cat_id", header: "cat_id", value: (row) => effectiveCatId(row) },
  { key: "original_cat_id", header: "original_cat_id", value: (row) => row.prediction?.cat_id ?? null },
  { key: "overridden", header: "overridden", value: (row) => isOverridden(row) },
  { key: "category", header: "category", value: (row) => row.prediction?.category ?? null },
  { key: "subcategory", header: "subcategory", value: (row) => row.prediction?.subcategory ?? null },
  { key: "cat_short", header: "cat_short", value: (row) => row.prediction?.cat_short ?? null },
  { key: "category_full", header: "category_full", value: (row) => row.prediction?.category_full ?? null },
  { key: "confidence", header: "confidence", value: (row) => round(effectiveConfidence(row)) },
  { key: "decision", header: "decision", value: (row) => effectiveDecision(row) },
  { key: "filename", header: "filename", value: (row) => row.prediction?.filename ?? null },
  { key: "fx_name", header: "fx_name", value: (row) => row.prediction?.fx_name ?? null },
  { key: "keywords", header: "keywords", value: (row) => (row.prediction?.keywords ?? []).join("; ") },
  { key: "transcript", header: "transcript", value: (row) => row.transcript },
];

/** Round a confidence to six places, matching the engine's own precision. */
function round(value: number): number {
  return Math.round(value * 1e6) / 1e6;
}

/** Quote one CSV field per RFC 4180. */
export function csvField(value: string | number | boolean | null): string {
  if (value === null) {
    return "";
  }
  const text = String(value);
  if (/[",\r\n]/.test(text)) {
    return `"${text.replaceAll('"', '""')}"`;
  }
  return text;
}

/** Render the queue as CSV, header first, `\n` line endings. */
export function toCsv(rows: readonly ReviewRow[], columns: readonly ExportColumn[] = EXPORT_COLUMNS): string {
  const lines = [columns.map((column) => csvField(column.header)).join(",")];
  for (const row of rows) {
    lines.push(columns.map((column) => csvField(column.value(row))).join(","));
  }
  return `${lines.join("\n")}\n`;
}

/** Render the queue as JSONL, one object per line, keys in column order. */
export function toJsonl(rows: readonly ReviewRow[], columns: readonly ExportColumn[] = EXPORT_COLUMNS): string {
  const lines = rows.map((row) => {
    const record: Record<string, string | number | boolean | null> = {};
    for (const column of columns) {
      record[column.key] = column.value(row);
    }
    return JSON.stringify(record);
  });
  return lines.length === 0 ? "" : `${lines.join("\n")}\n`;
}

/** A column preview: the headers and the first `limit` rows, already rendered as text. */
export interface ColumnPreview {
  /** The CSV headers. */
  readonly headers: string[];
  /** Up to `limit` rows of rendered cells. */
  readonly rows: string[][];
  /** How many rows were left out. */
  readonly omitted: number;
}

/** Build the preview shown above the export buttons. */
export function previewColumns(
  rows: readonly ReviewRow[],
  limit = 5,
  columns: readonly ExportColumn[] = EXPORT_COLUMNS,
): ColumnPreview {
  if (!Number.isInteger(limit) || limit < 0) {
    throw new RangeError(`preview limit must be a non-negative integer, got ${limit}`);
  }
  const shown = rows.slice(0, limit);
  return {
    headers: columns.map((column) => column.header),
    rows: shown.map((row) => columns.map((column) => String(column.value(row) ?? ""))),
    omitted: Math.max(0, rows.length - shown.length),
  };
}
