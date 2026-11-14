/**
 * The export panel: a column preview and the two export buttons.
 *
 * The preview is built from the same {@link previewColumns} the export uses, so what the
 * user sees above the buttons is the head of the file they are about to write.
 */
import { useMemo } from "react";

import { previewColumns, toCsv, toJsonl } from "../lib/export.ts";
import type { ReviewRow } from "../lib/queue.ts";

/** The formats the panel can write. */
export type ExportFormat = "csv" | "jsonl";

/** Props for {@link ExportPanel}. */
export interface ExportPanelProps {
  /** The rows to export, in queue order. */
  readonly rows: readonly ReviewRow[];
  /** Called with the format and the rendered text. */
  readonly onExport: (format: ExportFormat, text: string) => void;
  /** How many rows the preview shows. */
  readonly previewLimit?: number;
}

/** Render a queue in a format. */
export function renderExport(rows: readonly ReviewRow[], format: ExportFormat): string {
  return format === "csv" ? toCsv(rows) : toJsonl(rows);
}

/** The export pane. */
export function ExportPanel({ rows, onExport, previewLimit = 5 }: ExportPanelProps): React.ReactElement {
  const preview = useMemo(() => previewColumns(rows, previewLimit), [rows, previewLimit]);

  return (
    <section className="export" aria-label="Export">
      <h2 className="export__title">Export {rows.length} rows</h2>
      <div className="export__preview">
        <table>
          <thead>
            <tr>
              {preview.headers.map((header) => (
                <th key={header} scope="col">
                  {header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {preview.rows.map((cells, index) => (
              <tr key={index}>
                {cells.map((cell, cellIndex) => (
                  <td key={preview.headers[cellIndex] ?? cellIndex}>{cell}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {preview.omitted > 0 ? (
          <p className="field__hint">and {preview.omitted} more rows</p>
        ) : null}
      </div>
      <div className="export__actions">
        <button type="button" className="button" onClick={() => onExport("csv", renderExport(rows, "csv"))}>
          Export CSV
        </button>
        <button type="button" className="button" onClick={() => onExport("jsonl", renderExport(rows, "jsonl"))}>
          Export JSONL
        </button>
      </div>
    </section>
  );
}
