import { describe, expect, it } from "vitest";
import { getLatestRent } from "@/features/properties/latest-rent";
import type { RentChange } from "@/types";

const rc = (id: string, unitId: string, type: RentChange["type"], amount: number, effectiveDate: string): RentChange => ({
  id,
  unitId,
  type,
  amount,
  effectiveDate,
  reason: null,
});

describe("getLatestRent", () => {
  const changes = [
    rc("1", "u1", "rent", 500, "2023-01-01"),
    rc("2", "u1", "rent", 550, "2024-01-01"),
    rc("3", "u1", "prepayment", 100, "2023-06-01"),
    rc("4", "u2", "rent", 900, "2025-01-01"),
  ];

  it("returns the most recent rent and prepayment of the unit", () => {
    expect(getLatestRent(changes, "u1")).toEqual({ rent: 550, prepayment: 100 });
  });

  it("returns null for missing types and unknown units", () => {
    expect(getLatestRent(changes, "u2")).toEqual({ rent: 900, prepayment: null });
    expect(getLatestRent(changes, "x")).toEqual({ rent: null, prepayment: null });
  });
});
