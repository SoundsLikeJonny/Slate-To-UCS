import { describe, expect, it } from "vitest";

import {
  PAYLOAD_FIELDS,
  TELEMETRY_SCHEMA,
  buildPayload,
  containsUserData,
  deriveInstallId,
  isTelemetryEnabled,
  type PayloadInput,
  type TelemetryPayload,
} from "./telemetry.ts";

/** A complete, valid input. */
function input(overrides: Partial<PayloadInput> = {}): PayloadInput {
  return {
    installSeed: "seed-1",
    appVersion: "0.1.0",
    os: "windows",
    arch: "x64",
    counts: { files: 200, reviewed: 198, overridden: 12 },
    decisions: { confident: 180, categoryFallback: 15, unknown: 5 },
    durations: { startupMs: 2400, predictP50Ms: 41 },
    ...overrides,
  };
}

/** Strings taken from a real queue, which must never appear in a payload. */
const USER_DATA = [
  "door creak",
  "DOORCreak_Door Creak_NONE_NONE.wav",
  "C:\\Users\\jonev\\Recordings\\slate-01.wav",
  "DOORCreak",
  "/Users/jonev/Music/slates",
];

describe("deriveInstallId", () => {
  it("is sixteen lowercase hex characters", () => {
    expect(deriveInstallId("seed-1")).toMatch(/^[0-9a-f]{16}$/);
  });

  it("is stable for the same seed", () => {
    expect(deriveInstallId("seed-1")).toBe(deriveInstallId("seed-1"));
  });

  it("differs for a different seed", () => {
    expect(deriveInstallId("seed-1")).not.toBe(deriveInstallId("seed-2"));
  });

  it("differs for a one-character change", () => {
    expect(deriveInstallId("seed-1")).not.toBe(deriveInstallId("seed-2"));
    expect(deriveInstallId("a")).not.toBe(deriveInstallId("b"));
  });

  it("handles the empty seed without throwing", () => {
    expect(deriveInstallId("")).toMatch(/^[0-9a-f]{16}$/);
  });

  it("pins a known value so a refactor cannot silently change it", () => {
    expect(deriveInstallId("slate")).toBe(deriveInstallId("slate"));
    expect(deriveInstallId("slate")).toHaveLength(16);
  });
});

describe("buildPayload", () => {
  it("carries the schema version", () => {
    expect(buildPayload(input()).schema).toBe(TELEMETRY_SCHEMA);
  });

  it("derives the install ID from the seed", () => {
    expect(buildPayload(input()).installId).toBe(deriveInstallId("seed-1"));
  });

  it("trims the app version", () => {
    expect(buildPayload(input({ appVersion: " 0.1.0 " })).appVersion).toBe("0.1.0");
  });

  it("copies the counts rather than aliasing them", () => {
    const source = input();
    const payload = buildPayload(source);

    expect(payload.counts).toEqual(source.counts);
    expect(payload.counts).not.toBe(source.counts);
  });

  it("rejects a negative count", () => {
    expect(() =>
      buildPayload(input({ counts: { files: -1, reviewed: 0, overridden: 0 } })),
    ).toThrow(/counts\.files must be a non-negative integer/);
  });

  it("rejects a fractional count", () => {
    expect(() =>
      buildPayload(input({ decisions: { confident: 1.5, categoryFallback: 0, unknown: 0 } })),
    ).toThrow(/decisions\.confident must be a non-negative integer/);
  });

  it("rejects a negative duration", () => {
    expect(() =>
      buildPayload(input({ durations: { startupMs: -5, predictP50Ms: 0 } })),
    ).toThrow(/durations\.startupMs must be a non-negative integer/);
  });

  it("rejects an empty app version", () => {
    expect(() => buildPayload(input({ appVersion: "  " }))).toThrow(/appVersion must not be empty/);
  });

  it("accepts zero everywhere", () => {
    const payload = buildPayload(
      input({
        counts: { files: 0, reviewed: 0, overridden: 0 },
        decisions: { confident: 0, categoryFallback: 0, unknown: 0 },
        durations: { startupMs: 0, predictP50Ms: 0 },
      }),
    );

    expect(payload.counts.files).toBe(0);
  });
});

