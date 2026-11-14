import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  DEFAULT_INSPECTOR,
  MAX_INSPECTOR,
  MIN_INSPECTOR,
  SplitPane,
  clampInspector,
  readStoredWidth,
  storageKey,
  storeWidth,
} from "./layout.tsx";

describe("clampInspector", () => {
  it("keeps a width inside the range", () => {
    expect(clampInspector(400)).toBe(400);
  });

  it("raises a width below the minimum", () => {
    expect(clampInspector(10)).toBe(MIN_INSPECTOR);
  });

  it("lowers a width above the maximum", () => {
    expect(clampInspector(5_000)).toBe(MAX_INSPECTOR);
  });

  it("rounds a fractional width", () => {
    expect(clampInspector(400.6)).toBe(401);
  });

  it("falls back to the default for a non-finite width", () => {
    expect(clampInspector(Number.NaN)).toBe(DEFAULT_INSPECTOR);
    expect(clampInspector(Number.POSITIVE_INFINITY)).toBe(DEFAULT_INSPECTOR);
  });
});

describe("storageKey", () => {
  it("buckets by viewport width in hundreds", () => {
    expect(storageKey(1_440)).toBe("slate.inspector.width.14");
    expect(storageKey(1_499)).toBe("slate.inspector.width.15");
  });
});

describe("readStoredWidth and storeWidth", () => {
  it("round-trips a width", () => {
    storeWidth(1_440, 420);

    expect(readStoredWidth(1_440)).toBe(420);
  });

  it("returns the default when nothing is stored", () => {
    expect(readStoredWidth(1_440)).toBe(DEFAULT_INSPECTOR);
  });

  it("clamps a width that is out of range before storing it", () => {
    storeWidth(1_440, 9_999);

    expect(readStoredWidth(1_440)).toBe(MAX_INSPECTOR);
  });

  it("keeps widths separate per viewport bucket", () => {
    storeWidth(1_440, 400);
    storeWidth(1_920, 600);

    expect(readStoredWidth(1_440)).toBe(400);
    expect(readStoredWidth(1_920)).toBe(600);
  });

  it("still remembers a width when the webview has no localStorage", () => {
    const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get: () => {
        throw new Error("storage is disabled");
      },
    });

    try {
      storeWidth(1_600, 450);
      expect(readStoredWidth(1_600)).toBe(450);
    } finally {
      if (original === undefined) {
        Reflect.deleteProperty(globalThis, "localStorage");
      } else {
        Object.defineProperty(globalThis, "localStorage", original);
      }
    }
  });
});

describe("SplitPane", () => {
  it("renders both panes and the handle", () => {
    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} />);

    expect(screen.getByText("queue")).toBeInTheDocument();
    expect(screen.getByText("inspector")).toBeInTheDocument();
    expect(screen.getByRole("separator")).toHaveAttribute("aria-valuenow", String(DEFAULT_INSPECTOR));
  });

  it("widens the inspector with the left arrow", () => {
    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} />);
    const handle = screen.getByRole("separator");

    fireEvent.keyDown(handle, { key: "ArrowLeft" });

    expect(handle).toHaveAttribute("aria-valuenow", String(DEFAULT_INSPECTOR + 10));
  });

  it("narrows the inspector with the right arrow", () => {
    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} />);
    const handle = screen.getByRole("separator");

    fireEvent.keyDown(handle, { key: "ArrowRight" });

    expect(handle).toHaveAttribute("aria-valuenow", String(DEFAULT_INSPECTOR - 10));
  });

  it("moves further with shift held", () => {
    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} />);
    const handle = screen.getByRole("separator");

    fireEvent.keyDown(handle, { key: "ArrowLeft", shiftKey: true });

    expect(handle).toHaveAttribute("aria-valuenow", String(DEFAULT_INSPECTOR + 40));
  });

  it("stops at the minimum", () => {
    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} />);
    const handle = screen.getByRole("separator");

    for (let press = 0; press < 20; press += 1) {
      fireEvent.keyDown(handle, { key: "ArrowRight" });
    }

    expect(handle).toHaveAttribute("aria-valuenow", String(MIN_INSPECTOR));
  });

  it("ignores an unrelated key", () => {
    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} />);
    const handle = screen.getByRole("separator");

    fireEvent.keyDown(handle, { key: "Enter" });

    expect(handle).toHaveAttribute("aria-valuenow", String(DEFAULT_INSPECTOR));
  });

  it("restores a remembered width", () => {
    storeWidth(1_440, 500);

    render(<SplitPane main={<p>queue</p>} side={<p>inspector</p>} viewportWidth={1_440} />);

    expect(screen.getByRole("separator")).toHaveAttribute("aria-valuenow", "500");
  });
});
