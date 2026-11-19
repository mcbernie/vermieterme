import type { RentChange } from "@/types";

/** Latest rent and prepayment amounts of a unit (null when none is recorded). */
export interface LatestRent {
  rent: number | null;
  prepayment: number | null;
}

/**
 * Picks the most recent rent and prepayment change of a unit by effective date.
 * @param rentChanges all known rent changes
 * @param unitId the unit to look at
 */
export function getLatestRent(rentChanges: RentChange[], unitId: string): LatestRent {
  const latest = (type: RentChange["type"]): number | null => {
    const sorted = rentChanges
      .filter((rc) => rc.unitId === unitId && rc.type === type)
      .sort((a, b) => new Date(b.effectiveDate).getTime() - new Date(a.effectiveDate).getTime());
    return sorted[0]?.amount ?? null;
  };
  return { rent: latest("rent"), prepayment: latest("prepayment") };
}
