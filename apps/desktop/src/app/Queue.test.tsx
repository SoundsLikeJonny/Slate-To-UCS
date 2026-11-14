import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { OVERSCAN, ROW_HEIGHT, QueuePanel, confidencePercent, decisionLabel, visibleRange } from "./Queue.tsx";
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

describe("visibleRange", () => {
  it("renders the first window at the top", () => {
    expect(visibleRange(100, 0, 340)).toEqual({ start: 0, end: 16, offset: 0 });
  });

  it("moves the window with the scroll position", () => {
    const range = visibleRange(100, 340, 340);
    expect(range.start).toBe(10 - OVERSCAN);
    expect(range.offset).toBe(range.start * ROW_HEIGHT);
  });

  it("clamps the end to the row count", () => {
    expect(visibleRange(5, 0, 340).end).toBe(5);
  });

  it("returns an empty range for no rows", () => {
    expect(visibleRange(0, 0, 340)).toEqual({ start: 0, end: 0, offset: 0 });
  });

  it("clamps a scroll position past the end", () => {
    const range = visibleRange(10, 100000, 340);
    expect(range.start).toBe(10);
    expect(range.end).toBe(10);
  });

  it("never returns a negative start", () => {
    expect(visibleRange(10, -50, 340).start).toBe(0);
  });

  it("rejects a non-positive row height", () => {
    expect(() => visibleRange(10, 0, 340, 0)).toThrow(RangeError);
  });

  it("honours a custom overscan", () => {
    expect(visibleRange(100, 340, 340, ROW_HEIGHT, 0).start).toBe(10);
  });
});

describe("decisionLabel", () => {
  it("labels a confident row", () => {
    expect(decisionLabel(row())).toBe("confident");
  });

  it("labels a category fallback", () => {
    expect(decisionLabel(row({ prediction: prediction({ decision: "category_fallback" }) }))).toBe("category");
  });

  it("labels an unknown row", () => {
    expect(decisionLabel(row({ prediction: prediction({ decision: "unknown" }) }))).toBe("unknown");
  });

  it("labels an overridden row", () => {
    expect(decisionLabel(row({ override: "DOORSlid" }))).toBe("overridden");
  });

  it("falls back to the status before a prediction exists", () => {
    expect(decisionLabel(row({ prediction: null, status: "transcribing" }))).toBe("transcribing");
  });
});

describe("confidencePercent", () => {
  it("rounds to a whole percentage", () => {
    expect(confidencePercent(row({ prediction: prediction({ confidence: 0.965 }) }))).toBe("97%");
  });

  it("shows 100% for an override", () => {
    expect(confidencePercent(row({ override: "DOORSlid" }))).toBe("100%");
  });

  it("shows 0% without a prediction", () => {
    expect(confidencePercent(row({ prediction: null }))).toBe("0%");
  });
});

describe("QueuePanel", () => {
  const rows = [
    row({ id: "1", fileName: "a.wav" }),
    row({ id: "2", fileName: "b.wav", prediction: prediction({ decision: "unknown" }) }),
  ];

  it("renders a row per file", () => {
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    expect(screen.getByText("a.wav")).toBeInTheDocument();
    expect(screen.getByText("b.wav")).toBeInTheDocument();
  });

  it("shows the count on every chip", () => {
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    expect(screen.getByRole("button", { name: /All/ })).toHaveTextContent("2");
    expect(screen.getByRole("button", { name: /Unknown/ })).toHaveTextContent("1");
  });

  it("marks the active chip", () => {
    render(
      <QueuePanel rows={rows} filter="unknown" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    expect(screen.getByRole("button", { name: /Unknown/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /All/ })).toHaveAttribute("aria-pressed", "false");
  });

  it("calls onFilterChange when a chip is clicked", () => {
    const onFilterChange = vi.fn();
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={onFilterChange} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Unknown/ }));
    expect(onFilterChange).toHaveBeenCalledWith("unknown");
  });

  it("calls onSelect when a row is clicked", () => {
    const onSelect = vi.fn();
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={onSelect} onDropFiles={vi.fn()} />,
    );
    fireEvent.click(screen.getByText("b.wav"));
    expect(onSelect).toHaveBeenCalledWith("2");
  });

  it("marks the selected row", () => {
    render(
      <QueuePanel rows={rows} filter="all" selectedId="2" onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    expect(screen.getByText("b.wav").closest("button")).toHaveAttribute("aria-current", "true");
  });

  it("shows a placeholder when the filter matches nothing", () => {
    render(
      <QueuePanel rows={rows} filter="failed" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    expect(screen.getByText("No rows match this filter.")).toBeInTheDocument();
  });

  it("calls onDropFiles with the dropped names", () => {
    const onDropFiles = vi.fn();
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={onDropFiles} />,
    );
    const zone = screen.getByTestId("drop-zone");
    const file = new File(["x"], "dropped.wav", { type: "audio/wav" });
    fireEvent.drop(zone, { dataTransfer: { files: [file] } });
    expect(onDropFiles).toHaveBeenCalledWith(["dropped.wav"]);
  });

  it("does not call onDropFiles for an empty drop", () => {
    const onDropFiles = vi.fn();
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={onDropFiles} />,
    );
    fireEvent.drop(screen.getByTestId("drop-zone"), { dataTransfer: { files: [] } });
    expect(onDropFiles).not.toHaveBeenCalled();
  });

  it("marks the drop zone while dragging over it", () => {
    render(
      <QueuePanel rows={rows} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    const zone = screen.getByTestId("drop-zone");
    fireEvent.dragOver(zone);
    expect(zone.className).toContain("queue__drop--active");
    fireEvent.dragLeave(zone);
    expect(zone.className).not.toContain("queue__drop--active");
  });

  it("renders only the visible window for a long queue", () => {
    const many = Array.from({ length: 200 }, (_, index) =>
      row({ id: String(index), fileName: `file-${index}.wav` }),
    );
    render(
      <QueuePanel rows={many} filter="all" selectedId={null} onFilterChange={vi.fn()} onSelect={vi.fn()} onDropFiles={vi.fn()} />,
    );
    expect(screen.queryByText("file-199.wav")).not.toBeInTheDocument();
    expect(screen.getByText("file-0.wav")).toBeInTheDocument();
  });
});
