/**
 * The inspector: transcript, ranked alternatives, decision badge, editable file name and the
 * keyboard-first CatID override.
 *
 * The override search is a pure function over the taxonomy so its ranking is tested without
 * a DOM, and the input handles arrows and Enter so a reviewer never has to reach for a mouse.
 */
import { useMemo, useState } from "react";

import { validateFilename } from "../lib/filename.ts";
import { effectiveCatId, effectiveConfidence, effectiveDecision, isOverridden, type ReviewRow } from "../lib/queue.ts";
import type { TaxonomyEntry } from "../lib/schema.ts";

/** How many override candidates the search shows at once. */
export const OVERRIDE_LIMIT = 8;

/**
 * Rank taxonomy entries against a query.
 *
 * A CatID prefix beats a CatID substring, which beats a category or subcategory match; ties
 * break on CatID so the list is stable. An empty query returns the first `limit` entries in
 * taxonomy order, which is what the list shows before the user types.
 */
export function searchTaxonomy(
  entries: readonly TaxonomyEntry[],
  query: string,
  limit: number = OVERRIDE_LIMIT,
): TaxonomyEntry[] {
  if (!Number.isInteger(limit) || limit < 1) {
    throw new RangeError(`limit must be a positive integer, got ${limit}`);
  }
  const needle = query.trim().toLowerCase();
  if (needle.length === 0) {
    return entries.slice(0, limit);
  }
  const scored: Array<{ entry: TaxonomyEntry; score: number }> = [];
  for (const entry of entries) {
    const catId = entry.cat_id.toLowerCase();
    const category = entry.category.toLowerCase();
    const subcategory = entry.subcategory.toLowerCase();
    let score: number;
    if (catId.startsWith(needle)) {
      score = 0;
    } else if (catId.includes(needle)) {
      score = 1;
    } else if (subcategory.includes(needle)) {
      score = 2;
    } else if (category.includes(needle)) {
      score = 3;
    } else {
      continue;
    }
    scored.push({ entry, score });
  }
  scored.sort((left, right) => {
    if (left.score !== right.score) {
      return left.score - right.score;
    }
    return left.entry.cat_id.localeCompare(right.entry.cat_id);
  });
  return scored.slice(0, limit).map((item) => item.entry);
}

/** Props for {@link Inspector}. */
export interface InspectorProps {
  /** The selected row, or `null` when nothing is selected. */
  readonly row: ReviewRow | null;
  /** The taxonomy the override searches. */
  readonly taxonomy: readonly TaxonomyEntry[];
  /** Called with the chosen entry. */
  readonly onOverride: (entry: TaxonomyEntry) => void;
  /** Called when the override is cleared. */
  readonly onClearOverride: () => void;
  /** Called with a new file name as the user edits it. */
  readonly onFilenameChange: (filename: string) => void;
}

/** The inspector pane. */
export function Inspector({
  row,
  taxonomy,
  onOverride,
  onClearOverride,
  onFilenameChange,
}: InspectorProps): React.ReactElement {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [draft, setDraft] = useState<string | null>(null);

  const candidates = useMemo(() => searchTaxonomy(taxonomy, query), [taxonomy, query]);
  const knownCatIds = useMemo(() => new Set(taxonomy.map((entry) => entry.cat_id)), [taxonomy]);

  if (row === null) {
    return (
      <section className="inspector" aria-label="Inspector">
        <p className="placeholder">Select a row to inspect its prediction.</p>
      </section>
    );
  }

  const filename = draft ?? row.prediction?.filename ?? "";
  const verdict = validateFilename(filename, knownCatIds);
  const decision = effectiveDecision(row);

  return (
    <section className="inspector" aria-label="Inspector">
      <h2 className="inspector__title">{row.fileName}</h2>

      <div className="inspector__badges">
        <span className={`badge badge--${decision ?? "none"}`}>{decision ?? row.status}</span>
        <span className="badge">{Math.round(effectiveConfidence(row) * 100)}%</span>
        {isOverridden(row) ? <span className="badge badge--overridden">overridden</span> : null}
      </div>

      <label className="field">
        <span className="field__label">Transcript</span>
        <textarea className="field__input" readOnly value={row.transcript ?? ""} rows={3} />
      </label>

      <label className="field">
        <span className="field__label">File name</span>
        <input
          className="field__input"
          value={filename}
          aria-invalid={!verdict.ok}
          onChange={(event) => {
            setDraft(event.target.value);
            onFilenameChange(event.target.value);
          }}
        />
        {verdict.ok ? null : (
          <span className="field__error" role="alert">
            {verdict.reason}
          </span>
        )}
      </label>

      <div className="field">
        <span className="field__label">Alternatives</span>
        <ol className="alternatives">
          {(row.prediction?.alternatives ?? []).map((alternative) => (
            <li key={alternative.cat_id}>
              <span className="alternatives__cat">{alternative.cat_id}</span>
              <span className="alternatives__confidence">{Math.round(alternative.confidence * 100)}%</span>
            </li>
          ))}
        </ol>
      </div>

      <div className="field">
        <span className="field__label">Keywords</span>
        <p className="keywords">{(row.prediction?.keywords ?? []).join(", ") || "—"}</p>
      </div>

      <div className="field">
        <span className="field__label">Override CatID</span>
        <input
          className="field__input"
          role="combobox"
          aria-expanded={candidates.length > 0}
          aria-controls="override-list"
          aria-activedescendant={candidates[active] === undefined ? undefined : `override-${candidates[active].cat_id}`}
          placeholder="Type a CatID, category or subcategory"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((index) => Math.min(index + 1, Math.max(0, candidates.length - 1)));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((index) => Math.max(0, index - 1));
            } else if (event.key === "Enter") {
              const chosen = candidates[active];
              if (chosen !== undefined) {
                event.preventDefault();
                onOverride(chosen);
                setQuery("");
                setActive(0);
              }
            } else if (event.key === "Escape") {
              setQuery("");
              setActive(0);
            }
          }}
        />
        <ul className="override-list" id="override-list" role="listbox">
          {candidates.map((candidate, index) => (
            <li
              key={candidate.cat_id}
              id={`override-${candidate.cat_id}`}
              role="option"
              aria-selected={index === active}
              className={index === active ? "override-list__item override-list__item--active" : "override-list__item"}
              onMouseDown={(event) => {
                event.preventDefault();
                onOverride(candidate);
                setQuery("");
                setActive(0);
              }}
            >
              <span className="override-list__cat">{candidate.cat_id}</span>
              <span className="override-list__path">
                {candidate.category} / {candidate.subcategory}
              </span>
            </li>
          ))}
        </ul>
        {isOverridden(row) ? (
          <button type="button" className="button" onClick={onClearOverride}>
            Clear override (back to {row.prediction?.cat_id ?? "the guess"})
          </button>
        ) : null}
        <p className="field__hint">Current CatID: {effectiveCatId(row) ?? "—"}</p>
      </div>
    </section>
  );
}
