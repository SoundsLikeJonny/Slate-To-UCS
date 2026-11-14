import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ExportPanel, renderExport } from "./ExportPanel.tsx";
import type { ReviewRow } from "../lib/queue.ts";
import type { Prediction } from "../lib/schema.ts";

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

describe("renderExport", () => {
  it("renders CSV", () => {
    expect(renderExport([row()], "csv").split("\n")[0]).toContain("file,cat_id");
  });

  it("renders JSONL", () => {
    expect(JSON.parse(renderExport([row()], "jsonl").trimEnd())).toMatchObject({ file: "a.wav" });
  });

  it("renders an empty queue as a header-only CSV", () => {
    expect(renderExport([], "csv").trimEnd().split("\n")).toHaveLength(1);
  });

  it("renders an empty queue as an empty JSONL", () => {
    expect(renderExport([], "jsonl")).toBe("");
  });
});

describe("ExportPanel", () => {
  it("states how many rows will be written", () => {
    render(<ExportPanel rows={[row(), row({ id: "2", fileName: "b.wav" })]} onExport={vi.fn()} />);
    expect(screen.getByRole("heading", { name: "Export 2 rows" })).toBeInTheDocument();
  });

  it("shows the column headers", () => {
    render(<ExportPanel rows={[row()]} onExport={vi.fn()} />);
    expect(screen.getByRole("columnheader", { name: "cat_id" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "transcript" })).toBeInTheDocument();
  });

  it("shows the preview rows", () => {
    render(<ExportPanel rows={[row()]} onExport={vi.fn()} />);
    expect(screen.getByRole("cell", { name: "a.wav" })).toBeInTheDocument();
  });

  it("reports the omitted rows", () => {
    const rows = Array.from({ length: 8 }, (_, index) => row({ id: String(index), fileName: `f${index}.wav` }));
    render(<ExportPanel rows={rows} onExport={vi.fn()} previewLimit={3} />);
    expect(screen.getByText("and 5 more rows")).toBeInTheDocument();
  });

  it("omits nothing when the queue fits", () => {
    render(<ExportPanel rows={[row()]} onExport={vi.fn()} />);
    expect(screen.queryByText(/more rows/)).not.toBeInTheDocument();
  });

  it("exports CSV", () => {
    const onExport = vi.fn();
    render(<ExportPanel rows={[row()]} onExport={onExport} />);
    fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    expect(onExport).toHaveBeenCalledWith("csv", expect.stringContaining("file,cat_id"));
  });

  it("exports JSONL", () => {
    const onExport = vi.fn();
    render(<ExportPanel rows={[row()]} onExport={onExport} />);
    fireEvent.click(screen.getByRole("button", { name: "Export JSONL" }));
    expect(onExport).toHaveBeenCalledWith("jsonl", expect.stringContaining('"file":"a.wav"'));
  });

  it("exports an empty queue without throwing", () => {
    const onExport = vi.fn();
    render(<ExportPanel rows={[]} onExport={onExport} />);
    fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    expect(onExport).toHaveBeenCalledWith("csv", expect.any(String));
  });
});
