/**
 * The telemetry payload, its allowlist and the switches that turn it off.
 *
 * The payload is built from numbers and closed enums only. There is no field that can
 * carry a transcript, a filename, a path or a CatID, and {@link containsUserData}
 * exists so a test can prove that against a list of strings taken from a real queue.
 */

/** The payload schema version. Bumped when a field changes meaning. */
export const TELEMETRY_SCHEMA = 1;

/** The operating system, as a closed set. */
export type TelemetryOs = "windows" | "macos" | "linux" | "unknown";

/** The CPU architecture, as a closed set. */
export type TelemetryArch = "x64" | "arm64" | "unknown";

/** How many files were handled, and how many the owner acted on. */
export interface TelemetryCounts {
  /** Files dropped into the queue. */
  readonly files: number;
  /** Rows the owner marked reviewed. */
  readonly reviewed: number;
  /** Rows the owner overrode. */
  readonly overridden: number;
}

/** How the classifier decided, as counts. */
export interface TelemetryDecisions {
  /** Rows decided confidently. */
  readonly confident: number;
  /** Rows that fell back to the category. */
  readonly categoryFallback: number;
  /** Rows the classifier could not place. */
  readonly unknown: number;
}

/** Two durations, in whole milliseconds. */
export interface TelemetryDurations {
  /** Time from launch to a warm engine. */
  readonly startupMs: number;
  /** Median predict time for one batch. */
  readonly predictP50Ms: number;
}

/** Everything one telemetry event carries. */
export interface TelemetryPayload {
  /** The payload schema version. */
  readonly schema: typeof TELEMETRY_SCHEMA;
  /** A stable random identifier for this installation, never derived from the machine. */
  readonly installId: string;
  /** The application version, for example `0.1.0`. */
  readonly appVersion: string;
  /** The operating system. */
  readonly os: TelemetryOs;
  /** The CPU architecture. */
  readonly arch: TelemetryArch;
  /** File and review counts. */
  readonly counts: TelemetryCounts;
  /** Decision counts. */
  readonly decisions: TelemetryDecisions;
  /** Two durations. */
  readonly durations: TelemetryDurations;
}

/** What {@link buildPayload} needs. */
export interface PayloadInput {
  /** The seed the install ID is derived from. */
  readonly installSeed: string;
  /** The application version. */
  readonly appVersion: string;
  /** The operating system. */
  readonly os: TelemetryOs;
  /** The CPU architecture. */
  readonly arch: TelemetryArch;
  /** File and review counts. */
  readonly counts: TelemetryCounts;
  /** Decision counts. */
  readonly decisions: TelemetryDecisions;
  /** Two durations. */
  readonly durations: TelemetryDurations;
}

/** The environment variables that switch telemetry off or on. */
export interface TelemetryEnv {
  /** `DO_NOT_TRACK`, honoured when set to `1`. */
  readonly DO_NOT_TRACK?: string | undefined;
  /** `SLATE_TELEMETRY`, honoured when set to `0` or `1`. */
  readonly SLATE_TELEMETRY?: string | undefined;
}

/**
 * A 64-bit FNV-1a hash of the seed, as sixteen lowercase hex characters.
 *
 * Two 32-bit passes with different offset bases are concatenated, which is enough for
 * an install identifier and needs no dependency. The result is stable across runs and
 * platforms, so a test can pin it.
 */
export function deriveInstallId(seed: string): string {
  const fnv = (offset: number): string => {
    let hash = offset;
    for (let index = 0; index < seed.length; index += 1) {
      hash ^= seed.charCodeAt(index);
      hash = Math.imul(hash, 0x0100_0193) >>> 0;
    }
    return hash.toString(16).padStart(8, "0");
  };
  return `${fnv(0x811c_9dc5)}${fnv(0x0100_0193)}`;
}

/** Whether a value is a non-negative integer. */
function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

