/**
 * The queue: a drop zone, the filter chips and the virtualised row table.
 *
 * The table renders only the rows inside the scroll window plus a small overscan, so a
 * 200-file drop stays responsive. {@link visibleRange} is a pure function so the windowing
 * arithmetic is tested without a layout engine.
 */
import { useMemo, useRef, useState } from "react";

import {
  QUEUE_FILTERS,
  effectiveCatId,
  effectiveConfidence,
  effectiveDecision,
  filterCounts,
  filterLabel,
  filterRows,
  isOverridden,
  type QueueFilter,
  type ReviewRow,
} from "../lib/queue.ts";

/** The height of one row in pixels; the table assumes a fixed height to window it. */
export const ROW_HEIGHT = 34;

/** How many extra rows to render above and below the window. */
export const OVERSCAN = 6;

/** The slice of rows to render for a scroll position. */
export interface VisibleRange {
  /** First rendered index, inclusive. */
  readonly start: number;
  /** Last rendered index, exclusive. */
  readonly end: number;
  /** The offset the rendered block is translated by. */
  readonly offset: number;
}

/**
 * Compute the rendered window.
 *
 * `start` and `end` are clamped to the row count, so a scroll position past the end (which
 * happens for one frame after a filter shrinks the list) renders nothing rather than
 * throwing.
 */
export function visibleRange(
  count: number,
  scrollTop: number,
  viewportHeight: number,
  rowHeight: number = ROW_HEIGHT,
  overscan: number = OVERSCAN,
): VisibleRange {
  if (rowHeight <= 0) {
    throw new RangeError(`row height must be positive, got ${rowHeight}`);
  }
  if (count <= 0) {
    return { start: 0, end: 0, offset: 0 };
  }
  const first = Math.floor(Math.max(0, scrollTop) / rowHeight);
  const visible = Math.ceil(Math.max(0, viewportHeight) / rowHeight);
  const start = Math.max(0, Math.min(count, first - overscan));
  const end = Math.max(start, Math.min(count, first + visible + overscan));
  return { start, end, offset: start * rowHeight };
}

/** The decision badge text. */
export function decisionLabel(row: ReviewRow): string {
  if (isOverridden(row)) {
    return "overridden";
  }
  const decision = effectiveDecision(row);
  if (decision === null) {
    return row.status;
  }
  switch (decision) {
    case "confident":
      return "confident";
    case "category_fallback":
      return "category";
    case "unknown":
      return "unknown";
  }
}

/** The confidence as a whole percentage. */
export function confidencePercent(row: ReviewRow): string {
  return `${Math.round(effectiveConfidence(row) * 100)}%`;
}

/** Props for {@link QueuePanel}. */
export interface QueuePanelProps {
  /** Every row, in queue order. */
  readonly rows: readonly ReviewRow[];
  /** The active filter chip. */
  readonly filter: QueueFilter;
  /** The selected row id, if any. */
  readonly selectedId: string | null;
  /** Called when a chip is chosen. */
  readonly onFilterChange: (filter: QueueFilter) => void;
  /** Called when a row is selected. */
  readonly onSelect: (id: string) => void;
  /** Called with the file names dropped onto the zone. */
  readonly onDropFiles: (names: readonly string[]) => void;
  /** The height of the scroll viewport in pixels. */
  readonly viewportHeight?: number;
}

/** The queue screen. */
export function QueuePanel({
  rows,
  filter,
  selectedId,
  onFilterChange,
  onSelect,
  onDropFiles,
  viewportHeight = 480,
}: QueuePanelProps): React.ReactElement {
  const [scrollTop, setScrollTop] = useState(0);
  const [dragging, setDragging] = useState(false);
  const bodyRef = useRef<HTMLDivElement | null>(null);

  const counts = useMemo(() => filterCounts(rows), [rows]);
  const visible = useMemo(() => filterRows(rows, filter), [rows, filter]);
  const range = visibleRange(visible.length, scrollTop, viewportHeight);
  const windowed = visible.slice(range.start, range.end);

  return (
    <section className="queue" aria-label="Queue">
      <div
        className={`queue__drop${dragging ? " queue__drop--active" : ""}`}
        data-testid="drop-zone"
        onDragOver={(event) => {
          event.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={(event) => {
          event.preventDefault();
          setDragging(false);
          const names = Array.from(event.dataTransfer.files, (file) => file.name);
          if (names.length > 0) {
            onDropFiles(names);
          }
        }}
      >
        Drop recordings here, or use the file dialog.
      </div>

      <div className="queue__filters" role="toolbar" aria-label="Filters">
        {QUEUE_FILTERS.map((candidate) => (
          <button
            key={candidate}
            type="button"
            className={`chip${candidate === filter ? " chip--active" : ""}`}
            aria-pressed={candidate === filter}
            onClick={() => onFilterChange(candidate)}
          >
            {filterLabel(candidate)}
            <span className="chip__count">{counts[candidate]}</span>
          </button>
        ))}
      </div>

      <div
        className="queue__body"
        ref={bodyRef}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
        style={{ height: viewportHeight }}
        data-testid="queue-body"
      >
        <div style={{ height: visible.length * ROW_HEIGHT, position: "relative" }}>
          <div style={{ transform: `translateY(${range.offset}px)` }}>
            {windowed.map((row) => (
              <button
                key={row.id}
                type="button"
                className={`row${row.id === selectedId ? " row--selected" : ""}`}
                style={{ height: ROW_HEIGHT }}
                aria-current={row.id === selectedId ? "true" : undefined}
                onClick={() => onSelect(row.id)}
              >
                <span className="row__name">{row.fileName}</span>
                <span className="row__cat">{effectiveCatId(row) ?? "—"}</span>
                <span className={`row__badge row__badge--${decisionLabel(row)}`}>{decisionLabel(row)}</span>
                <span className="row__confidence">{confidencePercent(row)}</span>
              </button>
            ))}
          </div>
        </div>
        {visible.length === 0 ? <p className="placeholder">No rows match this filter.</p> : null}
      </div>
    </section>
  );
}
