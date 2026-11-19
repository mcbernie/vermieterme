"use client";

import { useState } from "react";
import { Field, MoneyInput, parseMoney, useFormat } from "@zweiund40gmbh/ui";

/**
 * Whether the text of a money field cannot be read as an amount. Empty text is fine (= no amount).
 * @param text Raw text of the field.
 * @param locale Locale the field parses with.
 */
export function isUnreadableMoney(text: string, locale: string): boolean {
  return text.trim() !== "" && parseMoney(text, locale, 2) === null;
}

/** Euros → cents for `MoneyInput`, `null` stays empty. */
export function toCents(euros: number | null): number | null {
  return euros === null ? null : Math.round(euros * 100);
}

interface Props {
  /** Accessible label (the column header is not linked to the input). */
  label: string;
  /** Amount in euros, `null` = empty. */
  value: number | null;
  /** New amount in euros (`null` when cleared); not called for unreadable text. */
  onChange: (euros: number | null) => void;
  /** Reports whether the field currently holds unreadable text. */
  onProblemChange: (bad: boolean) => void;
  disabled?: boolean;
}

/**
 * Compact amount input for table cells: `MoneyInput` in a `Field` with a hidden label, which shows
 * the error for unreadable text right under the input.
 */
export function MoneyCell({ label, value, onChange, onProblemChange, disabled }: Props) {
  const { locale } = useFormat();
  const [bad, setBad] = useState(false);

  /** Checks the text the input just committed (on blur or Enter). */
  function check(text: string) {
    const next = isUnreadableMoney(text, locale);
    setBad(next);
    onProblemChange(next);
  }

  return (
    <Field
      label={label}
      classNames={{ label: "sr-only" }}
      error={bad ? "Bitte einen gültigen Betrag eingeben." : undefined}
      className="ml-auto w-36"
    >
      <MoneyInput
        size="sm"
        className="text-right"
        value={toCents(value)}
        onChange={(cents) => onChange(cents === null ? null : cents / 100)}
        onBlur={(e) => check(e.currentTarget.value)}
        onKeyDown={(e) => e.key === "Enter" && check(e.currentTarget.value)}
        disabled={disabled}
        inputMode="text"
      />
    </Field>
  );
}
