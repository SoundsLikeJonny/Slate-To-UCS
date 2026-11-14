import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AboutPanel, aboutRows, orDash } from "./About.tsx";
import type { Health } from "../lib/schema.ts";

const HEALTH: Health = {
  schema_version: 1,
  status: "ok",
  warm: true,
  ttucs_version: "0.3.0",
  ucs_version: "8.2.1",
  model: {
    name: "bow",
    version: "0.2.0",
    source: "bundled",
    manifest_verified: true,
    manifest_error: null,
    path: "C:/models/bow",
  },
  offline: true,
  device: "cpu",
  unknown_threshold: 0.35,
  category_threshold: 0.5,
  top_k: 5,
  limits: { max_texts: 64, max_body_bytes: 65536, max_top_k: 20 },
};

describe("orDash", () => {
  it("renders an em dash for a missing value", () => {
    expect(orDash(undefined)).toBe("—");
    expect(orDash(null)).toBe("—");
    expect(orDash("")).toBe("—");
  });

  it("renders booleans as words", () => {
    expect(orDash(true)).toBe("yes");
    expect(orDash(false)).toBe("no");
  });

  it("renders numbers and strings as text", () => {
    expect(orDash(0)).toBe("0");
    expect(orDash("cpu")).toBe("cpu");
  });
});

describe("aboutRows", () => {
  it("reports the engine versions", () => {
    const rows = aboutRows(HEALTH);

    expect(rows).toContainEqual({ label: "ttucs", value: "0.3.0" });
    expect(rows).toContainEqual({ label: "UCS list", value: "8.2.1" });
  });

  it("joins the model name and version", () => {
    expect(aboutRows(HEALTH)).toContainEqual({ label: "Model", value: "bow 0.2.0" });
  });

  it("reports the manifest state and its error", () => {
    const rows = aboutRows(HEALTH);

    expect(rows).toContainEqual({ label: "Manifest", value: "yes" });
    expect(rows).toContainEqual({ label: "Manifest error", value: "—" });
  });

  it("formats the thresholds to three places", () => {
    const rows = aboutRows(HEALTH);

    expect(rows).toContainEqual({ label: "Unknown threshold", value: "0.350" });
    expect(rows).toContainEqual({ label: "Category threshold", value: "0.500" });
  });

  it("reports the advertised limits", () => {
    const rows = aboutRows(HEALTH);

    expect(rows).toContainEqual({ label: "Max texts", value: "64" });
    expect(rows).toContainEqual({ label: "Max body bytes", value: "65536" });
    expect(rows).toContainEqual({ label: "Max top k", value: "20" });
  });

  it("dashes the limits when the server omits them", () => {
    const rows = aboutRows({ ...HEALTH, limits: undefined, top_k: undefined, device: undefined });

    expect(rows).toContainEqual({ label: "Max texts", value: "—" });
    expect(rows).toContainEqual({ label: "Top k", value: "—" });
    expect(rows).toContainEqual({ label: "Device", value: "—" });
  });

  it("keeps a stable order and no duplicate labels", () => {
    const rows = aboutRows(HEALTH);
    const labels = rows.map((row) => row.label);

    expect(new Set(labels).size).toBe(labels.length);
    expect(labels[0]).toBe("ttucs");
  });
});

describe("AboutPanel", () => {
  it("shows the reason when health is missing", () => {
    render(<AboutPanel reason="the engine is still starting" />);

    expect(screen.getByRole("heading", { name: "About" })).toBeInTheDocument();
    expect(screen.getByText("the engine is still starting")).toBeInTheDocument();
  });

  it("shows a default message when health and reason are both missing", () => {
    render(<AboutPanel />);

    expect(
      screen.getByText("The engine has not reported its health yet."),
    ).toBeInTheDocument();
  });

  it("renders every row as a term and a definition", () => {
    render(<AboutPanel health={HEALTH} />);

    expect(screen.getByText("ttucs")).toBeInTheDocument();
    expect(screen.getByText("0.3.0")).toBeInTheDocument();
    expect(screen.getByText("bow 0.2.0")).toBeInTheDocument();
    expect(screen.getByText("0.350")).toBeInTheDocument();
  });

  it("is labelled for assistive technology", () => {
    render(<AboutPanel health={HEALTH} />);

    expect(screen.getByRole("region", { name: "About" })).toBeInTheDocument();
  });
});
