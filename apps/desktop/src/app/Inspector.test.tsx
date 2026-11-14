import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { Inspector, OVERRIDE_LIMIT, searchTaxonomy } from "./Inspector.tsx";
import type { ReviewRow } from "../lib/queue.ts";
import type { Prediction, TaxonomyEntry } from "../lib/schema.ts";

function prediction(overrides: Partial<Prediction> = {}): Prediction {
  return {
    cat_id: "DOORCreak",
    category: "DOORS",
    subcategory: "Creak",
    cat_short: "DOORCreak",
    category_full: "Doors",
    confidence: 0.5,
    decision: "confident",
    filename: "DOORCreak_Door Creak_NONE_NONE",
    keywords: ["Door", "Creak"],
    alternatives: [
      { cat_id: "DOORSlid", category: "DOORS", subcategory: "Slide", cat_short: "DOORSlid", confidence: 0.3 },
    ],
    ...overrides,
  };
}

function row(overrides: Partial<ReviewRow> = {}): ReviewRow {
  return {
    id: "/tmp/a.wav",
    fileName: "a.wav",
    transcript: "door creak",
    prediction: prediction(),
    override: null,
    status: "ready",
    error: null,
    ...overrides,
  };
}

function entry(catId: string, category = "DOORS", subcategory = "Creak"): TaxonomyEntry {
  return { cat_id: catId, category, subcategory, cat_short: catId, category_full: category };
}

const TAXONOMY: TaxonomyEntry[] = [
  entry("DOORCreak"),
  entry("DOORSlid", "DOORS", "Slide"),
  entry("VEHHorn", "VEHICLES", "Horn"),
  entry("WNDWMisc", "WINDOWS", "Misc"),
];

describe("searchTaxonomy", () => {
  it("returns the first entries for an empty query", () => {
    expect(searchTaxonomy(TAXONOMY, "").map((item) => item.cat_id)).toEqual([
      "DOORCreak",
      "DOORSlid",
      "VEHHorn",
      "WNDWMisc",
    ]);
  });

  it("ranks a CatID prefix first", () => {
    expect(searchTaxonomy(TAXONOMY, "DOOR")[0]?.cat_id).toBe("DOORCreak");
  });

  it("matches a CatID substring", () => {
    expect(searchTaxonomy(TAXONOMY, "Slid").map((item) => item.cat_id)).toEqual(["DOORSlid"]);
  });

  it("matches a subcategory", () => {
    expect(searchTaxonomy(TAXONOMY, "horn").map((item) => item.cat_id)).toEqual(["VEHHorn"]);
  });

  it("matches a category", () => {
    expect(searchTaxonomy(TAXONOMY, "windows").map((item) => item.cat_id)).toEqual(["WNDWMisc"]);
  });

  it("is case-insensitive", () => {
    expect(searchTaxonomy(TAXONOMY, "doorcreak").map((item) => item.cat_id)).toEqual(["DOORCreak"]);
  });

  it("trims the query", () => {
    expect(searchTaxonomy(TAXONOMY, "  horn  ").map((item) => item.cat_id)).toEqual(["VEHHorn"]);
  });

  it("returns nothing for an unmatched query", () => {
    expect(searchTaxonomy(TAXONOMY, "zzz")).toEqual([]);
  });

  it("limits the result", () => {
    expect(searchTaxonomy(TAXONOMY, "", 2)).toHaveLength(2);
  });

  it("defaults to the override limit", () => {
    const many = Array.from({ length: 20 }, (_, index) => entry(`CAT${index}`));
    expect(searchTaxonomy(many, "")).toHaveLength(OVERRIDE_LIMIT);
  });

  it("breaks ties on CatID", () => {
    const tied = [entry("ZZZ", "SAME", "SAME"), entry("AAA", "SAME", "SAME")];
    expect(searchTaxonomy(tied, "same").map((item) => item.cat_id)).toEqual(["AAA", "ZZZ"]);
  });

  it("rejects a zero limit", () => {
    expect(() => searchTaxonomy(TAXONOMY, "", 0)).toThrow(RangeError);
  });

  it("rejects a fractional limit", () => {
    expect(() => searchTaxonomy(TAXONOMY, "", 1.5)).toThrow(RangeError);
  });
});

