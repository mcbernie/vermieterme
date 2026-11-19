import { calculateMEAAmount, getMonthsInPeriod, suggestNextPrepayment } from "@/lib/billing";
import type { BillingPeriodDetail, CostCategory, Tenant, UnitWithTenants } from "@/types";

/**
 * Editable values of one cost position. Amounts are euros as stored by the API (`null` = empty), so
 * values loaded from the server keep their full precision until the user edits them.
 */
export interface CostFormValue {
  totalAmount: number | null;
  unitAmount: number | null;
  enabled: boolean;
  distributionKeyOverride: string | null;
}

/** Editable cost values per cost category id. */
export type CostValues = Record<string, CostFormValue>;
/** Editable monthly prepayment (euros, `null` = empty) per unit id. */
export type PrepaymentValues = Record<string, number | null>;

/** Value of a cost category without an entry yet. */
export const DEFAULT_COST_VALUE: CostFormValue = {
  totalAmount: null,
  unitAmount: null,
  enabled: true,
  distributionKeyOverride: null,
};

/** Cost and prepayment form values as loaded from the server. */
export function seedValues(bp: BillingPeriodDetail): { costs: CostValues; prepayments: PrepaymentValues } {
  const costs: CostValues = {};
  for (const cost of bp.costs) {
    costs[cost.costCategoryId] = {
      totalAmount: cost.totalAmount,
      unitAmount: cost.unitAmount ?? null,
      enabled: cost.enabled !== false,
      distributionKeyOverride: cost.distributionKeyOverride ?? null,
    };
  }
  const prepayments: PrepaymentValues = {};
  for (const pp of bp.prepayments) prepayments[pp.unitId] = pp.monthlyAmount;
  return { costs, prepayments };
}

/**
 * Combines freshly loaded cost values with the ones on the page after a reload. Amounts typed but
 * not saved yet stay (the server value would overwrite them); flags such as shown or distribution
 * key come from the server, which is the source of truth for them.
 */
export function mergeCosts(server: CostValues, local: CostValues): CostValues {
  const merged: CostValues = { ...server };
  for (const [id, mine] of Object.entries(local)) {
    const theirs = server[id];
    merged[id] = theirs ? { ...theirs, totalAmount: mine.totalAmount, unitAmount: mine.unitAmount } : mine;
  }
  return merged;
}

/** Like {@link mergeCosts} for prepayments: amounts on the page win over the loaded ones. */
export function mergePrepayments(server: PrepaymentValues, local: PrepaymentValues): PrepaymentValues {
  return { ...server, ...local };
}

/** Distribution key in effect for a category in this period (override first). */
export function effectiveKey(value: CostFormValue, category: CostCategory): string {
  return value.distributionKeyOverride ?? category.distributionKey;
}

/**
 * Costs of one unit: MEA positions by shares, all other keys by the entered unit amount. Positions
 * without a total amount and disabled positions are skipped.
 */
export function unitCosts(
  unit: UnitWithTenants,
  categories: CostCategory[],
  costValues: CostValues,
  totalShares: number,
): number {
  let total = 0;
  for (const category of categories) {
    const value = costValues[category.id];
    if (!value || value.totalAmount === null) continue;
    if (value.enabled === false) continue;
    if (effectiveKey(value, category) === "MEA") {
      total += calculateMEAAmount(value.totalAmount, unit.shares, totalShares);
    } else {
      total += value.unitAmount ?? 0;
    }
  }
  return total;
}

/** Monthly prepayment of a unit in euros (0 when empty). */
export function monthlyPrepayment(prepayments: PrepaymentValues, unitId: string): number {
  return prepayments[unitId] ?? 0;
}

/** Prepayment of a unit over the whole period. */
export function unitPrepayment(prepayments: PrepaymentValues, unitId: string, bp: BillingPeriodDetail): number {
  return monthlyPrepayment(prepayments, unitId) * getMonthsInPeriod(bp.startDate, bp.endDate);
}

/** Everything the summary and the recommendations need for one unit. */
export interface UnitResult {
  unit: UnitWithTenants;
  tenant: Tenant | undefined;
  monthly: number;
  costs: number;
  prepayment: number;
  /** Costs minus prepayments: positive = back payment, negative = refund. */
  result: number;
}

/** Current tenant of a unit (no move-out date). */
export function currentTenant(unit: UnitWithTenants): Tenant | undefined {
  return unit.tenants?.find((t) => !t.moveOutDate);
}

/** Display name of the current tenant, "Kein Mieter" without one. */
export function tenantName(tenant: Tenant | undefined): string {
  return tenant ? `${tenant.firstName} ${tenant.lastName}` : "Kein Mieter";
}

/** Results of all units of the period. */
export function unitResults(
  bp: BillingPeriodDetail,
  categories: CostCategory[],
  costValues: CostValues,
  prepayments: PrepaymentValues,
): UnitResult[] {
  return (bp.property.units || []).map((unit) => {
    const costs = unitCosts(unit, categories, costValues, bp.property.totalShares);
    const prepayment = unitPrepayment(prepayments, unit.id, bp);
    return {
      unit,
      tenant: currentTenant(unit),
      monthly: monthlyPrepayment(prepayments, unit.id),
      costs,
      prepayment,
      result: costs - prepayment,
    };
  });
}

/** Sums of costs and prepayments over all units. */
export function totalsOf(results: UnitResult[]): { costs: number; prepayments: number } {
  return {
    costs: results.reduce((sum, r) => sum + r.costs, 0),
    prepayments: results.reduce((sum, r) => sum + r.prepayment, 0),
  };
}

/** A unit whose period closes with a back payment, with the suggested new monthly prepayment. */
export interface NkRecommendation extends UnitResult {
  suggested: number;
}

/** Units for which a higher monthly prepayment is recommended. */
export function nkRecommendations(results: UnitResult[]): NkRecommendation[] {
  const out: NkRecommendation[] = [];
  for (const r of results) {
    const suggested = suggestNextPrepayment(r.monthly, r.result);
    if (suggested != null) out.push({ ...r, suggested });
  }
  return out;
}

/** Review state of the period: disabled cost positions do not count. */
export function reviewProgress(bp: BillingPeriodDetail) {
  const activeCosts = bp.costs.filter((c) => c.enabled !== false);
  const total = activeCosts.length + bp.prepayments.length;
  const reviewed =
    activeCosts.filter((c) => c.reviewed).length + bp.prepayments.filter((p) => p.reviewed).length;
  return { total, reviewed, allReviewed: total > 0 && reviewed === total };
}

/** Whether the cost position of a category exists and is not yet reviewed. */
export function isCostUnreviewed(bp: BillingPeriodDetail, categoryId: string): boolean {
  return bp.costs.find((c) => c.costCategoryId === categoryId)?.reviewed === false;
}

/** Whether the prepayment of a unit exists and is not yet reviewed. */
export function isPrepaymentUnreviewed(bp: BillingPeriodDetail, unitId: string): boolean {
  return bp.prepayments.find((p) => p.unitId === unitId)?.reviewed === false;
}

/** Request body that stores the amounts of a cost position. */
export function costPayload(categoryId: string, value: CostFormValue) {
  return { costCategoryId: categoryId, totalAmount: value.totalAmount ?? 0, unitAmount: value.unitAmount ?? null };
}
