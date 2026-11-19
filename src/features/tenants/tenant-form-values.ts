import { parseDecimal, type DateInputProblem } from "@zweiund40gmbh/ui";
import { dateError } from "@/components/form/date-problem";
import { validateIBAN } from "@/lib/iban";
import type { Tenant } from "@/types";

/** Lease types of a tenancy. */
export type LeaseType = Tenant["leaseType"];

/** Editable values of the tenant form; every field is text as typed (dates as ISO days, `''` = empty). */
export interface TenantFormValues {
  unitId: string;
  salutation: string;
  firstName: string;
  lastName: string;
  salutation2: string;
  firstName2: string;
  lastName2: string;
  phone: string;
  email: string;
  bankName: string;
  iban: string;
  accountHolder: string;
  moveInDate: string;
  moveOutDate: string;
  leaseType: LeaseType;
  indexBaseYear: string;
  indexReferenceValue: string;
  indexReferenceDate: string;
  indexMinMonths: string;
}

/** Date fields of the form, each backed by a `DateInput`. */
export type TenantDateField = "moveInDate" | "moveOutDate" | "indexReferenceDate";

/** Problems currently reported by the date inputs (`null` = readable). */
export type TenantDateProblems = Partial<Record<TenantDateField, DateInputProblem | null>>;

/** Field errors of the form (German texts). */
export type TenantFormErrors = Partial<Record<keyof TenantFormValues, string>>;

/** Labels of the lease types, in display order. */
export const LEASE_TYPE_OPTIONS: { value: LeaseType; label: string }[] = [
  { value: "standard", label: "Standard" },
  { value: "index", label: "Indexmiete" },
  { value: "staffel", label: "Staffelmiete" },
];

/** Values of an empty form for a new tenant. */
export const EMPTY_TENANT_FORM: TenantFormValues = {
  unitId: "",
  salutation: "Herr",
  firstName: "",
  lastName: "",
  salutation2: "",
  firstName2: "",
  lastName2: "",
  phone: "",
  email: "",
  bankName: "",
  iban: "",
  accountHolder: "",
  moveInDate: "",
  moveOutDate: "",
  leaseType: "standard",
  indexBaseYear: "",
  indexReferenceValue: "",
  indexReferenceDate: "",
  indexMinMonths: "12",
};

/** ISO day part of a stored timestamp (`2024-03-01T00:00:00.000Z` → `2024-03-01`). */
function isoDay(value: string | null | undefined): string {
  return value ? value.split("T")[0] : "";
}

/** Form values of an existing tenant. */
export function tenantToForm(tenant: Tenant): TenantFormValues {
  return {
    unitId: tenant.unitId,
    salutation: tenant.salutation,
    firstName: tenant.firstName,
    lastName: tenant.lastName,
    salutation2: tenant.salutation2 || "",
    firstName2: tenant.firstName2 || "",
    lastName2: tenant.lastName2 || "",
    phone: tenant.phone || "",
    email: tenant.email || "",
    bankName: tenant.bankName || "",
    iban: tenant.iban || "",
    accountHolder: tenant.accountHolder || "",
    moveInDate: isoDay(tenant.moveInDate),
    moveOutDate: isoDay(tenant.moveOutDate),
    leaseType: tenant.leaseType || "standard",
    indexBaseYear: tenant.indexBaseYear ? String(tenant.indexBaseYear) : "",
    indexReferenceValue: tenant.indexReferenceValue ? String(tenant.indexReferenceValue) : "",
    indexReferenceDate: isoDay(tenant.indexReferenceDate),
    indexMinMonths: String(tenant.indexMinMonths ?? 12),
  };
}

/** Request body for `POST /api/tenants` and `PUT /api/tenants/:id` (empty optional fields become `null`). */
export function formToPayload(form: TenantFormValues) {
  return {
    ...form,
    salutation2: form.salutation2 || null,
    firstName2: form.firstName2 || null,
    lastName2: form.lastName2 || null,
    phone: form.phone || null,
    email: form.email || null,
    bankName: form.bankName || null,
    iban: form.iban || null,
    accountHolder: form.accountHolder || null,
    moveOutDate: form.moveOutDate || null,
    indexBaseYear: form.indexBaseYear ? parseInt(form.indexBaseYear, 10) : null,
    indexReferenceValue: form.indexReferenceValue ? parseDecimal(form.indexReferenceValue) : null,
    indexReferenceDate: form.indexReferenceDate || null,
    indexMinMonths: parseInt(form.indexMinMonths, 10) || 12,
  };
}

/**
 * Hint for an IBAN that fails the check digits, or undefined. It is only a hint and does not block
 * saving (as before the redesign).
 */
export function ibanHint(iban: string): string | undefined {
  return iban && !validateIBAN(iban) ? "Ungültige IBAN" : undefined;
}

/**
 * Validates the form on submit. Returns the field errors; an empty object means the form can be sent.
 * @param form Current values.
 * @param problems Problems reported by the date inputs (unreadable text is not in `form`).
 */
export function validateTenantForm(form: TenantFormValues, problems: TenantDateProblems): TenantFormErrors {
  const errors: TenantFormErrors = {};
  if (!form.unitId) errors.unitId = "Bitte eine Wohnung wählen.";
  if (!form.firstName.trim()) errors.firstName = "Bitte einen Vornamen angeben.";
  if (!form.lastName.trim()) errors.lastName = "Bitte einen Nachnamen angeben.";
  errors.moveInDate = dateError(problems.moveInDate, form.moveInDate, true);
  errors.moveOutDate = dateError(problems.moveOutDate, form.moveOutDate, false);
  if (form.leaseType === "index") {
    errors.indexReferenceDate = dateError(problems.indexReferenceDate, form.indexReferenceDate, false);
    if (form.indexBaseYear && !/^\d{4}$/.test(form.indexBaseYear.trim())) {
      errors.indexBaseYear = "Bitte ein Jahr mit vier Ziffern angeben.";
    }
    if (form.indexReferenceValue && parseDecimal(form.indexReferenceValue) === null) {
      errors.indexReferenceValue = "Bitte eine Zahl angeben, z. B. 105,4.";
    }
    if (form.indexMinMonths && !/^\d+$/.test(form.indexMinMonths.trim())) {
      errors.indexMinMonths = "Bitte eine ganze Zahl angeben.";
    }
  }
  return Object.fromEntries(Object.entries(errors).filter(([, v]) => v)) as TenantFormErrors;
}
