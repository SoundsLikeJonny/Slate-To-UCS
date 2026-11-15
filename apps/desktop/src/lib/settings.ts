/**
 * User settings: defaults, validation and persistence.
 *
 * Every value here is user-editable, so every value is validated on the way in and
 * falls back to its default with a recorded problem rather than throwing. A settings
 * file written by a newer build, hand-edited, or truncated must never stop the app
 * from starting.
 */

/** The settings the app runs with. */
export interface Settings {
  /** The classifier model identifier the engine should load. */
  readonly model: string;
  /** Below this confidence a row is shown as needing review, in `[0, 1]`. */
  readonly confidenceThreshold: number;
  /** Below this category score the engine falls back to the category, in `[0, 1]`. */
  readonly categoryThreshold: number;
  /** The creator ID written into a filename, or the empty string for none. */
  readonly defaultCreator: string;
  /** The source ID written into a filename, or the empty string for none. */
  readonly defaultSource: string;
  /** The speech-to-text model name passed to Whisper. */
  readonly sttModel: string;
  /** A fixed engine port, or `null` to let the engine choose one. */
  readonly portOverride: number | null;
  /** Whether anonymous usage counts may be sent. */
  readonly telemetryEnabled: boolean;
}

/** The settings a fresh install uses. */
export const DEFAULT_SETTINGS: Settings = {
  model: "bow-v1",
  confidenceThreshold: 0.6,
  categoryThreshold: 0.35,
  defaultCreator: "",
  defaultSource: "",
  sttModel: "base.en",
  portOverride: null,
  telemetryEnabled: false,
};

/** The longest creator or source ID accepted, in characters. */
export const MAX_ID_LENGTH = 64;

/** The result of reading settings: the values to use and what was wrong with the input. */
export interface SettingsResult {
  /** The settings to run with. Always complete and always valid. */
  readonly settings: Settings;
  /** One sentence per rejected value, in the order the fields are declared. */
  readonly problems: readonly string[];
}

/** Whether a value is a finite number inside an inclusive range. */
function inRange(value: unknown, low: number, high: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= low && value <= high;
}

/** A trimmed string, or `undefined` when the value is not a string. */
function trimmed(value: unknown): string | undefined {
  return typeof value === "string" ? value.trim() : undefined;
}

/**
 * Validate one field, returning the value to use and a problem when it was rejected.
 *
 * Exported so the settings panel can validate a single control as the user types
 * without re-validating the whole object.
 */
export function validateField<K extends keyof Settings>(
  key: K,
  value: unknown,
): { readonly value: Settings[K]; readonly problem: string | null } {
  const fallback = DEFAULT_SETTINGS[key];
  switch (key) {
    case "model":
    case "sttModel": {
      const text = trimmed(value);
      if (text === undefined || text.length === 0) {
        return { value: fallback, problem: `${key} must not be empty` };
      }
      return { value: text as Settings[K], problem: null };
    }
    case "defaultCreator":
    case "defaultSource": {
      const text = trimmed(value);
      if (text === undefined) {
        return { value: fallback, problem: `${key} must be a string` };
      }
      if (text.length > MAX_ID_LENGTH) {
        return {
          value: fallback,
          problem: `${key} must be at most ${MAX_ID_LENGTH} characters`,
        };
      }
      return { value: text as Settings[K], problem: null };
    }
    case "confidenceThreshold":
    case "categoryThreshold": {
      if (!inRange(value, 0, 1)) {
        return { value: fallback, problem: `${key} must be a number from 0 to 1` };
      }
      return { value: value as Settings[K], problem: null };
    }
    case "portOverride": {
      if (value === null) {
        return { value: null as Settings[K], problem: null };
      }
      if (typeof value !== "number" || !Number.isInteger(value) || value < 1 || value > 65535) {
        return {
          value: fallback,
          problem: "portOverride must be null or an integer from 1 to 65535",
        };
      }
      return { value: value as Settings[K], problem: null };
    }
    case "telemetryEnabled": {
      if (typeof value !== "boolean") {
        return { value: fallback, problem: "telemetryEnabled must be true or false" };
      }
      return { value: value as Settings[K], problem: null };
    }
    default: {
      const unreachable: never = key;
      throw new Error(`unhandled settings key: ${String(unreachable)}`);
    }
  }
}

/** The keys of {@link Settings}, in declaration order. */
export const SETTINGS_KEYS: readonly (keyof Settings)[] = [
  "model",
  "confidenceThreshold",
  "categoryThreshold",
  "defaultCreator",
  "defaultSource",
  "sttModel",
  "portOverride",
  "telemetryEnabled",
];

/**
 * Merge a partial object over the defaults, validating every field that is present.
 *
 * Unknown keys are ignored rather than rejected: a settings file written by a newer
 * build must still load. A missing key keeps its default silently, because a missing
 * key is not a mistake.
 */
export function mergeSettings(partial: unknown): SettingsResult {
  const problems: string[] = [];
  if (partial === null || typeof partial !== "object" || Array.isArray(partial)) {
    if (partial !== undefined && partial !== null) {
      problems.push("settings must be an object");
    }
    return { settings: DEFAULT_SETTINGS, problems };
  }
  const source = partial as Record<string, unknown>;
  const merged: Record<string, unknown> = { ...DEFAULT_SETTINGS };
  for (const key of SETTINGS_KEYS) {
    if (!(key in source)) {
      continue;
    }
    const { value, problem } = validateField(key, source[key]);
    merged[key] = value;
    if (problem !== null) {
      problems.push(problem);
    }
  }
  return { settings: merged as unknown as Settings, problems };
}

/** Serialise settings as pretty JSON with a trailing newline. */
export function serialiseSettings(settings: Settings): string {
  const ordered: Record<string, unknown> = {};
  for (const key of SETTINGS_KEYS) {
    ordered[key] = settings[key];
  }
  return `${JSON.stringify(ordered, null, 2)}\n`;
}

/**
 * Read settings from stored text.
 *
 * `null` means nothing has been stored yet, which is not a problem. Text that is not
 * JSON, or not an object, is reported and replaced by the defaults.
 */
export function parseSettings(text: string | null): SettingsResult {
  if (text === null || text.trim().length === 0) {
    return { settings: DEFAULT_SETTINGS, problems: [] };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return { settings: DEFAULT_SETTINGS, problems: ["settings file is not valid JSON"] };
  }
  return mergeSettings(parsed);
}
