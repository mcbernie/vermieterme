"use client";

import { useEffect, useId, useState } from "react";
import { Button, DateInput, InlineNotice, Field, Modal, MoneyInput, Select, TextInput } from "@zweiund40gmbh/ui";
import { dateError, useDateProblem } from "./date-problem";
import type { RentChangeWithUnit } from "@/types";
import type { RentChangePayload } from "./use-rent-changes";
import { TYPE_OPTIONS, centsToEuros, eurosToCents } from "./rent-change-utils";

interface FormState {
  unitId: string;
  type: "rent" | "prepayment";
  cents: number | null;
  effectiveDate: string;
  reason: string;
}

const EMPTY: FormState = { unitId: "", type: "rent", cents: null, effectiveDate: "", reason: "" };

interface RentChangeFormModalProps {
  open: boolean;
  /** Rent change to edit; `null` creates a new one. */
  editing: RentChangeWithUnit | null;
  unitOptions: { value: string; label: string }[];
  onClose: () => void;
  onSubmit: (payload: RentChangePayload) => Promise<boolean>;
}

/** Initial form values for a new or an existing rent change. */
function initialState(editing: RentChangeWithUnit | null): FormState {
  if (!editing) return EMPTY;
  return {
    unitId: editing.unitId,
    type: editing.type,
    cents: eurosToCents(editing.amount),
    effectiveDate: editing.effectiveDate.split("T")[0],
    reason: editing.reason || "",
  };
}

/** Dialog to create or edit a rent change. */
export function RentChangeFormModal({ open, editing, unitOptions, onClose, onSubmit }: RentChangeFormModalProps) {
  const formId = useId();
  const [form, setForm] = useState<FormState>(EMPTY);
  const [submitting, setSubmitting] = useState(false);
  const [attempted, setAttempted] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const date = useDateProblem();

  useEffect(() => {
    if (open) {
      setForm(initialState(editing));
      setSubmitting(false);
      setAttempted(false);
      setSubmitError(null);
      date.reset();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, editing]);

  const set = <K extends keyof FormState>(key: K, value: FormState[K]) => setForm((f) => ({ ...f, [key]: value }));
  const valid = form.unitId !== "" && form.cents !== null && form.effectiveDate !== "" && !date.problem;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setAttempted(true);
    if (!valid || form.cents === null) return;
    setSubmitting(true);
    setSubmitError(null);
    try {
      const ok = await onSubmit({
        unitId: form.unitId,
        type: form.type,
        amount: centsToEuros(form.cents),
        effectiveDate: form.effectiveDate,
        reason: form.reason || null,
      });
      if (ok) onClose();
      else setSubmitError("Das Speichern ist fehlgeschlagen.");
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : "Das Speichern ist fehlgeschlagen.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Modal
      open={open}
      onOpenChange={(next) => !next && onClose()}
      title={editing ? "Anpassung bearbeiten" : "Neue Anpassung anlegen"}
      footer={
        <>
          <Button onClick={onClose}>Abbrechen</Button>
          <Button type="submit" form={formId} variant="primary" disabled={submitting || date.problem !== null}>
            {editing ? "Aktualisieren" : "Speichern"}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={handleSubmit} className="grid gap-4">
        <Field label="Wohnung" required error={attempted && !form.unitId ? "Bitte eine Wohnung wählen." : undefined}>
          <Select
            options={unitOptions}
            value={form.unitId || null}
            onChange={(v) => set("unitId", v)}
            placeholder="Wohnung wählen..."
          />
        </Field>
        <Field label="Typ" required>
          <Select options={[...TYPE_OPTIONS]} value={form.type} onChange={(v) => set("type", v)} />
        </Field>
        <Field label="Betrag (EUR)" required error={attempted && form.cents === null ? "Bitte einen Betrag angeben." : undefined}>
          <MoneyInput value={form.cents} onChange={(v) => set("cents", v)} min={0} />
        </Field>
        <Field label="Gültig ab" required error={dateError(date.problem, form.effectiveDate, attempted)}>
          <DateInput
            value={form.effectiveDate}
            onChange={(v) => set("effectiveDate", v)}
            onProblemChange={date.onProblemChange}
            showError={false}
          />
        </Field>
        <Field label="Grund (optional)">
          <TextInput value={form.reason} onChange={(e) => set("reason", e.target.value)} />
        </Field>
        {submitError && <InlineNotice tone="bad">{submitError}</InlineNotice>}
      </form>
    </Modal>
  );
}
