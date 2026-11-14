import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ROUTES, navigate, parseRoute, routeHref, routeLabel, useRoute } from "./router.tsx";

describe("parseRoute", () => {
  it("defaults to the workspace", () => {
    expect(parseRoute("")).toBe("workspace");
    expect(parseRoute("#")).toBe("workspace");
    expect(parseRoute("#/")).toBe("workspace");
  });

  it("reads the settings route", () => {
    expect(parseRoute("#/settings")).toBe("settings");
    expect(parseRoute("#settings")).toBe("settings");
    expect(parseRoute("#/settings/")).toBe("settings");
  });

  it("reads the about route", () => {
    expect(parseRoute("#/about")).toBe("about");
    expect(parseRoute("#about")).toBe("about");
    expect(parseRoute("#/about/")).toBe("about");
  });

  it("falls back to the workspace for an unknown route", () => {
    expect(parseRoute("#/nope")).toBe("workspace");
  });

  it("ignores a query string after the route", () => {
    expect(parseRoute("#/settings?tab=model")).toBe("workspace");
  });
});

describe("routeHref", () => {
  it("round-trips through parseRoute", () => {
    for (const route of ROUTES) {
      expect(parseRoute(routeHref(route))).toBe(route);
    }
  });

  it("produces a hash-prefixed path", () => {
    expect(routeHref("workspace")).toBe("#/workspace");
    expect(routeHref("settings")).toBe("#/settings");
    expect(routeHref("about")).toBe("#/about");
  });
});

describe("routeLabel", () => {
  it("names every route", () => {
    expect(routeLabel("workspace")).toBe("Workspace");
    expect(routeLabel("settings")).toBe("Settings");
    expect(routeLabel("about")).toBe("About");
  });

  it("labels every route in ROUTES", () => {
    for (const route of ROUTES) {
      expect(routeLabel(route).length).toBeGreaterThan(0);
    }
  });
});

describe("useRoute", () => {
  it("follows the hash", () => {
    const { result } = renderHook(() => useRoute());
    expect(result.current).toBe("workspace");

    act(() => {
      navigate("settings");
    });

    expect(result.current).toBe("settings");
  });

  it("returns to the workspace when the hash is cleared", () => {
    const { result } = renderHook(() => useRoute());
    act(() => {
      navigate("settings");
    });
    act(() => {
      globalThis.location.hash = "";
      globalThis.dispatchEvent(new Event("hashchange"));
    });

    expect(result.current).toBe("workspace");
  });
});
