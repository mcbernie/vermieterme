import { Field, Select, TextInput } from "@zweiund40gmbh/ui";
import { SALUTATIONS_SECONDARY } from "@/lib/constants";
import { formatIBAN } from "@/lib/iban";
import { FormSection, type TenantFieldsProps } from "./form-section";

/** Salutations of the second tenant without the empty entry (the select is clearable instead). */
const SECOND_SALUTATIONS = SALUTATIONS_SECONDARY.filter((o) => o.value !== "");

/** Contact data, bank account and optional second tenant of the tenant form. */
export function TenantContactFields({ form, errors, set }: Omit<TenantFieldsProps, "onDateProblem">) {
  return (
    <>
      <FormSection title="Kontaktdaten">
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Telefon / Handy">
            <TextInput
              type="tel"
              value={form.phone}
              placeholder="z. B. +49 170 1234567"
              onChange={(e) => set("phone", e.target.value)}
            />
          </Field>
          <Field label="E-Mail">
            <TextInput type="email" value={form.email} onChange={(e) => set("email", e.target.value)} />
          </Field>
        </div>
      </FormSection>

      <FormSection title="Bankverbindung (für Gutschriften)">
        <div className="grid gap-4 sm:grid-cols-3">
          <Field label="Bankname">
            <TextInput value={form.bankName} onChange={(e) => set("bankName", e.target.value)} />
          </Field>
          <Field label="IBAN" error={errors.iban}>
            <TextInput
              className="num"
              value={form.iban}
              onChange={(e) => set("iban", e.target.value)}
              onBlur={() => form.iban && set("iban", formatIBAN(form.iban))}
            />
          </Field>
          <Field label="Kontoinhaber">
            <TextInput value={form.accountHolder} onChange={(e) => set("accountHolder", e.target.value)} />
          </Field>
        </div>
      </FormSection>

      <FormSection title="Zweiter Mieter (optional)">
        <div className="grid gap-4 sm:grid-cols-3">
          <Field label="Anrede">
            <Select
              options={SECOND_SALUTATIONS}
              value={form.salutation2 || null}
              onChange={(v) => set("salutation2", v ?? "")}
              placeholder="—"
              clearable
            />
          </Field>
          <Field label="Vorname">
            <TextInput value={form.firstName2} onChange={(e) => set("firstName2", e.target.value)} />
          </Field>
          <Field label="Nachname">
            <TextInput value={form.lastName2} onChange={(e) => set("lastName2", e.target.value)} />
          </Field>
        </div>
      </FormSection>
    </>
  );
}
