"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, Select, TextInput } from "@zweiund40gmbh/ui";
import { MONTH_OPTIONS, saveVpiEntry, type VpiPayload } from "./vpi";
import { useAction } from "@/components/form/use-action";

/** Text values of the entry form. */
export interface VpiFormValues {
  baseYear: string;
  year: string;
  month: string;
  value: string;
}

const EMPTY: VpiFormValues = { baseYear: "", year: "", month: "", value: "" };

/** Field errors of the entry form; empty when it can be saved. */
export function vpiFormErrors(form: VpiFormValues): Partial<Record<keyof VpiFormValues, string>> {
  const errors: Partial<Record<keyof VpiFormValues, string>> = {};
  if (!Number.isInteger(parseInt(form.baseYear))) errors.baseYear = "Bitte ein Basisjahr angeben.";
  if (!Number.isInteger(parseInt(form.year))) errors.year = "Bitte ein Jahr angeben.";
  if (!form.month) errors.month = "Bitte einen Monat wählen.";
  if (!Number.isFinite(parseFloat(form.value))) errors.value = "Bitte einen VPI-Wert angeben.";
  return errors;
}

/** Converts the checked form into the API payload. */
function toPayload(form: VpiFormValues): VpiPayload {
  return {
    baseYear: parseInt(form.baseYear),
    year: parseInt(form.year),
    month: parseInt(form.month),
    value: parseFloat(form.value),
  };
}

interface Props {
  /** Called after a value was stored. */
  onSaved: () => Promise<void>;
}

/** Form to add or replace one VPI value. Base year and year stay filled for the next entry. */
export function VpiEntryForm({ onSaved }: Props) {
  const [form, setForm] = useState<VpiFormValues>(EMPTY);
  const [attempted, setAttempted] = useState(false);
  const [saved, setSaved] = useState(false);
  const { busy, error, run } = useAction("Fehler beim Speichern");
  const errors = attempted ? vpiFormErrors(form) : {};

  /** Changes one field and hides an old success message. */
  function set(key: keyof VpiFormValues, value: string) {
    setForm((prev) => ({ ...prev, [key]: value }));
    setSaved(false);
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    setSaved(false);
    if (Object.keys(vpiFormErrors(form)).length > 0) return;
    const ok = await run(async () => {
      await saveVpiEntry(toPayload(form));
      setForm((prev) => ({ ...EMPTY, baseYear: prev.baseYear, year: prev.year }));
      setAttempted(false);
      await onSaved();
    });
    setSaved(ok);
  }

  return (
    <form noValidate onSubmit={submit} className="panel grid gap-4" aria-label="VPI-Eintrag hinzufügen">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Field label="Basisjahr" required error={errors.baseYear}>
          <TextInput
            type="number"
            placeholder="z.B. 2020"
            value={form.baseYear}
            onChange={(e) => set("baseYear", e.target.value)}
          />
        </Field>
        <Field label="Jahr" required error={errors.year}>
          <TextInput type="number" value={form.year} onChange={(e) => set("year", e.target.value)} />
        </Field>
        <Field label="Monat" required error={errors.month}>
          <Select
            value={form.month || null}
            options={MONTH_OPTIONS}
            placeholder="Monat wählen …"
            onChange={(month) => set("month", month)}
          />
        </Field>
        <Field label="VPI-Wert" required error={errors.value}>
          <TextInput
            type="number"
            step="0.1"
            placeholder="z.B. 117.4"
            value={form.value}
            onChange={(e) => set("value", e.target.value)}
          />
        </Field>
      </div>
      {error && <InlineNotice tone="bad">{error}</InlineNotice>}
      {saved && <InlineNotice tone="ok">Gespeichert</InlineNotice>}
      <div>
        <Button type="submit" variant="primary" disabled={busy}>
          {busy ? "Speichern …" : "Speichern"}
        </Button>
      </div>
    </form>
  );
}
