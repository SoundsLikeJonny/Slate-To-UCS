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
  });
});

describe("routeLabel", () => {
  it("names both routes", () => {
    expect(routeLabel("workspace")).toBe("Workspace");
    expect(routeLabel("settings")).toBe("Settings");
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
