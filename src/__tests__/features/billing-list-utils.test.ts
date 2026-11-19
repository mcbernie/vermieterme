import { describe, expect, it } from "vitest";
import {
  copySourceOptions,
  groupByProperty,
  previousYearSuggestions,
  shiftYear,
} from "@/features/billing/billing-list-utils";
import type { BillingPeriodWithProperty, Property } from "@/types";

const prop = (id: string): Property => ({ id, street: `S${id}`, zip: "1", city: "C", totalShares: 100 });
const period = (id: string, propertyId: string, year: number): BillingPeriodWithProperty => ({
  id,
  propertyId,
  startDate: `${year}-01-01T00:00:00.000Z`,
  endDate: `${year}-12-31T00:00:00.000Z`,
  billingDate: null,
  sentDate: null,
  paidDate: null,
  copiedFromId: null,
  property: prop(propertyId),
});

describe("billing-list-utils", () => {
  it("groups periods by property in order of first appearance", () => {
    const groups = groupByProperty([period("1", "b", 2024), period("2", "a", 2024), period("3", "b", 2023)]);
    expect(groups.map((g) => g.propertyId)).toEqual(["b", "a"]);
    expect(groups[0]?.periods.map((p) => p.id)).toEqual(["1", "3"]);
  });

  it("suggests the previous year only for properties without it, copying the latest period", () => {
    const periods = [period("1", "a", 2023), period("2", "a", 2024), period("3", "b", 2025), period("4", "b", 2023)];
    const suggestions = previousYearSuggestions([prop("a"), prop("b"), prop("c")], periods, 2026);
    expect(suggestions).toHaveLength(1);
    expect(suggestions[0]?.property.id).toBe("a");
    expect(suggestions[0]?.previousYear).toBe(2025);
    expect(suggestions[0]?.sourcePeriod.id).toBe("2");
  });

  it("offers copy sources only for the chosen property", () => {
    const periods = [period("1", "a", 2024), period("2", "b", 2024)];
    expect(copySourceOptions(periods, "")).toEqual([]);
    expect(copySourceOptions(periods, "a").map((o) => o.value)).toEqual(["1"]);
  });

  it("shifts a date by one year", () => {
    expect(shiftYear("2024-01-01")).toBe("2025-01-01");
    expect(shiftYear("2024-12-31")).toBe("2025-12-31");
  });
});
