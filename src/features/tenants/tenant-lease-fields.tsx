import { DateInput, Field, SegmentedControl, TextInput } from "@zweiund40gmbh/ui";
import { FormSection, type TenantFieldsProps } from "./form-section";
import { LEASE_TYPE_OPTIONS } from "./tenant-form-values";

/** Lease type and, for index leases, the consumer price index reference of the tenant form. */
export function TenantLeaseFields({ form, errors, set, onDateProblem }: TenantFieldsProps) {
  return (
    <FormSection title="Mietvertrag">
      <Field label="Vertragsart">
        <SegmentedControl value={form.leaseType} onChange={(v) => set("leaseType", v)} options={LEASE_TYPE_OPTIONS} />
      </Field>
      {form.leaseType === "index" && (
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="VPI Basisjahr" error={errors.indexBaseYear}>
            <TextInput
              inputMode="numeric"
              value={form.indexBaseYear}
              placeholder="z. B. 2020"
              onChange={(e) => set("indexBaseYear", e.target.value)}
            />
          </Field>
          <Field label="VPI Referenzwert" error={errors.indexReferenceValue}>
            <TextInput
              inputMode="decimal"
              className="num"
              value={form.indexReferenceValue}
              placeholder="z. B. 105,4"
              onChange={(e) => set("indexReferenceValue", e.target.value)}
            />
          </Field>
          <Field
            label="Referenzdatum (letzte Anpassung)"
            helper="Leer lassen, wenn noch nicht angepasst."
            error={errors.indexReferenceDate}
          >
            <DateInput
              value={form.indexReferenceDate}
              onChange={(v) => set("indexReferenceDate", v)}
              onProblemChange={(p) => onDateProblem("indexReferenceDate", p)}
              showError={false}
            />
          </Field>
          <Field label="Min. Monate" error={errors.indexMinMonths}>
            <TextInput
              inputMode="numeric"
              className="num"
              value={form.indexMinMonths}
              onChange={(e) => set("indexMinMonths", e.target.value)}
            />
          </Field>
        </div>
      )}
    </FormSection>
  );
}
