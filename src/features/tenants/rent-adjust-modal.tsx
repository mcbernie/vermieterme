"use client";

import { useId, useRef, useState, type FormEvent } from "react";
import {
  Button,
  DateInput,
  Field,
  InlineNotice,
  Modal,
  MoneyInput,
  SegmentedControl,
  TextInput,
  parseMoney,
  todayIso,
} from "@zweiund40gmbh/ui";
import { dateError, useDateProblem } from "@/components/form/date-problem";
import type { RentChangeType } from "./tenant-helpers";
import type { MutationError, RentAdjustment } from "./use-tenants";

const TYPE_OPTIONS: { value: RentChangeType; label: string }[] = [
  { value: "prepayment", label: "NK-Vorauszahlung" },
  { value: "rent", label: "Kaltmiete" },
];

interface Props {
  unitId: string;
  /** Name shown in the description (whose rent is changed). */
  tenantName: string;
  /** Preselected kind of change. */
  defaultType: RentChangeType;
  /** Saves the change; resolves to an error text or null on success. */
  onSave: (adjustment: RentAdjustment) => Promise<MutationError>;
  onClose: () => void;
}

/**
 * Dialog to record a new base rent or operating-cost prepayment from a date on.
 * Validates on submit and shows errors at the fields. Mount it only while it should be open.
 */
export function RentAdjustModal({ unitId, tenantName, defaultType, onSave, onClose }: Props) {
  const formId = useId();
  const [type, setType] = useState<RentChangeType>(defaultType);
  const [cents, setCents] = useState<number | null>(null);
  const [effectiveDate, setEffectiveDate] = useState(todayIso());
  const date = useDateProblem();
  const [reason, setReason] = useState("");
  const [attempted, setAttempted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [amountUnreadable, setAmountUnreadable] = useState(false);
  const amountRef = useRef<HTMLInputElement>(null);

  const amountError = amountUnreadable
    ? "Bitte einen gültigen Betrag eingeben."
    : attempted && cents === null
      ? "Bitte einen Betrag angeben."
      : undefined;
  const dateMessage = dateError(date.problem, effectiveDate, attempted);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    setSaveError(null);
    // MoneyInput keeps unreadable text without reporting it, so the last reported amount may be stale.
    const unreadable = isUnreadableAmount(amountRef.current?.value ?? "");
    setAmountUnreadable(unreadable);
    if (unreadable || cents === null || !effectiveDate || date.problem) return;
    setBusy(true);
    const error = await onSave({
      unitId,
      type,
      amount: cents / 100,
      effectiveDate,
      reason: reason.trim() || null,
    });
    setBusy(false);
    if (error) setSaveError(error);
    else onClose();
  }

  return (
    <Modal
      title="Miete / Vorauszahlung anpassen"
      description={tenantName}
      onClose={onClose}
      preventClose={busy}
      footer={
        <>
          <Button type="button" variant="secondary" onClick={onClose} disabled={busy}>
            Abbrechen
          </Button>
          <Button type="submit" form={formId} variant="primary" disabled={busy}>
            Speichern
          </Button>
        </>
      }
    >
      <form id={formId} noValidate onSubmit={submit} className="grid gap-4">
        {saveError && <InlineNotice tone="bad">{saveError}</InlineNotice>}
        <Field label="Art">
          <SegmentedControl value={type} onChange={setType} options={TYPE_OPTIONS} />
        </Field>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Neuer Betrag" required error={amountError}>
            <MoneyInput
              ref={amountRef}
              value={cents}
              onChange={(v) => {
                setCents(v);
                setAmountUnreadable(false);
              }}
              min={0}
            />
          </Field>
          <Field label="Gültig ab" required error={dateMessage}>
            <DateInput
              value={effectiveDate}
              onChange={setEffectiveDate}
              onProblemChange={date.onProblemChange}
              showError={false}
            />
          </Field>
        </div>
        <Field label="Grund (optional)">
          <TextInput value={reason} placeholder="z. B. Anpassung NK" onChange={(e) => setReason(e.target.value)} />
        </Field>
      </form>
    </Modal>
  );
}

/** Whether the text of the amount field is neither empty nor a readable amount. */
export function isUnreadableAmount(text: string): boolean {
  return text.trim() !== "" && parseMoney(text, "de-DE") === null;
}
