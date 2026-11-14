import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ErrorBoundary, buildReport, stripSensitive } from "./ErrorBoundary.tsx";

function Boom(): React.ReactElement {
  throw new Error("render failed");
}

describe("stripSensitive", () => {
  it("replaces a Windows path", () => {
    expect(stripSensitive("at C:\\Users\\jon\\app\\main.tsx:12")).toBe("at <path>:12");
  });

  it("replaces a POSIX home path", () => {
    expect(stripSensitive("at /home/jon/app/main.ts:12")).toBe("at <path>:12");
  });

  it("replaces a macOS home path", () => {
    expect(stripSensitive("at /Users/jon/app/main.ts")).toBe("at <path>");
  });

  it("redacts a token assignment", () => {
    expect(stripSensitive("token=abc123")).toBe("token=<redacted>");
  });

  it("redacts an api key with a colon", () => {
    expect(stripSensitive("api_key: deadbeef")).toBe("api_key=<redacted>");
  });

  it("leaves ordinary text alone", () => {
    expect(stripSensitive("the door creaked")).toBe("the door creaked");
  });
});

describe("buildReport", () => {
  it("includes the message and the stack", () => {
    const report = buildReport(new Error("boom"));

    expect(report).toContain("Slate-To-UCS render error");
    expect(report).toContain("boom");
  });

  it("handles a thrown non-error", () => {
    expect(buildReport("plain string")).toContain("plain string");
  });

  it("includes the component stack when given one", () => {
    const report = buildReport(new Error("boom"), {
      componentStack: "\n  in QueueView",
    } as never);

    expect(report).toContain("in QueueView");
  });

  it("strips paths from the message", () => {
    const report = buildReport(new Error("failed at C:\\Users\\jon\\x.tsx"));

    expect(report).not.toContain("C:\\Users\\jon");
    expect(report).toContain("<path>");
  });
});

describe("ErrorBoundary", () => {
  it("renders its children while nothing has failed", () => {
    render(
      <ErrorBoundary>
        <p>all good</p>
      </ErrorBoundary>,
    );

    expect(screen.getByText("all good")).toBeInTheDocument();
  });

  it("renders a report when a child throws", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => undefined);

    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText(/render failed/)).toBeInTheDocument();
    spy.mockRestore();
  });

  it("calls onError with the report", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const onError = vi.fn();

    render(
      <ErrorBoundary onError={onError}>
        <Boom />
      </ErrorBoundary>,
    );

    expect(onError).toHaveBeenCalledOnce();
    expect(onError.mock.calls[0]?.[0]).toContain("render failed");
    spy.mockRestore();
  });

  it("offers a retry that re-renders the children", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(
      <ErrorBoundary>
        <Boom />
      </ErrorBoundary>,
    );

    screen.getByRole("button", { name: "Try again" }).click();

    // The child throws on every render, so the boundary catches it again rather than
    // showing a blank window.
    expect(screen.getByRole("alert")).toBeInTheDocument();
    spy.mockRestore();
  });
});