describe("Inspector", () => {
  it("shows a placeholder with no row", () => {
    render(
      <Inspector row={null} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    expect(screen.getByText("Select a row to inspect its prediction.")).toBeInTheDocument();
  });

  it("shows the file name and transcript", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    expect(screen.getByRole("heading", { name: "a.wav" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("door creak")).toBeInTheDocument();
  });

  it("shows the decision and confidence", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    expect(screen.getByText("confident")).toBeInTheDocument();
    expect(screen.getByText("50%")).toBeInTheDocument();
  });

  it("lists the alternatives", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const alternatives = screen.getByRole("list", { name: "" }) ?? null;
    expect(alternatives).not.toBeNull();
    expect(screen.getAllByText("DOORSlid").length).toBeGreaterThan(0);
    expect(screen.getByText("30%")).toBeInTheDocument();
  });

  it("lists the keywords", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    expect(screen.getByText("Door, Creak")).toBeInTheDocument();
  });

  it("shows a dash when there are no keywords", () => {
    render(
      <Inspector
        row={row({ prediction: prediction({ keywords: [] }) })}
        taxonomy={TAXONOMY}
        onOverride={vi.fn()}
        onClearOverride={vi.fn()}
        onFilenameChange={vi.fn()}
      />,
    );
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("reports an invalid file name", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByDisplayValue("DOORCreak_Door Creak_NONE_NONE");
    fireEvent.change(input, { target: { value: "broken" } });
    expect(screen.getByRole("alert")).toHaveTextContent("Expected CATID_FxName_UserCategory_VendorCategory.");
  });

  it("marks the input invalid while the name is invalid", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByDisplayValue("DOORCreak_Door Creak_NONE_NONE");
    fireEvent.change(input, { target: { value: "broken" } });
    expect(input).toHaveAttribute("aria-invalid", "true");
  });

  it("reports a CatID outside the taxonomy", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByDisplayValue("DOORCreak_Door Creak_NONE_NONE");
    fireEvent.change(input, { target: { value: "NOPE_Door Creak_NONE_NONE" } });
    expect(screen.getByRole("alert")).toHaveTextContent('"NOPE" is not a CatID in this taxonomy.');
  });

  it("calls onFilenameChange as the user types", () => {
    const onFilenameChange = vi.fn();
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={onFilenameChange} />,
    );
    fireEvent.change(screen.getByDisplayValue("DOORCreak_Door Creak_NONE_NONE"), {
      target: { value: "DOORCreak_Door Creak_NONE_USER" },
    });
    expect(onFilenameChange).toHaveBeenCalledWith("DOORCreak_Door Creak_NONE_USER");
  });

  it("filters the override list as the user types", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "horn" } });
    expect(screen.getByRole("option", { name: /VEHHorn/ })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /WNDWMisc/ })).not.toBeInTheDocument();
  });

  it("chooses the active candidate on Enter", () => {
    const onOverride = vi.fn();
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={onOverride} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "DOOR" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onOverride).toHaveBeenCalledWith(expect.objectContaining({ cat_id: "DOORCreak" }));
  });

  it("moves the active candidate with the arrow keys", () => {
    const onOverride = vi.fn();
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={onOverride} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "DOOR" } });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onOverride).toHaveBeenCalledWith(expect.objectContaining({ cat_id: "DOORSlid" }));
  });

  it("does not move above the first candidate", () => {
    const onOverride = vi.fn();
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={onOverride} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "DOOR" } });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onOverride).toHaveBeenCalledWith(expect.objectContaining({ cat_id: "DOORCreak" }));
  });

  it("clears the query on Escape", () => {
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "horn" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(input).toHaveValue("");
  });

  it("chooses a candidate on mouse down", () => {
    const onOverride = vi.fn();
    render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={onOverride} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    fireEvent.mouseDown(screen.getByRole("option", { name: /VEHHorn/ }));
    expect(onOverride).toHaveBeenCalledWith(expect.objectContaining({ cat_id: "VEHHorn" }));
  });

  it("shows the clear button only when overridden", () => {
    const { rerender } = render(
      <Inspector row={row()} taxonomy={TAXONOMY} onOverride={vi.fn()} onClearOverride={vi.fn()} onFilenameChange={vi.fn()} />,
    );
    expect(screen.queryByRole("button", { name: /Clear override/ })).not.toBeInTheDocument();

    rerender(
      <Inspector
        row={row({ override: "DOORSlid" })}
        taxonomy={TAXONOMY}
        onOverride={vi.fn()}
        onClearOverride={vi.fn()}
        onFilenameChange={vi.fn()}
      />,
    );
    expect(screen.getByRole("button", { name: /Clear override/ })).toBeInTheDocument();
  });

  it("calls onClearOverride", () => {
    const onClearOverride = vi.fn();
    render(
      <Inspector
        row={row({ override: "DOORSlid" })}
        taxonomy={TAXONOMY}
        onOverride={vi.fn()}
        onClearOverride={onClearOverride}
        onFilenameChange={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Clear override/ }));
    expect(onClearOverride).toHaveBeenCalledOnce();
  });

  it("shows the effective CatID", () => {
    render(
      <Inspector
        row={row({ override: "DOORSlid" })}
        taxonomy={TAXONOMY}
        onOverride={vi.fn()}
        onClearOverride={vi.fn()}
        onFilenameChange={vi.fn()}
      />,
    );
    expect(screen.getByText("Current CatID: DOORSlid")).toBeInTheDocument();
  });
});
