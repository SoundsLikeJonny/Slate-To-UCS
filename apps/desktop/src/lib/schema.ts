/**
 * zod schemas mirroring `packages/contract/openapi.yaml`.
 *
 * Every response the UI receives is parsed through one of these before it reaches a
 * component, so a shape mismatch surfaces as a visible error instead of a blank cell. The
 * schemas are deliberately strict about required fields and lenient about optional ones:
 * the contract marks a field optional only when an older server may omit it.
 */
import { z } from "zod";

/** The contract version this client implements. */
export const SCHEMA_VERSION = 1;

/** The three decisions the classifier can return. */
export const DecisionSchema = z.enum(["confident", "category_fallback", "unknown"]);

/** A decision as a TypeScript union. */
export type Decision = z.infer<typeof DecisionSchema>;

/** One ranked alternative CatID. */
export const AlternativeSchema = z.object({
  cat_id: z.string(),
  category: z.string(),
  subcategory: z.string(),
  cat_short: z.string(),
  confidence: z.number(),
});

/** A category-level score, used by the inspector's category bar. */
export const CategoryScoreSchema = z.object({
  category: z.string(),
  confidence: z.number(),
});

/** Slate fields parsed from a description without running the classifier. */
export const SlateFieldsSchema = z.object({
  description: z.string(),
  fx_name: z.string(),
  keywords: z.array(z.string()),
  creator_id: z.string().nullable().optional(),
  source_id: z.string().nullable().optional(),
  user_category: z.string().nullable().optional(),
  vendor_category: z.string().nullable().optional(),
  user_data: z.string().nullable().optional(),
});

/** One prediction, as returned by `POST /predict`. */
export const PredictionSchema = z.object({
  text: z.string().optional(),
  description: z.string().optional(),
  cat_id: z.string(),
  category: z.string(),
  subcategory: z.string(),
  cat_short: z.string(),
  category_full: z.string(),
  confidence: z.number(),
  category_confidence: z.number().optional(),
  decision: DecisionSchema,
  alternatives: z.array(AlternativeSchema).optional(),
  categories: z.array(CategoryScoreSchema).optional(),
  fx_name: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  filename: z.string(),
  is_unknown: z.boolean().optional(),
  slate: SlateFieldsSchema.optional(),
});

/** The `/predict` envelope. */
export const PredictResponseSchema = z.object({
  schema_version: z.number().int(),
  predictions: z.array(PredictionSchema),
});

/** The `/parse` envelope. */
export const ParseResponseSchema = z.object({
  schema_version: z.number().int(),
  slates: z.array(SlateFieldsSchema),
});

/** One UCS entry, as returned by the taxonomy routes. */
export const TaxonomyEntrySchema = z.object({
  cat_id: z.string(),
  category: z.string(),
  subcategory: z.string(),
  cat_short: z.string(),
  category_full: z.string(),
  description: z.string().optional(),
});

/** The `/taxonomy/search` envelope. */
export const TaxonomySearchResponseSchema = z.object({
  schema_version: z.number().int(),
  entries: z.array(TaxonomyEntrySchema),
});

/** The `/taxonomy/{cat_id}` envelope. */
export const TaxonomyEntryResponseSchema = z.object({
  schema_version: z.number().int(),
  entry: TaxonomyEntrySchema,
});

/** The `/shutdown` envelope. */
export const ShutdownResponseSchema = z.object({
  schema_version: z.number().int(),
  ok: z.boolean(),
});

/** Model identity, shown in the About panel. */
export const ModelInfoSchema = z.object({
  name: z.string(),
  version: z.string(),
  source: z.string().optional(),
  manifest_verified: z.boolean().optional(),
  manifest_error: z.string().nullable().optional(),
  path: z.string().nullable().optional(),
});

/** Request caps the server advertises. */
export const LimitsSchema = z.object({
  max_texts: z.number().int().optional(),
  max_body_bytes: z.number().int().optional(),
  max_top_k: z.number().int().optional(),
});

/** The `/health` payload. */
export const HealthSchema = z.object({
  schema_version: z.number().int(),
  status: z.literal("ok"),
  warm: z.boolean(),
  ttucs_version: z.string(),
  ucs_version: z.string(),
  model: ModelInfoSchema,
  offline: z.boolean(),
  device: z.string().optional(),
  unknown_threshold: z.number(),
  category_threshold: z.number(),
  top_k: z.number().int().optional(),
  limits: LimitsSchema.optional(),
});

/** A parsed prediction. */
export type Prediction = z.infer<typeof PredictionSchema>;
/** A parsed health payload. */
export type Health = z.infer<typeof HealthSchema>;
/** A parsed taxonomy entry. */
export type TaxonomyEntry = z.infer<typeof TaxonomyEntrySchema>;
/** Parsed slate fields. */
export type SlateFields = z.infer<typeof SlateFieldsSchema>;

/**
 * Whether the server's contract version is one this client implements.
 *
 * The runtime gate is the integer, not the OpenAPI document: a server that speaks a newer
 * version may have changed a field's meaning without changing its name.
 */
export function isSupportedSchemaVersion(version: number): boolean {
  return version === SCHEMA_VERSION;
}
