"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, TextInput } from "@zweiund40gmbh/ui";
import { formatIBAN } from "@/lib/iban";
import type { LandlordInfo } from "@/types";
import { ibanHint } from "@/features/tenants/tenant-form-values";
import { landlordErrors } from "./landlord";
import { requestJson } from "./request";
import { useAction } from "./use-action";

type TextKey = Exclude<keyof LandlordInfo, "id">;

/** Plain text fields of the landlord block: key, label, input type, required. */
const CONTACT_FIELDS: { key: TextKey; label: string; type?: string; required?: boolean }[] = [
  { key: "name", label: "Name", required: true },
  { key: "street", label: "Straße", required: true },
  { key: "zip", label: "PLZ", required: true },
  { key: "city", label: "Stadt", required: true },
  { key: "phone", label: "Telefon", type: "tel" },
  { key: "email", label: "E-Mail", type: "email" },
];

interface Props {
  /** Loaded landlord data. */
  initial: LandlordInfo;
}

/**
 * Landlord and bank details form. Checked on submit (required address), errors appear at the fields; an
 * invalid IBAN is only flagged while typing and does not block saving. The button is only disabled while saving.
 */
export function LandlordForm({ initial }: Props) {
  const [form, setForm] = useState<LandlordInfo>(initial);
  const [attempted, setAttempted] = useState(false);
  const [saved, setSaved] = useState(false);
  const { busy, error, run } = useAction("Fehler beim Speichern");
  const errors = attempted ? landlordErrors(form) : {};

  /** Changes one field and hides an old success message. */
  function set(key: TextKey, value: string) {
    setForm((prev) => ({ ...prev, [key]: value }));
    setSaved(false);
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    setSaved(false);
    if (Object.keys(landlordErrors(form)).length > 0) return;
    const ok = await run(async () => {
      const result = await requestJson<{ id: string }>("/api/settings", "Fehler beim Speichern", {
        method: "POST",
        json: form,
      });
      setForm((prev) => ({ ...prev, id: result.id }));
    });
    setSaved(ok);
  }

  return (
    <form noValidate onSubmit={submit} className="panel grid gap-6" aria-label="Vermieter-Informationen">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {CONTACT_FIELDS.map(({ key, label, type, required }) => (
          <Field key={key} label={label} required={required} error={errors[key]}>
            <TextInput type={type ?? "text"} value={form[key]} onChange={(e) => set(key, e.target.value)} />
          </Field>
        ))}
      </div>

      <div className="grid gap-4 border-t border-line pt-4">
        <h3 className="label-caps">Bankverbindung</h3>
        <div className="grid gap-4 sm:grid-cols-3">
          <Field label="Bankname">
            <TextInput value={form.bankName} onChange={(e) => set("bankName", e.target.value)} />
          </Field>
          <Field label="IBAN" error={ibanHint(form.iban)}>
            <TextInput
              value={form.iban}
              autoComplete="off"
              onChange={(e) => set("iban", e.target.value)}
              onBlur={() => {
                const formatted = formatIBAN(form.iban);
                if (form.iban && formatted !== form.iban) set("iban", formatted);
              }}
            />
          </Field>
          <Field label="Kontoinhaber">
            <TextInput value={form.accountHolder} onChange={(e) => set("accountHolder", e.target.value)} />
          </Field>
        </div>
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
