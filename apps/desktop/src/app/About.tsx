/**
 * The About panel: what the engine says about itself.
 *
 * Every field comes from `/health`, so the panel is a view over one payload rather than a
 * second source of truth. A missing optional field renders as an em dash instead of
 * disappearing, so the layout does not jump between a development and a packaged build.
 */
import type { Health } from "../lib/schema.ts";

/** One labelled row. */
export interface AboutRow {
  /** The label shown on the left. */
  readonly label: string;
  /** The value shown on the right. */
  readonly value: string;
}

/** Render a value, or an em dash when it is absent. */
export function orDash(value: string | number | boolean | null | undefined): string {
  if (value === undefined || value === null || value === "") return "—";
  if (typeof value === "boolean") return value ? "yes" : "no";
  return String(value);
}

/** The rows the panel shows for a health payload, in display order. */
export function aboutRows(health: Health): readonly AboutRow[] {
  return [
    { label: "ttucs", value: orDash(health.ttucs_version) },
    { label: "UCS list", value: orDash(health.ucs_version) },
    { label: "Model", value: `${health.model.name} ${health.model.version}`.trim() },
    { label: "Model source", value: orDash(health.model.source) },
    { label: "Manifest", value: orDash(health.model.manifest_verified) },
    { label: "Manifest error", value: orDash(health.model.manifest_error) },
    { label: "Model path", value: orDash(health.model.path) },
    { label: "Device", value: orDash(health.device) },
    { label: "Offline", value: orDash(health.offline) },
    { label: "Warm", value: orDash(health.warm) },
    { label: "Unknown threshold", value: health.unknown_threshold.toFixed(3) },
    { label: "Category threshold", value: health.category_threshold.toFixed(3) },
    { label: "Top k", value: orDash(health.top_k) },
    { label: "Max texts", value: orDash(health.limits?.max_texts) },
    { label: "Max body bytes", value: orDash(health.limits?.max_body_bytes) },
    { label: "Max top k", value: orDash(health.limits?.max_top_k) },
  ];
}

/** Props for {@link AboutPanel}. */
export interface AboutPanelProps {
  /** The health payload, or `undefined` while it is unknown. */
  readonly health?: Health | undefined;
  /** Why the payload is missing, when it is. */
  readonly reason?: string | undefined;
}

/** The panel itself. */
export function AboutPanel({ health, reason }: AboutPanelProps): React.ReactElement {
  if (health === undefined) {
    return (
      <section aria-label="About" className="about" style={{ padding: "var(--space-4)" }}>
        <h2 className="about__title">About</h2>
        <p className="placeholder">{reason ?? "The engine has not reported its health yet."}</p>
      </section>
    );
  }
  return (
    <section aria-label="About" className="about" style={{ padding: "var(--space-4)" }}>
      <h2 className="about__title">About</h2>
      <dl className="about__list">
        {aboutRows(health).map((row) => (
          <div className="about__row" key={row.label}>
            <dt>{row.label}</dt>
            <dd>{row.value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
