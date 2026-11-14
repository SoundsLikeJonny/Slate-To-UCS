/**
 * Building and validating UCS file names in the inspector.
 *
 * The format is `<cat_id>_<fx_name>_<user_category>_<vendor_category>`, the same one
 * `slate-ucs` builds and parses. Validation splits from the right, so an `fx_name` that
 * contains an underscore still round-trips.
 */

/** The placeholder UCS uses for an empty user or vendor category. */
export const NONE = "NONE";

/** The outcome of validating a file name. */
export type FilenameVerdict = { readonly ok: true } | { readonly ok: false; readonly reason: string };

/** The parts of a file name. */
export interface FilenameParts {
  /** The CatID, for example `DOORCreak`. */
  readonly catId: string;
  /** The human-readable effect name. */
  readonly fxName: string;
  /** The user category, usually `NONE`. */
  readonly userCategory: string;
  /** The vendor category, usually `NONE`. */
  readonly vendorCategory: string;
}

/** Join the parts into a file name. */
export function buildFilename(parts: FilenameParts): string {
  return [parts.catId, parts.fxName, parts.userCategory, parts.vendorCategory].join("_");
}

/**
 * Split a file name back into its parts.
 *
 * Returns `null` when there are fewer than four fields, because the split is then
 * ambiguous: any guess would silently move text between fields.
 */
export function parseFilename(value: string): FilenameParts | null {
  const fields = value.split("_");
  if (fields.length < 4) {
    return null;
  }
  const [catId = "", ...rest] = fields;
  const vendorCategory = rest.pop() as string;
  const userCategory = rest.pop() as string;
  const fxName = rest.join("_");
  return { catId, fxName, userCategory, vendorCategory };
}

/**
 * Validate a file name as the user types it.
 *
 * `knownCatIds` is the taxonomy the engine reported; when it is empty the CatID check is
 * skipped rather than rejecting every name, because a queue can be reviewed before the
 * taxonomy has loaded.
 */
export function validateFilename(
  value: string,
  knownCatIds: ReadonlySet<string> = new Set(),
): FilenameVerdict {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return { ok: false, reason: "A file name cannot be empty." };
  }
  if (trimmed !== value) {
    return { ok: false, reason: "A file name cannot start or end with a space." };
  }
  if (/[\\/:*?"<>|]/.test(trimmed)) {
    return { ok: false, reason: 'A file name cannot contain \\ / : * ? " < > |.' };
  }
  const parts = parseFilename(trimmed);
  if (parts === null) {
    return {
      ok: false,
      reason: "Expected CATID_FxName_UserCategory_VendorCategory.",
    };
  }
  if (parts.catId.length === 0) {
    return { ok: false, reason: "The CatID is missing." };
  }
  if (parts.fxName.length === 0) {
    return { ok: false, reason: "The effect name is missing." };
  }
  if (parts.userCategory.length === 0 || parts.vendorCategory.length === 0) {
    return { ok: false, reason: "Use NONE for an empty user or vendor category." };
  }
  if (knownCatIds.size > 0 && !knownCatIds.has(parts.catId)) {
    return { ok: false, reason: `"${parts.catId}" is not a CatID in this taxonomy.` };
  }
  return { ok: true };
}
