import type { PropertyWithUnits, RentChange, TenantWithUnit } from "@/types";

/** Obvious sample data for the tenant tests (no real persons). */
export const PROPERTY: PropertyWithUnits = {
  id: "p1",
  street: "Musterstraße 1",
  zip: "12345",
  city: "Beispielstadt",
  totalShares: 1000,
  units: [
    { id: "u1", propertyId: "p1", name: "Wohnung EG", floor: "EG", shares: 500, tenants: [] },
    { id: "u2", propertyId: "p1", name: "Wohnung OG", floor: "OG", shares: 500, tenants: [] },
  ],
};

/** A tenant of unit u1 with index lease. */
export const TENANT: TenantWithUnit = {
  id: "t1",
  unitId: "u1",
  salutation: "Frau",
  firstName: "Erika",
  lastName: "Muster",
  salutation2: null,
  firstName2: null,
  lastName2: null,
  phone: null,
  email: "erika@example.org",
  bankName: null,
  iban: null,
  accountHolder: null,
  moveInDate: "2022-04-01T00:00:00.000Z",
  moveOutDate: null,
  leaseType: "index",
  indexBaseYear: 2020,
  indexReferenceValue: 105.4,
  indexReferenceDate: "2023-01-01T00:00:00.000Z",
  indexMinMonths: 12,
  unit: { id: "u1", propertyId: "p1", name: "Wohnung EG", floor: "EG", shares: 500, property: PROPERTY },
};

/** Rent changes of unit u1 (two rents, one prepayment) and one of another unit. */
export const RENT_CHANGES: RentChange[] = [
  { id: "r1", unitId: "u1", type: "rent", amount: 600, effectiveDate: "2022-04-01T00:00:00.000Z", reason: null },
  { id: "r2", unitId: "u1", type: "rent", amount: 650, effectiveDate: "2024-04-01T00:00:00.000Z", reason: "VPI" },
  { id: "r3", unitId: "u1", type: "prepayment", amount: 180, effectiveDate: "2022-04-01T00:00:00.000Z", reason: null },
  { id: "r4", unitId: "u2", type: "rent", amount: 900, effectiveDate: "2025-01-01T00:00:00.000Z", reason: null },
];
