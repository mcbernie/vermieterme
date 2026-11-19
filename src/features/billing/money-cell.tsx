"use client";

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Field, MoneyInput, parseMoney, useFormat } from "@zweiund40gmbh/ui";
import type { AmountProblem } from "./use-billing-save";

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
  /** New amount in euros (`null` when cleared), already while typing; not called for unreadable text. */
  onChange: (euros: number | null) => void;
  /**
   * Reports whether the field blocks saving: `true` for unreadable text (marked), `"unfinished"` for
   * text without a digit (not marked); withdrawn with `false` when fixed, hidden or disabled.
   */
  onProblemChange: (problem: AmountProblem) => void;
  disabled?: boolean;
}

/**
 * Compact amount input for table cells: `MoneyInput` in a `Field` with a hidden label, which shows
 * the error for unreadable text right under the input. Every readable keystroke is reported, so the
 * automatic save starts from the last keystroke and not only when the field is left.
 */
export function MoneyCell({ label, value, onChange, onProblemChange, disabled }: Props) {
  const { locale } = useFormat();
  const [bad, setBad] = useState(false);
  // Text without a digit while typing ("-", "€"): not finished yet, blocks saving but shows no error.
  const [unfinished, setUnfinished] = useState(false);
  // A disabled field cannot be corrected, so it does not block saving.
  const problem: AmountProblem = disabled ? false : bad || (unfinished && "unfinished");
  const blocking = problem !== false;
  const report = useRef(onProblemChange);
  useLayoutEffect(() => {
    report.current = onProblemChange;
  }, [onProblemChange]);

  // Report the kind of problem; switching kinds (e.g. "12x" → "-") is no withdrawal in between.
  useEffect(() => {
    if (problem !== false) report.current(problem);
  }, [problem]);

  // Withdraw the problem when the field is fixed, disappears or is disabled.
  useEffect(() => {
    if (!blocking) return;
    const notify = report.current;
    return () => notify(false);
  }, [blocking]);

  /** Checks the text the input just committed (on blur or Enter). */
  function check(text: string) {
    setUnfinished(false);
    setBad(isUnreadableMoney(text, locale));
  }

  /**
   * Passes a readable amount on while typing. Unreadable text is marked at once, so the last readable
   * intermediate value (e.g. "12" before "12x3") is not saved; correcting it lifts the mark again.
   * Text without any digit is only unfinished (blocks saving, no error) until the field is left.
   */
  function typed(text: string) {
    setUnfinished(false);
    if (text.trim() === "") {
      setBad(false);
      if (value !== null) onChange(null);
      return;
    }
    const cents = parseMoney(text, locale, 2);
    if (cents === null && !/\d/.test(text)) {
      setBad(false);
      setUnfinished(true);
      return;
    }
    setBad(cents === null);
    if (cents !== null && cents !== toCents(value)) onChange(cents / 100);
  }

  return (
    <Field
      label={label}
      classNames={{ label: "sr-only" }}
      error={bad && !disabled ? "Bitte einen gültigen Betrag eingeben." : undefined}
      className="ml-auto w-36"
    >
      <MoneyInput
        size="sm"
        className="text-right"
        value={toCents(value)}
        onChange={(cents) => onChange(cents === null ? null : cents / 100)}
        onInput={(e) => typed(e.currentTarget.value)}
        onBlur={(e) => check(e.currentTarget.value)}
        onKeyDown={(e) => e.key === "Enter" && check(e.currentTarget.value)}
        disabled={disabled}
        inputMode="text"
      />
    </Field>
  );
}
