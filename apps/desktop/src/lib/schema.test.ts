import { describe, expect, it } from "vitest";

import {
  HealthSchema,
  PredictionSchema,
  PredictResponseSchema,
  SCHEMA_VERSION,
  TaxonomyEntrySchema,
  isSupportedSchemaVersion,
} from "./schema.ts";

const health = {
  schema_version: 1,
  status: "ok",
  warm: true,
  ttucs_version: "0.3.0",
  ucs_version: "8.2.1",
  model: { name: "bow", version: "0.2.0", manifest_verified: true },
  offline: true,
  unknown_threshold: 0.4,
  category_threshold: 0.55,
};

const prediction = {
  cat_id: "DOORCreak",
  category: "DOOR",
  subcategory: "Creak",
  cat_short: "DOORCreak",
  category_full: "Doors",
  confidence: 0.91,
  decision: "confident",
  filename: "DOORCreak_Door Creak_NONE_NONE",
};

describe("isSupportedSchemaVersion", () => {
  it("accepts the version this client implements", () => {
    expect(isSupportedSchemaVersion(SCHEMA_VERSION)).toBe(true);
  });

  it("rejects a newer version", () => {
    expect(isSupportedSchemaVersion(SCHEMA_VERSION + 1)).toBe(false);
  });

  it("rejects a version below one", () => {
    expect(isSupportedSchemaVersion(0)).toBe(false);
  });
});

describe("HealthSchema", () => {
  it("parses a complete payload", () => {
    const parsed = HealthSchema.parse(health);

    expect(parsed.model.name).toBe("bow");
    expect(parsed.offline).toBe(true);
  });

  it("rejects a status other than ok", () => {
    const result = HealthSchema.safeParse({ ...health, status: "degraded" });

    expect(result.success).toBe(false);
  });

  it("rejects a missing required field", () => {
    const { warm: _warm, ...withoutWarm } = health;

    expect(HealthSchema.safeParse(withoutWarm).success).toBe(false);
  });

  it("accepts an absent optional limits block", () => {
    const parsed = HealthSchema.parse(health);

    expect(parsed.limits).toBeUndefined();
  });

  it("accepts a null manifest error", () => {
    const parsed = HealthSchema.parse({
      ...health,
      model: { ...health.model, manifest_error: null },
    });

    expect(parsed.model.manifest_error).toBeNull();
  });
});

describe("PredictionSchema", () => {
  it("parses a minimal prediction", () => {
    const parsed = PredictionSchema.parse(prediction);

    expect(parsed.cat_id).toBe("DOORCreak");
    expect(parsed.alternatives).toBeUndefined();
  });

  it("rejects an unknown decision", () => {
    const result = PredictionSchema.safeParse({ ...prediction, decision: "maybe" });

    expect(result.success).toBe(false);
  });

  it("rejects a confidence that is not a number", () => {
    const result = PredictionSchema.safeParse({ ...prediction, confidence: "high" });

    expect(result.success).toBe(false);
  });

  it("parses alternatives and categories when present", () => {
    const parsed = PredictionSchema.parse({
      ...prediction,
      alternatives: [
        {
          cat_id: "DOORSlid",
          category: "DOOR",
          subcategory: "Slide",
          cat_short: "DOORSlid",
          confidence: 0.05,
        },
      ],
      categories: [{ category: "DOOR", confidence: 0.96 }],
    });

    expect(parsed.alternatives).toHaveLength(1);
    expect(parsed.categories?.[0]?.category).toBe("DOOR");
  });
});

describe("PredictResponseSchema", () => {
  it("parses an envelope with one prediction", () => {
    const parsed = PredictResponseSchema.parse({
      schema_version: 1,
      predictions: [prediction],
    });

    expect(parsed.predictions).toHaveLength(1);
  });

  it("rejects a bare list, which is the shape the server must not send", () => {
    expect(PredictResponseSchema.safeParse([prediction]).success).toBe(false);
  });

  it("rejects a non-integer schema version", () => {
    const result = PredictResponseSchema.safeParse({
      schema_version: 1.5,
      predictions: [],
    });

    expect(result.success).toBe(false);
  });
});

describe("TaxonomyEntrySchema", () => {
  it("parses an entry without a description", () => {
    const parsed = TaxonomyEntrySchema.parse({
      cat_id: "WNDWSlam",
      category: "WNDW",
      subcategory: "Slam",
      cat_short: "WNDWSlam",
      category_full: "Windows",
    });

    expect(parsed.description).toBeUndefined();
  });

  it("rejects an entry without a cat_id", () => {
    expect(
      TaxonomyEntrySchema.safeParse({
        category: "WNDW",
        subcategory: "Slam",
        cat_short: "WNDWSlam",
        category_full: "Windows",
      }).success,
    ).toBe(false);
  });
});
