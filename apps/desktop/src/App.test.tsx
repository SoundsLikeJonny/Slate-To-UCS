import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { App, StatusBar, probeEngine, statusText, statusTone, type EngineStatus } from "./App.tsx";
import { EngineClient, type FetchLike } from "./lib/engine.ts";

const health = {
  schema_version: 1,
  status: "ok",
  warm: true,
  ttucs_version: "0.3.0",
  ucs_version: "8.2.1",
  model: { name: "bow", version: "0.2.0" },
  offline: true,
  unknown_threshold: 0.4,
  category_threshold: 0.55,
};

function clientReturning(body: unknown, status = 200): EngineClient {
  const fetch: FetchLike = () =>
    Promise.resolve(new Response(JSON.stringify(body), { status }));
  return new EngineClient({ endpoint: { baseUrl: "http://127.0.0.1:1", token: "t" }, fetch });
}

describe("probeEngine", () => {
  it("reports a healthy engine", async () => {
    const status = await probeEngine(clientReturning(health));

    expect(status.state).toBe("healthy");
  });

  it("reports an unavailable engine with the error kind", async () => {
    const status = await probeEngine(clientReturning({ error: "no" }, 401));

    expect(status.state).toBe("unavailable");
    if (status.state === "unavailable") {
      expect(status.reason).toContain("unauthorised");
    }
  });
});

describe("statusTone", () => {
  it("is ok for a warm healthy engine", () => {
    expect(statusTone({ state: "healthy", health: { ...health, warm: true } as never })).toBe("ok");
  });

  it("is warn for a healthy but cold engine", () => {
    expect(statusTone({ state: "healthy", health: { ...health, warm: false } as never })).toBe(
      "warn",
    );
  });

  it("is warn while checking", () => {
    expect(statusTone({ state: "checking" })).toBe("warn");
  });

  it("is danger when unavailable", () => {
    expect(statusTone({ state: "unavailable", reason: "x" })).toBe("danger");
  });
});

describe("statusText", () => {
  it("says it is checking", () => {
    expect(statusText({ state: "checking" })).toContain("Checking");
  });

  it("names the versions when healthy", () => {
    const text = statusText({ state: "healthy", health: health as never });

    expect(text).toContain("ttucs 0.3.0");
    expect(text).toContain("UCS 8.2.1");
    expect(text).toContain("warm");
    expect(text).toContain("offline");
  });

  it("says loading when the model is cold", () => {
    const text = statusText({ state: "healthy", health: { ...health, warm: false } as never });

    expect(text).toContain("loading");
  });

  it("includes the reason when unavailable", () => {
    expect(statusText({ state: "unavailable", reason: "transport: refused" })).toContain(
      "transport: refused",
    );
  });
});

describe("StatusBar", () => {
  it("announces the status politely", () => {
    const status: EngineStatus = { state: "unavailable", reason: "no endpoint" };

    render(<StatusBar status={status} />);

    expect(screen.getByRole("status")).toHaveTextContent("Engine unavailable — no endpoint");
  });
});

describe("App", () => {
  it("renders the workspace with both panes", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Slate-To-UCS" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Queue" })).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "Inspector" })).toBeInTheDocument();
  });

  it("marks the active route in the navigation", () => {
    render(<App />);

    expect(screen.getByRole("link", { name: "Workspace" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("link", { name: "Settings" })).not.toHaveAttribute("aria-current");
  });

  it("shows the settings screen when the hash says so", () => {
    globalThis.location.hash = "#/settings";

    render(<App />);

    expect(screen.getByRole("region", { name: "Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Queue" })).not.toBeInTheDocument();
  });

  it("reports a missing endpoint in the status bar", async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole("status")).toHaveTextContent("no engine endpoint configured");
    });
  });

  it("reports a healthy engine in the status bar", async () => {
    render(<App client={clientReturning(health)} />);

    await waitFor(() => {
      expect(screen.getByRole("status")).toHaveTextContent("ttucs 0.3.0");
    });
  });

  it("reports an unreachable engine in the status bar", async () => {
    render(<App client={clientReturning({ error: "no" }, 500)} />);

    await waitFor(() => {
      expect(screen.getByRole("status")).toHaveTextContent("Engine unavailable");
    });
  });
});
