"use client";

import { useState } from "react";
import type { DateInputProblem } from "@zweiund40gmbh/ui";

/**
 * Tracks whether a `DateInput` currently rejects its text. An unreadable text is not reported through
 * `onChange`, so the form must block submitting while a problem is present.
 */
export function useDateProblem() {
  const [problem, setProblem] = useState<DateInputProblem | null>(null);
  return { problem, onProblemChange: setProblem, reset: () => setProblem(null) };
}

/**
 * Error text for a date field, or `undefined` when the field is fine.
 * @param problem Current problem reported by the `DateInput` (`undefined` = none reported yet).
 * @param value Current ISO day (`''` = empty).
 * @param emptyIsError Whether an empty field is an error, e.g. a required field after a submit attempt.
 */
export function dateError(
  problem: DateInputProblem | null | undefined,
  value: string,
  emptyIsError: boolean
): string | undefined {
  if (problem) return "Bitte ein gültiges Datum eingeben.";
  if (emptyIsError && value === "") return "Bitte ein Datum angeben.";
  return undefined;
}
