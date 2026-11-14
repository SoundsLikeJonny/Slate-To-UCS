import { describe, expect, it } from "vitest";

import {
  DEFAULT_SETTINGS,
  MAX_ID_LENGTH,
  SETTINGS_KEYS,
  mergeSettings,
  parseSettings,
  serialiseSettings,
  validateField,
  type Settings,
} from "./settings.ts";

describe("DEFAULT_SETTINGS", () => {
  it("is valid under its own validator", () => {
    const { settings, problems } = mergeSettings(DEFAULT_SETTINGS);

    expect(problems).toEqual([]);
    expect(settings).toEqual(DEFAULT_SETTINGS);
  });

  it("leaves telemetry off", () => {
    expect(DEFAULT_SETTINGS.telemetryEnabled).toBe(false);
  });

  it("lets the engine choose a port", () => {
    expect(DEFAULT_SETTINGS.portOverride).toBeNull();
  });

  it("declares every key exactly once", () => {
    expect(new Set(SETTINGS_KEYS).size).toBe(SETTINGS_KEYS.length);
    expect([...SETTINGS_KEYS].sort()).toEqual(Object.keys(DEFAULT_SETTINGS).sort());
  });
});

describe("validateField", () => {
  it("trims a model name", () => {
    expect(validateField("model", "  bow-v2  ")).toEqual({ value: "bow-v2", problem: null });
  });

  it("rejects an empty model name", () => {
    expect(validateField("model", "   ")).toEqual({
      value: DEFAULT_SETTINGS.model,
      problem: "model must not be empty",
    });
  });

  it("rejects a non-string model name", () => {
    expect(validateField("model", 7).problem).toBe("model must not be empty");
  });

  it("accepts the inclusive ends of the confidence range", () => {
    expect(validateField("confidenceThreshold", 0)).toEqual({ value: 0, problem: null });
    expect(validateField("confidenceThreshold", 1)).toEqual({ value: 1, problem: null });
  });

  it("rejects a confidence just outside the range", () => {
    expect(validateField("confidenceThreshold", -0.01).problem).toBe(
      "confidenceThreshold must be a number from 0 to 1",
    );
    expect(validateField("confidenceThreshold", 1.01).problem).toBe(
      "confidenceThreshold must be a number from 0 to 1",
    );
  });

  it("rejects a NaN threshold", () => {
    expect(validateField("categoryThreshold", Number.NaN).problem).toBe(
      "categoryThreshold must be a number from 0 to 1",
    );
  });

  it("rejects an infinite threshold", () => {
    expect(validateField("categoryThreshold", Number.POSITIVE_INFINITY).problem).not.toBeNull();
  });

  it("accepts an empty creator", () => {
    expect(validateField("defaultCreator", "")).toEqual({ value: "", problem: null });
  });

  it("trims a creator", () => {
    expect(validateField("defaultCreator", "  JE  ")).toEqual({ value: "JE", problem: null });
  });

  it("accepts a creator of exactly the maximum length", () => {
    const id = "x".repeat(MAX_ID_LENGTH);

    expect(validateField("defaultSource", id)).toEqual({ value: id, problem: null });
  });

  it("rejects a creator one character too long", () => {
    const id = "x".repeat(MAX_ID_LENGTH + 1);

    expect(validateField("defaultSource", id)).toEqual({
      value: DEFAULT_SETTINGS.defaultSource,
      problem: `defaultSource must be at most ${MAX_ID_LENGTH} characters`,
    });
  });

  it("rejects a non-string creator", () => {
    expect(validateField("defaultCreator", null).problem).toBe("defaultCreator must be a string");
  });

  it("accepts a null port override", () => {
    expect(validateField("portOverride", null)).toEqual({ value: null, problem: null });
  });

  it("accepts the inclusive ends of the port range", () => {
    expect(validateField("portOverride", 1)).toEqual({ value: 1, problem: null });
    expect(validateField("portOverride", 65535)).toEqual({ value: 65535, problem: null });
  });

  it("rejects port zero", () => {
    expect(validateField("portOverride", 0).problem).toBe(
      "portOverride must be null or an integer from 1 to 65535",
    );
  });

  it("rejects port 65536", () => {
    expect(validateField("portOverride", 65536).problem).not.toBeNull();
  });

  it("rejects a fractional port", () => {
    expect(validateField("portOverride", 8080.5).problem).not.toBeNull();
  });

  it("rejects a port given as a string", () => {
    expect(validateField("portOverride", "8080").problem).not.toBeNull();
  });

  it("accepts both telemetry values", () => {
    expect(validateField("telemetryEnabled", true)).toEqual({ value: true, problem: null });
    expect(validateField("telemetryEnabled", false)).toEqual({ value: false, problem: null });
  });

  it("rejects a telemetry value that is not a boolean", () => {
    expect(validateField("telemetryEnabled", "yes").problem).toBe(
      "telemetryEnabled must be true or false",
    );
  });
});

