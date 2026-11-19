import type { RentChangeWithUnit } from "@/types";

/** Rent changes grouped by property label, then by unit name. */
export type RentChangeGroups = Record<string, Record<string, RentChangeWithUnit[]>>;

/** Labels of the two rent change types. */
export const TYPE_LABELS = { rent: "Kaltmiete", prepayment: "Vorauszahlung NK" } as const;

/** Select options for the rent change type. */
export const TYPE_OPTIONS = [
  { value: "rent", label: TYPE_LABELS.rent },
  { value: "prepayment", label: TYPE_LABELS.prepayment },
] as const;

/**
 * Converts an amount in euros to whole cents (for `MoneyInput`).
 * @param euros Amount in euros.
 */
export function eurosToCents(euros: number): number {
  return Math.round(euros * 100);
}

/**
 * Converts whole cents to euros (for the API).
 * @param cents Amount in cents.
 */
export function centsToEuros(cents: number): number {
  return cents / 100;
}

/**
 * Groups rent changes by property, then by unit; each unit's list is sorted by effective date, newest first.
 * @param changes Rent changes with their unit (and property).
 */
export function groupRentChanges(changes: RentChangeWithUnit[]): RentChangeGroups {
  const grouped: RentChangeGroups = {};
  for (const change of changes) {
    const propKey = change.unit?.property
      ? `${change.unit.property.street}, ${change.unit.property.city}`
      : "Unbekanntes Objekt";
    const unitKey = change.unit?.name || "Unbekannte Wohnung";
    ((grouped[propKey] ??= {})[unitKey] ??= []).push(change);
  }
  for (const units of Object.values(grouped)) {
    for (const list of Object.values(units)) {
      list.sort((a, b) => new Date(b.effectiveDate).getTime() - new Date(a.effectiveDate).getTime());
    }
  }
  return grouped;
}