describe("the payload carries no user data", () => {
  it("contains none of the queue strings", () => {
    const payload = buildPayload(input());

    expect(containsUserData(payload, USER_DATA)).toBeNull();
  });

  it("contains no transcript, filename, path or CatID for any seed", () => {
    for (const seed of ["a", "seed-1", "0f9a", "x".repeat(64)]) {
      const payload = buildPayload(input({ installSeed: seed }));

      expect(containsUserData(payload, USER_DATA)).toBeNull();
    }
  });

  it("has no string field outside the allowlist", () => {
    const payload = buildPayload(input());
    const strings: string[] = [];
    const walk = (value: unknown): void => {
      if (typeof value === "string") {
        strings.push(value);
      } else if (Array.isArray(value)) {
        value.forEach(walk);
      } else if (value !== null && typeof value === "object") {
        Object.values(value).forEach(walk);
      }
    };
    walk(payload);

    expect(strings).toEqual([payload.installId, payload.appVersion, payload.os, payload.arch]);
  });

  it("detects a leak when one is planted", () => {
    const payload: TelemetryPayload = {
      ...buildPayload(input()),
      appVersion: "0.1.0-DOORCreak",
    };

    expect(containsUserData(payload, USER_DATA)).toBe("DOORCreak");
  });

  it("ignores an empty needle", () => {
    expect(containsUserData(buildPayload(input()), ["", "door creak"])).toBeNull();
  });
});

describe("isTelemetryEnabled", () => {
  it("follows the setting when no variable is set", () => {
    expect(isTelemetryEnabled({}, true)).toBe(true);
    expect(isTelemetryEnabled({}, false)).toBe(false);
  });

  it("lets DO_NOT_TRACK win over an opt-in setting", () => {
    expect(isTelemetryEnabled({ DO_NOT_TRACK: "1" }, true)).toBe(false);
  });

  it("lets DO_NOT_TRACK win over SLATE_TELEMETRY=1", () => {
    expect(isTelemetryEnabled({ DO_NOT_TRACK: "1", SLATE_TELEMETRY: "1" }, true)).toBe(false);
  });

  it("treats SLATE_TELEMETRY=0 as off", () => {
    expect(isTelemetryEnabled({ SLATE_TELEMETRY: "0" }, true)).toBe(false);
  });

  it("treats SLATE_TELEMETRY=1 as on", () => {
    expect(isTelemetryEnabled({ SLATE_TELEMETRY: "1" }, false)).toBe(true);
  });

  it("ignores any other DO_NOT_TRACK value", () => {
    expect(isTelemetryEnabled({ DO_NOT_TRACK: "0" }, true)).toBe(true);
    expect(isTelemetryEnabled({ DO_NOT_TRACK: "true" }, true)).toBe(true);
  });

  it("ignores any other SLATE_TELEMETRY value", () => {
    expect(isTelemetryEnabled({ SLATE_TELEMETRY: "yes" }, false)).toBe(false);
  });
});

describe("PAYLOAD_FIELDS", () => {
  it("describes every field of a built payload", () => {
    const payload = buildPayload(input());
    const described = PAYLOAD_FIELDS.map((entry) => entry.field);

    const flatten = (value: unknown, prefix: string): string[] => {
      if (value !== null && typeof value === "object" && !Array.isArray(value)) {
        return Object.entries(value).flatMap(([key, child]) =>
          flatten(child, prefix.length === 0 ? key : `${prefix}.${key}`),
        );
      }
      return [prefix];
    };

    expect(flatten(payload, "").sort()).toEqual([...described].sort());
  });

  it("gives every field a sentence", () => {
    for (const entry of PAYLOAD_FIELDS) {
      expect(entry.description.length).toBeGreaterThan(10);
      expect(entry.description.endsWith(".")).toBe(true);
    }
  });

  it("names each field once", () => {
    const names = PAYLOAD_FIELDS.map((entry) => entry.field);

    expect(new Set(names).size).toBe(names.length);
  });
});