describe("mergeSettings", () => {
  it("returns the defaults for an empty object", () => {
    expect(mergeSettings({})).toEqual({ settings: DEFAULT_SETTINGS, problems: [] });
  });

  it("keeps a valid override", () => {
    const { settings, problems } = mergeSettings({ sttModel: "small.en" });

    expect(problems).toEqual([]);
    expect(settings.sttModel).toBe("small.en");
    expect(settings.model).toBe(DEFAULT_SETTINGS.model);
  });

  it("ignores an unknown key", () => {
    const { settings, problems } = mergeSettings({ futureKnob: 3 });

    expect(problems).toEqual([]);
    expect(settings).toEqual(DEFAULT_SETTINGS);
    expect(settings).not.toHaveProperty("futureKnob");
  });

  it("falls back per field and reports every problem", () => {
    const { settings, problems } = mergeSettings({
      model: "",
      confidenceThreshold: 2,
      portOverride: -1,
    });

    expect(settings.model).toBe(DEFAULT_SETTINGS.model);
    expect(settings.confidenceThreshold).toBe(DEFAULT_SETTINGS.confidenceThreshold);
    expect(settings.portOverride).toBe(DEFAULT_SETTINGS.portOverride);
    expect(problems).toEqual([
      "model must not be empty",
      "confidenceThreshold must be a number from 0 to 1",
      "portOverride must be null or an integer from 1 to 65535",
    ]);
  });

  it("reports problems in declaration order, not input order", () => {
    const { problems } = mergeSettings({ portOverride: 0, model: "" });

    expect(problems).toEqual([
      "model must not be empty",
      "portOverride must be null or an integer from 1 to 65535",
    ]);
  });

  it("rejects an array", () => {
    expect(mergeSettings([])).toEqual({
      settings: DEFAULT_SETTINGS,
      problems: ["settings must be an object"],
    });
  });

  it("rejects a string", () => {
    expect(mergeSettings("nope").problems).toEqual(["settings must be an object"]);
  });

  it("treats undefined as absent", () => {
    expect(mergeSettings(undefined)).toEqual({ settings: DEFAULT_SETTINGS, problems: [] });
  });

  it("treats null as absent", () => {
    expect(mergeSettings(null)).toEqual({ settings: DEFAULT_SETTINGS, problems: [] });
  });
});

describe("serialiseSettings", () => {
  it("writes every key in declaration order", () => {
    const text = serialiseSettings(DEFAULT_SETTINGS);

    expect(Object.keys(JSON.parse(text))).toEqual([...SETTINGS_KEYS]);
  });

  it("ends with a newline", () => {
    expect(serialiseSettings(DEFAULT_SETTINGS).endsWith("}\n")).toBe(true);
  });

  it("round-trips through parseSettings", () => {
    const custom: Settings = {
      ...DEFAULT_SETTINGS,
      model: "bow-v3",
      confidenceThreshold: 0.75,
      defaultCreator: "JE",
      portOverride: 8731,
      telemetryEnabled: true,
    };

    const { settings, problems } = parseSettings(serialiseSettings(custom));

    expect(problems).toEqual([]);
    expect(settings).toEqual(custom);
  });
});

describe("parseSettings", () => {
  it("returns the defaults for null", () => {
    expect(parseSettings(null)).toEqual({ settings: DEFAULT_SETTINGS, problems: [] });
  });

  it("returns the defaults for blank text", () => {
    expect(parseSettings("   \n ")).toEqual({ settings: DEFAULT_SETTINGS, problems: [] });
  });

  it("reports malformed JSON and keeps the defaults", () => {
    expect(parseSettings("{ not json")).toEqual({
      settings: DEFAULT_SETTINGS,
      problems: ["settings file is not valid JSON"],
    });
  });

  it("reports a JSON scalar", () => {
    expect(parseSettings("42").problems).toEqual(["settings must be an object"]);
  });

  it("keeps the valid fields of a partly broken file", () => {
    const { settings, problems } = parseSettings('{"model":"bow-v2","portOverride":"x"}');

    expect(settings.model).toBe("bow-v2");
    expect(settings.portOverride).toBeNull();
    expect(problems).toEqual(["portOverride must be null or an integer from 1 to 65535"]);
  });
});
