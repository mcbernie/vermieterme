import { formatIBAN } from "@/lib/iban";
import type { LandlordInfo } from "@/types";
import { requestJson } from "./request";
import { LOAD_FAILED } from "./use-load";

/** Landlord record as the API returns it (optional fields may be null). */
type LandlordRecord = { id?: string } & { [K in keyof Omit<LandlordInfo, "id">]?: string | null };

/** Empty landlord form. */
export const EMPTY_LANDLORD: LandlordInfo = {
  name: "",
  street: "",
  zip: "",
  city: "",
  phone: "",
  email: "",
  bankName: "",
  iban: "",
  accountHolder: "",
};

/** Turns the stored record (or `null` when none exists yet) into form values. */
export function toLandlordForm(data: LandlordRecord | null): LandlordInfo {
  if (!data) return { ...EMPTY_LANDLORD };
  const form: LandlordInfo = { ...EMPTY_LANDLORD };
  for (const key of Object.keys(EMPTY_LANDLORD) as (keyof typeof EMPTY_LANDLORD)[]) {
    if (key !== "id") form[key] = data[key] || "";
  }
  return data.id ? { id: data.id, ...form } : form;
}

/** Loads the landlord data as form values. */
export async function loadLandlord(signal: AbortSignal): Promise<LandlordInfo> {
  return toLandlordForm(await requestJson<LandlordRecord | null>("/api/settings", LOAD_FAILED, { signal }));
}

/** Field errors of the landlord form. */
export type LandlordErrors = Partial<Record<keyof LandlordInfo, string>>;

const REQUIRED: { key: keyof LandlordInfo; message: string }[] = [
  { key: "name", message: "Bitte einen Namen angeben." },
  { key: "street", message: "Bitte eine Straße angeben." },
  { key: "zip", message: "Bitte eine PLZ angeben." },
  { key: "city", message: "Bitte eine Stadt angeben." },
];

/**
 * Checks the landlord form: name and address are required. An invalid IBAN is only a hint at the field
 * (`ibanHint`) and does not block saving, as in the tenant form.
 * @returns Errors by field; empty when the form can be saved.
 */
export function landlordErrors(form: LandlordInfo): LandlordErrors {
  const errors: LandlordErrors = {};
  for (const { key, message } of REQUIRED) {
    if (!form[key]?.trim()) errors[key] = message;
  }
  return errors;
}

/**
 * Whether two landlord forms hold the same values (the record id is ignored). IBANs are compared in the
 * form `formatIBAN` gives them, so formatting the field on blur does not count as a change.
 */
export function sameLandlord(a: LandlordInfo, b: LandlordInfo): boolean {
  return (Object.keys(EMPTY_LANDLORD) as (keyof LandlordInfo)[]).every((key) =>
    key === "iban" ? formatIBAN(a.iban) === formatIBAN(b.iban) : a[key] === b[key],
  );
}