/** Throw unless every count and duration is a non-negative integer. */
function assertCounts(input: PayloadInput): void {
  const values: readonly [string, unknown][] = [
    ["counts.files", input.counts.files],
    ["counts.reviewed", input.counts.reviewed],
    ["counts.overridden", input.counts.overridden],
    ["decisions.confident", input.decisions.confident],
    ["decisions.categoryFallback", input.decisions.categoryFallback],
    ["decisions.unknown", input.decisions.unknown],
    ["durations.startupMs", input.durations.startupMs],
    ["durations.predictP50Ms", input.durations.predictP50Ms],
  ];
  for (const [name, value] of values) {
    if (!isCount(value)) {
      throw new RangeError(`${name} must be a non-negative integer, got ${String(value)}`);
    }
  }
}

/**
 * Build the payload.
 *
 * # Errors
 *
 * Throws a `RangeError` naming the field when a count or duration is not a
 * non-negative integer, so a caller cannot send a negative or fractional count.
 */
export function buildPayload(input: PayloadInput): TelemetryPayload {
  assertCounts(input);
  if (input.appVersion.trim().length === 0) {
    throw new RangeError("appVersion must not be empty");
  }
  return {
    schema: TELEMETRY_SCHEMA,
    installId: deriveInstallId(input.installSeed),
    appVersion: input.appVersion.trim(),
    os: input.os,
    arch: input.arch,
    counts: { ...input.counts },
    decisions: { ...input.decisions },
    durations: { ...input.durations },
  };
}

/**
 * Whether telemetry may be sent.
 *
 * `DO_NOT_TRACK=1` always wins, then `SLATE_TELEMETRY=0`, then `SLATE_TELEMETRY=1`,
 * then the stored setting. Privacy switches beat an opt-in setting, and an explicit
 * environment opt-in beats a setting the user may have forgotten.
 */
export function isTelemetryEnabled(env: TelemetryEnv, settingEnabled: boolean): boolean {
  if (env.DO_NOT_TRACK === "1") {
    return false;
  }
  if (env.SLATE_TELEMETRY === "0") {
    return false;
  }
  if (env.SLATE_TELEMETRY === "1") {
    return true;
  }
  return settingEnabled;
}

/** One row of the "what is sent" panel. */
export interface PayloadField {
  /** The dotted path in the payload. */
  readonly field: string;
  /** One sentence the panel shows verbatim. */
  readonly description: string;
}

/** The "what is sent" panel, in payload order. */
export const PAYLOAD_FIELDS: readonly PayloadField[] = [
  { field: "schema", description: "The version of this payload format." },
  { field: "installId", description: "A random identifier for this installation." },
  { field: "appVersion", description: "The application version." },
  { field: "os", description: "Your operating system, as one of four fixed values." },
  { field: "arch", description: "Your CPU architecture, as one of three fixed values." },
  { field: "counts.files", description: "How many files you dropped in." },
  { field: "counts.reviewed", description: "How many rows you marked reviewed." },
  { field: "counts.overridden", description: "How many rows you overrode." },
  { field: "decisions.confident", description: "How many rows were decided confidently." },
  { field: "decisions.categoryFallback", description: "How many rows fell back to a category." },
  { field: "decisions.unknown", description: "How many rows could not be placed." },
  { field: "durations.startupMs", description: "How long the engine took to warm, in milliseconds." },
  { field: "durations.predictP50Ms", description: "The median predict time, in milliseconds." },
];

/** Every string in the payload, for the leak check. */
function payloadStrings(payload: TelemetryPayload): readonly string[] {
  return [payload.installId, payload.appVersion, payload.os, payload.arch];
}

/**
 * Whether any of `forbidden` appears anywhere in the payload.
 *
 * The payload is serialised once and searched, so a value nested in an object or an
 * array cannot slip past a field-by-field check. An empty needle is ignored, because
 * every string contains it.
 */
export function containsUserData(
  payload: TelemetryPayload,
  forbidden: readonly string[],
): string | null {
  const haystack = JSON.stringify(payload);
  const strings = payloadStrings(payload);
  for (const needle of forbidden) {
    if (needle.length === 0) {
      continue;
    }
    if (haystack.includes(needle)) {
      return needle;
    }
    for (const value of strings) {
      if (value.includes(needle)) {
        return needle;
      }
    }
  }
  return null;
}
