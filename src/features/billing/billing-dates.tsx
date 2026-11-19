"use client";

import { DateInput, Field } from "@zweiund40gmbh/ui";
import { formatDateISO } from "@/lib/format";
import { dateError, useDateProblem } from "@/components/form/date-problem";

interface Props {
  sentDate: string | null;
  paidDate: string | null;
  /** Stores a date; `''` clears it. */
  onChange: (field: "sentDate" | "paidDate", value: string) => void;
}

/** One optional date that is stored as soon as a readable day (or nothing) is entered. */
function DateField({ label, value, onChange }: { label: string; value: string | null; onChange: (v: string) => void }) {
  const date = useDateProblem();
  const day = value ? formatDateISO(value) : "";
  return (
    <Field label={label} error={dateError(date.problem, day, false)} className="w-48">
      <DateInput value={day} onChange={onChange} onProblemChange={date.onProblemChange} showError={false} size="sm" />
    </Field>
  );
}

/** Sent and paid date of a billing period. */
export function BillingDates({ sentDate, paidDate, onChange }: Props) {
  return (
    <div className="flex flex-wrap gap-4">
      <DateField label="Versendet am" value={sentDate} onChange={(v) => onChange("sentDate", v)} />
      <DateField label="Bezahlt am" value={paidDate} onChange={(v) => onChange("paidDate", v)} />
    </div>
  );
}
