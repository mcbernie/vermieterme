import { calculateBillingTotals } from "@/lib/billing";
import { formatDate } from "@/lib/format";
import type { BillingPeriodWithProperty, Property } from "@/types";

/** Billing periods of one property, in API order. */
export interface PropertyGroup {
  propertyId: string;
  property: BillingPeriodWithProperty["property"];
  periods: BillingPeriodWithProperty[];
}

/** A property without a period for the previous year, plus the latest period to copy from. */
export interface PreviousYearSuggestion {
  property: Property;
  previousYear: number;
  sourcePeriod: BillingPeriodWithProperty;
}

/** Groups billing periods by property, keeping the order of first appearance. */
export function groupByProperty(periods: BillingPeriodWithProperty[]): PropertyGroup[] {
  const groups = new Map<string, PropertyGroup>();
  for (const bp of periods) {
    const group = groups.get(bp.propertyId);
    if (group) group.periods.push(bp);
    else groups.set(bp.propertyId, { propertyId: bp.propertyId, property: bp.property, periods: [bp] });
  }
  return [...groups.values()];
}

/**
 * Properties that still need a billing period for the year before `currentYear`. Only properties
 * with at least one earlier period are suggested, because the new period copies from the latest one.
 */
export function previousYearSuggestions(
  properties: Property[],
  periods: BillingPeriodWithProperty[],
  currentYear: number,
): PreviousYearSuggestion[] {
  const previousYear = currentYear - 1;
  const suggestions: PreviousYearSuggestion[] = [];
  for (const property of properties) {
    const own = periods.filter((bp) => bp.propertyId === property.id);
    const covered = own.some(
      (bp) =>
        new Date(bp.startDate).getFullYear() === previousYear ||
        new Date(bp.endDate).getFullYear() === previousYear,
    );
    if (covered) continue;
    const latest = own.sort((a, b) => new Date(b.startDate).getTime() - new Date(a.startDate).getTime())[0];
    if (latest) suggestions.push({ property, previousYear, sourcePeriod: latest });
  }
  return suggestions;
}

/** Options to copy from: the periods of the chosen property (none without a property). */
export function copySourceOptions(
  periods: BillingPeriodWithProperty[],
  propertyId: string,
): { value: string; label: string }[] {
  if (!propertyId) return [];
  return periods
    .filter((bp) => bp.propertyId === propertyId)
    .map((bp) => ({ value: bp.id, label: `${formatDate(bp.startDate)} – ${formatDate(bp.endDate)}` }));
}

/** The same day one year later as `YYYY-MM-DD` (used to prefill a period copied from the previous year). */
export function shiftYear(dateStr: string): string {
  const d = new Date(dateStr);
  d.setFullYear(d.getFullYear() + 1);
  return d.toISOString().split("T")[0] ?? "";
}

/** Cost totals of a listed period, `null` while it has no costs. */
export function listTotals(bp: BillingPeriodWithProperty) {
  if (!bp.costs || bp.costs.length === 0) return null;
  return calculateBillingTotals(bp.costs, bp.prepayments ?? [], bp.startDate, bp.endDate);
}
