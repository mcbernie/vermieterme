"use client";

import { useEffect, useId, useState } from "react";
import { Button, DateInput, Field, Modal, MoneyInput, TextInput } from "@zweiund40gmbh/ui";

export interface RentChangeDialogValue {
  amount: number;
  effectiveDate: string;
  reason: string | null;
}

interface RentChangeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (value: RentChangeDialogValue) => void | Promise<void>;
  title: string;
  description?: string;
  defaultAmount: number;
  defaultEffectiveDate: string;
  defaultReason?: string;
  amountLabel?: string;
  submitLabel?: string;
}

/** Dialog to confirm amount, effective date and reason of a rent change. Amounts are euros in and out. */
export function RentChangeDialog({
  open,
  onOpenChange,
  onSubmit,
  title,
  description,
  defaultAmount,
  defaultEffectiveDate,
  defaultReason = "",
  amountLabel = "Neuer Betrag (EUR)",
  submitLabel = "Anpassung anlegen",
}: RentChangeDialogProps) {
  const formId = useId();
  const [cents, setCents] = useState<number | null>(Math.round(defaultAmount * 100));
  const [effectiveDate, setEffectiveDate] = useState<string>(defaultEffectiveDate);
  const [reason, setReason] = useState<string>(defaultReason);
  const [submitting, setSubmitting] = useState(false);

  // Re-seed inputs whenever the dialog (re)opens with fresh defaults.
  useEffect(() => {
    if (open) {
      setCents(Math.round(defaultAmount * 100));
      setEffectiveDate(defaultEffectiveDate);
      setReason(defaultReason);
      setSubmitting(false);
    }
  }, [open, defaultAmount, defaultEffectiveDate, defaultReason]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (cents === null || cents < 0 || !effectiveDate) return;
    setSubmitting(true);
    try {
      await onSubmit({
        amount: cents / 100,
        effectiveDate,
        reason: reason.trim() ? reason.trim() : null,
      });
      onOpenChange(false);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      title={title}
      description={description || undefined}
      size="sm"
      footer={
        <>
          <Button onClick={() => onOpenChange(false)}>Abbrechen</Button>
          <Button type="submit" form={formId} variant="primary" disabled={submitting}>
            {submitting ? "Speichern..." : submitLabel}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={handleSubmit} className="grid gap-4">
        <Field label={amountLabel} required>
          <MoneyInput value={cents} onChange={setCents} min={0} />
        </Field>
        <Field label="Gültig ab" required>
          <DateInput value={effectiveDate} onChange={setEffectiveDate} showError={false} />
        </Field>
        <Field label="Grund (optional)">
          <TextInput value={reason} onChange={(e) => setReason(e.target.value)} />
        </Field>
      </form>
    </Modal>
  );
}
