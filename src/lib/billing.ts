import type {
  BillingPeriodWithProperty,
  Cost,
  CostCategory,
  Prepayment,
} from "@/types";

// Per-period override (Cost.distributionKeyOverride) takes precedence over
// the category default. Used wherever calculation or display needs the
// effective key for a given billing period.
export function effectiveDistributionKey(
  cost: Pick<Cost, "distributionKeyOverride">,
  category: Pick<CostCategory, "distributionKey">
): string {
  return cost.distributionKeyOverride ?? category.distributionKey;
}

export function getBillingStatus(bp: BillingPeriodWithProperty) {
  if (bp.paidDate) {
    return { label: "Bezahlt", className: "bg-green-100 text-green-700" };
  }
  if (bp.sentDate) {
    return { label: "Versendet", className: "bg-blue-100 text-blue-700" };
  }
  if (bp.billingDate) {
    return { label: "Abgeschlossen", className: "bg-emerald-100 text-emerald-700" };
  }
  if (bp._count && bp._count.costs > 0) {
    return { label: "In Bearbeitung", className: "bg-amber-100 text-amber-700" };
  }
  return { label: "Offen", className: "bg-zinc-100 text-zinc-600" };
}

export function getUnreviewedCount(
  costs: Cost[],
  prepayments: Prepayment[]
): number {
  // Disabled positions are skipped in calculations, so they should not
  // block the review progress either.
  const unreviewedCosts = costs.filter((c) => c.enabled !== false && !c.reviewed)
    .length;
  const unreviewedPrepayments = prepayments.filter((p) => !p.reviewed).length;
  return unreviewedCosts + unreviewedPrepayments;
}

export function calculateBillingTotals(
  costs: { totalAmount: number; unitAmount: number | null }[],
  prepayments: { monthlyAmount: number }[],
  startDate: string,
  endDate: string
) {
  const totalCosts = costs.reduce((sum, c) => sum + c.totalAmount, 0);
  const totalUnitCosts = costs.reduce(
    (sum, c) => sum + (c.unitAmount ?? 0),
    0
  );
  const months = getMonthsInPeriod(startDate, endDate);
  const totalPrepayment = prepayments.reduce(
    (sum, p) => sum + p.monthlyAmount * months,
    0
  );
  const difference = totalPrepayment - totalUnitCosts;
  return { totalCosts, totalUnitCosts, totalPrepayment, difference };
}

export function calculateMEAAmount(
  totalAmount: number,
  unitShares: number,
  propertyTotalShares: number
): number {
  if (propertyTotalShares === 0) return 0;
  return totalAmount * (unitShares / propertyTotalShares);
}

export function calculateAllocationAmount(
  totalAmount: number,
  unitValue: number,
  totalValue: number
): number {
  if (totalValue === 0) return 0;
  return totalAmount * (unitValue / totalValue);
}

export function getMonthsInPeriod(startDate: string, endDate: string): number {
  const start = new Date(startDate);
  const end = new Date(endDate);
  const months =
    (end.getFullYear() - start.getFullYear()) * 12 +
    (end.getMonth() - start.getMonth()) +
    1;
  return Math.max(1, months);
}

export function getDaysInPeriod(startDate: string, endDate: string): number {
  const start = new Date(startDate);
  const end = new Date(endDate);
  const diffTime = Math.abs(end.getTime() - start.getTime());
  return Math.ceil(diffTime / (1000 * 60 * 60 * 24)) + 1;
}

// Recommended new monthly NK prepayment after a billing period that closes
// with a shortfall (Nachzahlung): keep the current monthly amount and add the
// shortfall spread over twelve months, rounded up to the next 5-EUR step. If
// the period closes with a refund, no change is suggested (returns null).
export function suggestNextPrepayment(
  currentMonthly: number,
  resultAmount: number
): number | null {
  if (resultAmount <= 0) return null;
  const raw = currentMonthly + resultAmount / 12;
  return Math.ceil(raw / 5) * 5;
}

// Default effective date for a new prepayment that follows a closed billing
// period: the day after the period ends.
export function dayAfter(dateStr: string): string {
  const d = new Date(dateStr);
  d.setDate(d.getDate() + 1);
  return d.toISOString().split("T")[0];
}
