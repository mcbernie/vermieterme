import { describe, expect, it } from "vitest";
import {
  latestRentChange,
  propertyLabel,
  rentHistory,
  secondTenantName,
  tenantName,
  unitOptions,
} from "@/features/tenants/tenant-helpers";
import { PROPERTY, RENT_CHANGES, TENANT } from "./tenant-fixtures";

describe("rent changes", () => {
  it("returns the latest change of a unit and type", () => {
    expect(latestRentChange(RENT_CHANGES, "u1", "rent")?.amount).toBe(650);
    expect(latestRentChange(RENT_CHANGES, "u1", "prepayment")?.amount).toBe(180);
    expect(latestRentChange(RENT_CHANGES, "u3", "rent")).toBeNull();
  });

  it("lists the history of a unit newest first", () => {
    expect(rentHistory(RENT_CHANGES, "u1").map((rc) => rc.id)).toEqual(["r2", "r1", "r3"]);
  });

  it("does not reorder the input", () => {
    const input = [...RENT_CHANGES];
    rentHistory(input, "u1");
    expect(input).toEqual(RENT_CHANGES);
  });
});

describe("names and labels", () => {
  it("builds the tenant names", () => {
    expect(tenantName(TENANT)).toBe("Frau Erika Muster");
    expect(secondTenantName(TENANT)).toBeNull();
    expect(secondTenantName({ ...TENANT, salutation2: "Herr", firstName2: "Max", lastName2: "Muster" })).toBe(
      "Herr Max Muster",
    );
  });

  it("labels the property or falls back", () => {
    expect(propertyLabel(TENANT)).toBe("Musterstraße 1, Beispielstadt");
    expect(propertyLabel({ ...TENANT, unit: { ...TENANT.unit, property: undefined } })).toBe("Unbekanntes Objekt");
  });

  it("offers every unit with the property as second line", () => {
    expect(unitOptions([PROPERTY])).toEqual([
      { value: "u1", label: "Wohnung EG (EG)", sub: "Musterstraße 1, Beispielstadt" },
      { value: "u2", label: "Wohnung OG (OG)", sub: "Musterstraße 1, Beispielstadt" },
    ]);
  });
});
