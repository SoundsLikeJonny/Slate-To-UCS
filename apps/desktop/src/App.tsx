/**
 * The shell: header, split body and status bar.
 *
 * The status bar is the only place the shell touches the engine, and it does so through the
 * typed client, so a contract mismatch shows up as a red status with the reason rather than
 * as a blank window.
 */
import { useEffect, useState } from "react";

import { ErrorBoundary } from "./app/ErrorBoundary.tsx";
import { AboutPanel } from "./app/About.tsx";
import { ExportPanel } from "./app/ExportPanel.tsx";
import { Inspector } from "./app/Inspector.tsx";
import { QueuePanel } from "./app/Queue.tsx";
import { SplitPane } from "./app/layout.tsx";
import { ROUTES, routeHref, routeLabel, useRoute } from "./app/router.tsx";
import { EngineClient, EngineError } from "./lib/engine.ts";
import { applyOverride, clearOverride, type QueueFilter, type ReviewRow } from "./lib/queue.ts";
import type { Health, TaxonomyEntry } from "./lib/schema.ts";

/** What the status bar knows about the engine. */
export type EngineStatus =
  | { readonly state: "checking" }
  | { readonly state: "healthy"; readonly health: Health }
  | { readonly state: "unavailable"; readonly reason: string };

/** Ask the engine for its health once, and describe the outcome. */
export async function probeEngine(client: EngineClient): Promise<EngineStatus> {
  try {
    const health = await client.health();
    return { state: "healthy", health };
  } catch (error) {
    const reason =
      error instanceof EngineError ? `${error.kind}: ${error.message}` : String(error);
    return { state: "unavailable", reason };
  }
}

/** The dot class for a status. */
export function statusTone(status: EngineStatus): "ok" | "warn" | "danger" {
  if (status.state === "healthy") return status.health.warm ? "ok" : "warn";
  if (status.state === "checking") return "warn";
  return "danger";
}

/** The one-line summary the status bar shows. */
export function statusText(status: EngineStatus): string {
  switch (status.state) {
    case "checking":
      return "Checking the engine…";
    case "healthy":
      return `ttucs ${status.health.ttucs_version} · UCS ${status.health.ucs_version} · ${
        status.health.warm ? "warm" : "loading"
      }${status.health.offline ? " · offline" : ""}`;
    case "unavailable":
      return `Engine unavailable — ${status.reason}`;
  }
}

/** Props for {@link StatusBar}. */
export interface StatusBarProps {
  /** The current status. */
  readonly status: EngineStatus;
}

/** The bottom bar. */
export function StatusBar({ status }: StatusBarProps): React.ReactElement {
  return (
    <footer className="status" role="status" aria-live="polite">
      <span className={`status__dot status__dot--${statusTone(status)}`} aria-hidden="true" />
      <span>{statusText(status)}</span>
    </footer>
  );
}

/** Props for {@link App}. */
export interface AppProps {
  /** The engine client; omitted in tests that do not want a probe. */
  readonly client?: EngineClient;
  /** The taxonomy the inspector searches; empty until the engine reports one. */
  readonly taxonomy?: readonly TaxonomyEntry[];
}

/** Build a queue row for a dropped file. */
export function droppedRow(name: string): ReviewRow {
  return {
    id: name,
    fileName: name,
    transcript: null,
    prediction: null,
    override: null,
    status: "pending",
    error: null,
  };
}

/** The application shell. */
export function App({ client, taxonomy = [] }: AppProps): React.ReactElement {
  const route = useRoute();
  const [status, setStatus] = useState<EngineStatus>({ state: "checking" });
  const [rows, setRows] = useState<readonly ReviewRow[]>([]);
  const [filter, setFilter] = useState<QueueFilter>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    if (client === undefined) {
      setStatus({ state: "unavailable", reason: "no engine endpoint configured" });
      return;
    }
    let cancelled = false;
    void probeEngine(client).then((next) => {
      if (!cancelled) setStatus(next);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  const selected = rows.find((row) => row.id === selectedId) ?? null;

  /** Replace one row, leaving the rest of the queue untouched. */
  function replaceRow(next: ReviewRow): void {
    setRows((current) => current.map((row) => (row.id === next.id ? next : row)));
  }

  return (
    <ErrorBoundary>
      <div className="app">
        <header className="app__header">
          <h1 className="app__title">Slate-To-UCS</h1>
          <nav className="app__nav" aria-label="Screens">
            {ROUTES.map((candidate) => (
              <a
                key={candidate}
                href={routeHref(candidate)}
                aria-current={candidate === route ? "page" : undefined}
              >
                {routeLabel(candidate)}
              </a>
            ))}
          </nav>
        </header>
        <div className="app__body">
          {route === "workspace" ? (
            <SplitPane
              main={
                <>
                  <QueuePanel
                    rows={rows}
                    filter={filter}
                    selectedId={selectedId}
                    onFilterChange={setFilter}
                    onSelect={setSelectedId}
                    onDropFiles={(names) => {
                      setRows((current) => [...current, ...names.map(droppedRow)]);
                    }}
                  />
                  <ExportPanel rows={rows} onExport={() => undefined} />
                </>
              }
              side={
                <Inspector
                  row={selected}
                  taxonomy={taxonomy}
                  onOverride={(entry) => {
                    if (selected !== null) replaceRow(applyOverride(selected, entry));
                  }}
                  onClearOverride={() => {
                    if (selected !== null) replaceRow(clearOverride(selected));
                  }}
                  onFilenameChange={() => undefined}
                />
              }
            />
          ) : route === "about" ? (
            <AboutPanel
              health={status.state === "healthy" ? status.health : undefined}
              reason={status.state === "unavailable" ? status.reason : undefined}
            />
          ) : (
            <section aria-label="Settings" style={{ padding: "var(--space-4)" }}>
              <p className="placeholder">
                Model, thresholds and default IDs are read from the engine in a later week.
              </p>
            </section>
          )}
        </div>
        <StatusBar status={status} />
      </div>
    </ErrorBoundary>
  );
}
