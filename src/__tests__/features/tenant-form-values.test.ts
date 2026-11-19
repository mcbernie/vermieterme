import { describe, expect, it } from "vitest";
import {
  EMPTY_TENANT_FORM,
  formToPayload,
  ibanHint,
  tenantToForm,
  validateTenantForm,
  type TenantFormValues,
} from "@/features/tenants/tenant-form-values";
import { TENANT } from "./tenant-fixtures";

/** A complete, valid form. */
const VALID: TenantFormValues = {
  ...EMPTY_TENANT_FORM,
  unitId: "u1",
  firstName: "Erika",
  lastName: "Muster",
  moveInDate: "2024-01-01",
};

describe("tenantToForm", () => {
  it("maps stored timestamps to ISO days and nulls to empty text", () => {
    const form = tenantToForm(TENANT);
    expect(form.moveInDate).toBe("2022-04-01");
    expect(form.moveOutDate).toBe("");
    expect(form.indexReferenceDate).toBe("2023-01-01");
    expect(form.indexBaseYear).toBe("2020");
    expect(form.indexReferenceValue).toBe("105.4");
    expect(form.phone).toBe("");
    expect(form.salutation2).toBe("");
  });
});

describe("formToPayload", () => {
  it("turns empty optional fields into null and parses numbers", () => {
    const payload = formToPayload({ ...VALID, indexBaseYear: "2020", indexReferenceValue: "105,4", indexMinMonths: "" });
    expect(payload.phone).toBeNull();
    expect(payload.moveOutDate).toBeNull();
    expect(payload.salutation2).toBeNull();
    expect(payload.indexBaseYear).toBe(2020);
    expect(payload.indexReferenceValue).toBe(105.4);
    expect(payload.indexMinMonths).toBe(12);
  });

  it("builds the complete request body", () => {
    expect(formToPayload(tenantToForm(TENANT))).toEqual({
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
      moveInDate: "2022-04-01",
      moveOutDate: null,
      leaseType: "index",
      indexBaseYear: 2020,
      indexReferenceValue: 105.4,
      indexReferenceDate: "2023-01-01",
      indexMinMonths: 12,
    });
  });

  it("accepts a dot as decimal separator for the index value", () => {
    expect(formToPayload({ ...VALID, indexReferenceValue: "105.4" }).indexReferenceValue).toBe(105.4);
  });
});

describe("validateTenantForm", () => {
  it("accepts a complete form", () => {
    expect(validateTenantForm(VALID, {})).toEqual({});
  });

  it("requires unit, names and move-in date", () => {
    const errors = validateTenantForm(EMPTY_TENANT_FORM, {});
    expect(Object.keys(errors).sort()).toEqual(["firstName", "lastName", "moveInDate", "unitId"]);
  });

  it("reports unreadable dates even when a stale value is present", () => {
    const errors = validateTenantForm(VALID, { moveInDate: "invalid", moveOutDate: "invalid" });
    expect(errors.moveInDate).toBe("Bitte ein gültiges Datum eingeben.");
    expect(errors.moveOutDate).toBe("Bitte ein gültiges Datum eingeben.");
  });

  it("does not block saving for an invalid IBAN", () => {
    expect(validateTenantForm({ ...VALID, iban: "DE00 1234" }, {})).toEqual({});
  });

  it("checks the index fields only for index leases", () => {
    const bad = { ...VALID, indexBaseYear: "20", indexReferenceValue: "abc", indexMinMonths: "x" };
    expect(validateTenantForm(bad, { indexReferenceDate: "invalid" })).toEqual({});
    const errors = validateTenantForm({ ...bad, leaseType: "index" }, { indexReferenceDate: "invalid" });
    expect(Object.keys(errors).sort()).toEqual([
      "indexBaseYear",
      "indexMinMonths",
      "indexReferenceDate",
      "indexReferenceValue",
    ]);
  });
});

describe("ibanHint", () => {
  it("hints at an invalid IBAN only", () => {
    expect(ibanHint("")).toBeUndefined();
    expect(ibanHint("DE00 1234")).toBe("Ungültige IBAN");
    expect(ibanHint("DE89 3704 0044 0532 0130 00")).toBeUndefined();
  });
});
