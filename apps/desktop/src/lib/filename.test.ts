import { describe, expect, it } from "vitest";

import { NONE, buildFilename, parseFilename, validateFilename } from "./filename.ts";

describe("buildFilename", () => {
  it("joins the four fields with underscores", () => {
    expect(
      buildFilename({ catId: "DOORCreak", fxName: "Door Creak", userCategory: NONE, vendorCategory: NONE }),
    ).toBe("DOORCreak_Door Creak_NONE_NONE");
  });

  it("keeps an underscore inside the effect name", () => {
    expect(
      buildFilename({ catId: "DOORCreak", fxName: "Door_Creak", userCategory: NONE, vendorCategory: NONE }),
    ).toBe("DOORCreak_Door_Creak_NONE_NONE");
  });
});

describe("parseFilename", () => {
  it("splits a plain name", () => {
    expect(parseFilename("DOORCreak_Door Creak_NONE_NONE")).toEqual({
      catId: "DOORCreak",
      fxName: "Door Creak",
      userCategory: NONE,
      vendorCategory: NONE,
    });
  });

  it("keeps an underscore inside the effect name", () => {
    expect(parseFilename("DOORCreak_Door_Creak_NONE_NONE")?.fxName).toBe("Door_Creak");
  });

  it("returns null with fewer than four fields", () => {
    expect(parseFilename("DOORCreak_Door Creak_NONE")).toBeNull();
  });

  it("returns null for an empty string", () => {
    expect(parseFilename("")).toBeNull();
  });

  it("round-trips through buildFilename", () => {
    const parts = { catId: "VEHHorn", fxName: "Car Horn Honk", userCategory: "USER", vendorCategory: "VEND" };
    expect(parseFilename(buildFilename(parts))).toEqual(parts);
  });
});

describe("validateFilename", () => {
  it("accepts a well-formed name", () => {
    expect(validateFilename("DOORCreak_Door Creak_NONE_NONE")).toEqual({ ok: true });
  });

  it("rejects an empty name", () => {
    const verdict = validateFilename("");
    expect(verdict.ok).toBe(false);
    expect(verdict).toMatchObject({ reason: "A file name cannot be empty." });
  });

  it("rejects a name with surrounding spaces", () => {
    const verdict = validateFilename(" DOORCreak_Door Creak_NONE_NONE");
    expect(verdict).toMatchObject({ reason: "A file name cannot start or end with a space." });
  });

  it.each(["\\", "/", ":", "*", "?", '"', "<", ">", "|"])(
    "rejects the reserved character %s",
    (character) => {
      const verdict = validateFilename(`DOORCreak_Door${character}Creak_NONE_NONE`);
      expect(verdict.ok).toBe(false);
    },
  );

  it("rejects too few fields", () => {
    expect(validateFilename("DOORCreak_Door Creak_NONE")).toMatchObject({
      reason: "Expected CATID_FxName_UserCategory_VendorCategory.",
    });
  });

  it("rejects a missing CatID", () => {
    expect(validateFilename("_Door Creak_NONE_NONE")).toMatchObject({ reason: "The CatID is missing." });
  });

  it("rejects a missing effect name", () => {
    expect(validateFilename("DOORCreak__NONE_NONE")).toMatchObject({
      reason: "The effect name is missing.",
    });
  });

  it("rejects an empty user category", () => {
    expect(validateFilename("DOORCreak_Door Creak__NONE")).toMatchObject({
      reason: "Use NONE for an empty user or vendor category.",
    });
  });

  it("rejects an empty vendor category", () => {
    expect(validateFilename("DOORCreak_Door Creak_NONE_")).toMatchObject({
      reason: "Use NONE for an empty user or vendor category.",
    });
  });

  it("rejects a CatID outside the taxonomy", () => {
    const verdict = validateFilename("NOPE_Door Creak_NONE_NONE", new Set(["DOORCreak"]));
    expect(verdict).toMatchObject({ reason: '"NOPE" is not a CatID in this taxonomy.' });
  });

  it("accepts a CatID inside the taxonomy", () => {
    expect(validateFilename("DOORCreak_Door Creak_NONE_NONE", new Set(["DOORCreak"]))).toEqual({ ok: true });
  });

  it("skips the taxonomy check when no taxonomy is loaded", () => {
    expect(validateFilename("NOPE_Door Creak_NONE_NONE", new Set())).toEqual({ ok: true });
  });
});
