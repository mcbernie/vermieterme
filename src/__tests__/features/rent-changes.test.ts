import { describe, expect, it } from "vitest";
import { centsToEuros, eurosToCents, groupRentChanges } from "@/features/rent-changes/rent-change-utils";
import type { RentChangeWithUnit } from "@/types";

function change(id: string, date: string, unit = "EG", street = "Weg 1"): RentChangeWithUnit {
  return {
    id,
    unitId: unit,
    type: "rent",
    amount: 500,
    effectiveDate: date,
    reason: null,
    unit: { name: unit, property: { street, city: "Bremen" } },
  } as unknown as RentChangeWithUnit;
}

describe("money conversion", () => {
  it("rounds euros to whole cents", () => {
    expect(eurosToCents(19.99)).toBe(1999);
    expect(eurosToCents(0.29)).toBe(29);
    expect(centsToEuros(1999)).toBe(19.99);
  });
});

describe("groupRentChanges", () => {
  it("groups by property and unit, newest first", () => {
    const groups = groupRentChanges([
      change("a", "2024-01-01"),
      change("b", "2025-01-01"),
      change("c", "2024-06-01", "OG", "Weg 2"),
    ]);
    expect(Object.keys(groups)).toEqual(["Weg 1, Bremen", "Weg 2, Bremen"]);
    expect(groups["Weg 1, Bremen"].EG.map((c) => c.id)).toEqual(["b", "a"]);
  });

  it("uses fallbacks without unit data", () => {
    const orphan = { id: "x", effectiveDate: "2024-01-01" } as unknown as RentChangeWithUnit;
    expect(groupRentChanges([orphan])).toEqual({ "Unbekanntes Objekt": { "Unbekannte Wohnung": [orphan] } });
  });
});
