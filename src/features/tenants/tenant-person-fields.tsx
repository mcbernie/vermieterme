import { DateInput, Field, Select, TextInput, type SelectOption } from "@zweiund40gmbh/ui";
import { SALUTATIONS } from "@/lib/constants";
import { FormSection, type TenantFieldsProps } from "./form-section";

/** Unit, main tenant and tenancy period of the tenant form. */
export function TenantPersonFields({
  form,
  errors,
  set,
  onDateProblem,
  units,
}: TenantFieldsProps & { units: SelectOption<string>[] }) {
  return (
    <FormSection title="Mieter">
      <Field label="Wohnung" required error={errors.unitId}>
        <Select
          options={units}
          value={form.unitId || null}
          onChange={(v) => set("unitId", v)}
          placeholder="Wohnung wählen …"
          searchable
        />
      </Field>
      <div className="grid gap-4 sm:grid-cols-3">
        <Field label="Anrede">
          <Select options={SALUTATIONS} value={form.salutation} onChange={(v) => set("salutation", v)} />
        </Field>
        <Field label="Vorname" required error={errors.firstName}>
          <TextInput value={form.firstName} onChange={(e) => set("firstName", e.target.value)} />
        </Field>
        <Field label="Nachname" required error={errors.lastName}>
          <TextInput value={form.lastName} onChange={(e) => set("lastName", e.target.value)} />
        </Field>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="Einzugsdatum" required error={errors.moveInDate}>
          <DateInput
            value={form.moveInDate}
            onChange={(v) => set("moveInDate", v)}
            onProblemChange={(p) => onDateProblem("moveInDate", p)}
            showError={false}
          />
        </Field>
        <Field label="Auszugsdatum" helper="Leer lassen, solange der Mieter nicht ausgezogen ist." error={errors.moveOutDate}>
          <DateInput
            value={form.moveOutDate}
            onChange={(v) => set("moveOutDate", v)}
            onProblemChange={(p) => onDateProblem("moveOutDate", p)}
            showError={false}
          />
        </Field>
      </div>
    </FormSection>
  );
}
