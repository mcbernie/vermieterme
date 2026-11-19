import type { SelectOption } from "@zweiund40gmbh/ui";
import type { PropertyWithUnits, RentChange, Tenant, TenantWithUnit } from "@/types";

/** Kind of a rent change: base rent or prepayment of operating costs. */
export type RentChangeType = RentChange["type"];

/** Changes of one unit and type, newest effective date first. */
export function rentHistory(rentChanges: readonly RentChange[], unitId: string, type?: RentChangeType): RentChange[] {
  return rentChanges
    .filter((rc) => rc.unitId === unitId && (type === undefined || rc.type === type))
    .sort((a, b) => new Date(b.effectiveDate).getTime() - new Date(a.effectiveDate).getTime());
}

/** Latest change of a unit and type (by effective date), or null when there is none. */
export function latestRentChange(
  rentChanges: readonly RentChange[],
  unitId: string,
  type: RentChangeType,
): RentChange | null {
  return rentHistory(rentChanges, unitId, type)[0] ?? null;
}

/** Address line of the property of a tenant (`Straße, Ort`), or a fallback when unknown. */
export function propertyLabel(tenant: TenantWithUnit): string {
  const property = tenant.unit?.property;
  return property ? `${property.street}, ${property.city}` : "Unbekanntes Objekt";
}

/** Full name of the main tenant including salutation. */
export function tenantName(tenant: Pick<Tenant, "salutation" | "firstName" | "lastName">): string {
  return [tenant.salutation, tenant.firstName, tenant.lastName].filter(Boolean).join(" ");
}

/** Full name of the second tenant, or null when there is none. */
export function secondTenantName(tenant: Tenant): string | null {
  if (!tenant.firstName2 || !tenant.lastName2) return null;
  return [tenant.salutation2, tenant.firstName2, tenant.lastName2].filter(Boolean).join(" ");
}

/** Options of the unit select: unit name and floor, property address as second line. */
export function unitOptions(properties: readonly PropertyWithUnits[]): SelectOption<string>[] {
  return properties.flatMap((property) =>
    (property.units || []).map((unit) => ({
      value: unit.id,
      label: `${unit.name} (${unit.floor})`,
      sub: `${property.street}, ${property.city}`,
    })),
  );
}
