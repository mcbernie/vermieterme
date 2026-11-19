"use client";

import { useState, type FormEvent } from "react";
import { Button, DateInput, Field, InlineNotice, Modal, Select } from "@zweiund40gmbh/ui";
import { useSave } from "@/features/properties/use-save";
import { dateError, useDateProblem } from "@/components/form/date-problem";
import type { BillingPeriodWithProperty, Property } from "@/types";
import type { WriteResult } from "./billing-http";
import { copySourceOptions, shiftYear } from "./billing-list-utils";
import type { CreatePeriodValues } from "./use-billing-list";

interface FormState {
  propertyId: string;
  startDate: string;
  endDate: string;
  billingDate: string;
  copyFromId: string;
}

const EMPTY: FormState = { propertyId: "", startDate: "", endDate: "", billingDate: "", copyFromId: "" };
const FORM_ID = "billing-create-form";

interface Props {
  properties: Property[];
  periods: BillingPeriodWithProperty[];
  onCreate: (values: CreatePeriodValues) => Promise<WriteResult>;
  onClose: () => void;
}

/** Dialog to create a billing period, optionally copying a previous one. Mount it only while open. */
export function BillingCreateModal({ properties, periods, onCreate, onClose }: Props) {
  const [form, setForm] = useState<FormState>(EMPTY);
  const [attempted, setAttempted] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const { saving, error, run } = useSave();
  const start = useDateProblem();
  const end = useDateProblem();
  const billing = useDateProblem();

  const propertyOptions = properties.map((p) => ({ value: p.id, label: `${p.street}, ${p.city}` }));
  const copyOptions = copySourceOptions(periods, form.propertyId);
  const set = <K extends keyof FormState>(key: K, value: FormState[K]) => setForm((f) => ({ ...f, [key]: value }));

  /** Picks the period to copy and moves the dates one year past it. */
  function chooseCopySource(copyFromId: string) {
    const source = periods.find((bp) => bp.id === copyFromId);
    setForm((f) =>
      source
        ? { ...f, copyFromId, startDate: shiftYear(source.startDate), endDate: shiftYear(source.endDate) }
        : { ...f, copyFromId },
    );
  }

  function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    const invalid =
      !form.propertyId || !form.startDate || !form.endDate || start.problem || end.problem || billing.problem;
    if (invalid) return;
    void run(async () => {
      setServerError(null);
      const result = await onCreate({
        ...form,
        billingDate: form.billingDate || null,
        copyFromId: form.copyFromId || null,
      });
      setServerError(result.error);
      return result.ok;
    }, onClose);
  }

  return (
    <Modal
      title="Neue Abrechnung erstellen"
      onClose={onClose}
      preventClose={saving}
      footer={
        <>
          <Button type="button" variant="secondary" onClick={onClose} disabled={saving}>
            Abbrechen
          </Button>
          <Button type="submit" form={FORM_ID} variant="primary" disabled={saving}>
            Erstellen
          </Button>
        </>
      }
    >
      <form id={FORM_ID} onSubmit={submit} className="grid gap-4 sm:grid-cols-2" noValidate>
        <Field
          label="Objekt"
          required
          className="sm:col-span-2"
          error={attempted && !form.propertyId ? "Bitte ein Objekt wählen." : undefined}
        >
          <Select
            options={propertyOptions}
            value={form.propertyId || null}
            onChange={(v) => setForm((f) => ({ ...f, propertyId: v, copyFromId: "" }))}
            placeholder="Objekt wählen..."
          />
        </Field>
        {copyOptions.length > 0 && (
          <Field label="Vorjahr übernehmen" className="sm:col-span-2">
            <Select
              options={copyOptions}
              value={form.copyFromId || null}
              onChange={chooseCopySource}
              placeholder="Vorjahr wählen (optional)..."
            />
          </Field>
        )}
        <Field label="Beginn" required error={dateError(start.problem, form.startDate, attempted)}>
          <DateInput
            value={form.startDate}
            onChange={(v) => set("startDate", v)}
            onProblemChange={start.onProblemChange}
            showError={false}
          />
        </Field>
        <Field label="Ende" required error={dateError(end.problem, form.endDate, attempted)}>
          <DateInput
            value={form.endDate}
            onChange={(v) => set("endDate", v)}
            onProblemChange={end.onProblemChange}
            showError={false}
          />
        </Field>
        <Field
          label="Abrechnungsdatum"
          helper="Optional"
          error={dateError(billing.problem, form.billingDate, false)}
        >
          <DateInput
            value={form.billingDate}
            onChange={(v) => set("billingDate", v)}
            onProblemChange={billing.onProblemChange}
            showError={false}
          />
        </Field>
        {(serverError ?? error) && (
          <InlineNotice tone="bad" className="sm:col-span-2">
            {serverError ?? error}
          </InlineNotice>
        )}
      </form>
    </Modal>
  );
}
