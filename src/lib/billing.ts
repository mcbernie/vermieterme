import type { BillingPeriodWithProperty } from "@/types";

export function getBillingStatus(bp: BillingPeriodWithProperty) {
  if (bp.billingDate) {
    return { label: "Abgeschlossen", className: "bg-green-100 text-green-700" };
  }
  if (bp._count && bp._count.costs > 0) {
    return {
      label: "In Bearbeitung",
      className: "bg-amber-100 text-amber-700",
    };
  }
  return { label: "Offen", className: "bg-zinc-100 text-zinc-600" };
}

export function calculateMEAAmount(
  totalAmount: number,
  unitShares: number,
  propertyTotalShares: number
): number {
  if (propertyTotalShares === 0) return 0;
  return totalAmount * (unitShares / propertyTotalShares);
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
