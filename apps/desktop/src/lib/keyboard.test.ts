import { describe, expect, it } from "vitest";

import {
  BINDINGS,
  BOUND_ACTIONS,
  bindingFor,
  describeBindings,
  resolveAction,
  type Action,
} from "./keyboard.ts";

/** Every action the union declares, so a missing binding is a test failure. */
const ALL_ACTIONS: readonly Action[] = [
  "next",
  "previous",
  "open",
  "edit",
  "confirm",
  "override",
  "play",
  "export",
];

describe("resolveAction", () => {
  it("maps j to next", () => {
    expect(resolveAction({ key: "j" })).toBe("next");
  });

  it("maps k to previous", () => {
    expect(resolveAction({ key: "k" })).toBe("previous");
  });

  it("maps Enter to open", () => {
    expect(resolveAction({ key: "Enter" })).toBe("open");
  });

  it("maps e to edit", () => {
    expect(resolveAction({ key: "e" })).toBe("edit");
  });

  it("maps c to confirm", () => {
    expect(resolveAction({ key: "c" })).toBe("confirm");
  });

  it("maps o to override", () => {
    expect(resolveAction({ key: "o" })).toBe("override");
  });

  it("maps the space bar to play", () => {
    expect(resolveAction({ key: " " })).toBe("play");
  });

  it("maps the legacy space bar name to play", () => {
    expect(resolveAction({ key: "Spacebar" })).toBe("play");
  });

  it("maps the DOM space bar name to play", () => {
    expect(resolveAction({ key: "Space" })).toBe("play");
  });

  it("maps Cmd+E to export", () => {
    expect(resolveAction({ key: "e", metaKey: true })).toBe("export");
  });

  it("maps Ctrl+E to export", () => {
    expect(resolveAction({ key: "e", ctrlKey: true })).toBe("export");
  });

  it("maps Cmd+Shift+E to export", () => {
    expect(resolveAction({ key: "E", metaKey: true, shiftKey: true })).toBe("export");
  });

  it("ignores an uppercase letter without a modifier", () => {
    expect(resolveAction({ key: "J", shiftKey: true })).toBe("next");
  });

  it("ignores a command key that is not E", () => {
    expect(resolveAction({ key: "j", metaKey: true })).toBeNull();
  });

  it("ignores alt, so alt+j is not next", () => {
    expect(resolveAction({ key: "j", altKey: true })).toBeNull();
  });

  it("ignores alt even with the command key", () => {
    expect(resolveAction({ key: "e", metaKey: true, altKey: true })).toBeNull();
  });

  it("ignores every binding while a text field has focus", () => {
    for (const key of ["j", "k", "e", "c", "o", "Enter", " "]) {
      expect(resolveAction({ key, editable: true })).toBeNull();
    }
  });

  it("still exports from a text field, because the command key is deliberate", () => {
    expect(resolveAction({ key: "e", metaKey: true, editable: true })).toBe("export");
  });

  it("ignores an unbound key", () => {
    expect(resolveAction({ key: "q" })).toBeNull();
  });

  it("ignores an unbound named key", () => {
    expect(resolveAction({ key: "Escape" })).toBeNull();
  });

  it("ignores an empty key", () => {
    expect(resolveAction({ key: "" })).toBeNull();
  });

  it("is case-insensitive for letters", () => {
    expect(resolveAction({ key: "O" })).toBe("override");
  });
});

describe("BINDINGS", () => {
  it("covers every action in the union", () => {
    expect([...BOUND_ACTIONS].sort()).toEqual([...ALL_ACTIONS].sort());
  });

  it("binds each action once", () => {
    expect(new Set(BOUND_ACTIONS).size).toBe(BOUND_ACTIONS.length);
  });

  it("gives every action a label a button can show", () => {
    for (const entry of BINDINGS) {
      expect(entry.label.length).toBeGreaterThan(3);
      expect(entry.label.trim()).toBe(entry.label);
    }
  });

  it("gives every action at least one key", () => {
    for (const entry of BINDINGS) {
      expect(entry.keys.length).toBeGreaterThan(0);
    }
  });

  it("names no key twice", () => {
    const keys = BINDINGS.flatMap((entry) => entry.keys);

    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("bindingFor", () => {
  it("returns the binding for an action", () => {
    expect(bindingFor("export").keys).toEqual(["Cmd/Ctrl+E"]);
  });

  it("returns the same object the table holds", () => {
    expect(bindingFor("next")).toBe(BINDINGS[0]);
  });

  it("throws for an action with no binding", () => {
    expect(() => bindingFor("nope" as Action)).toThrow(/no binding for action: nope/);
  });
});

describe("describeBindings", () => {
  it("writes one line per binding", () => {
    expect(describeBindings().split("\n")).toHaveLength(BINDINGS.length);
  });

  it("starts with the next-row line", () => {
    expect(describeBindings().split("\n")[0]).toBe("j — Next row");
  });

  it("shows the export chord", () => {
    expect(describeBindings()).toContain("Cmd/Ctrl+E — Export the queue");
  });
});
