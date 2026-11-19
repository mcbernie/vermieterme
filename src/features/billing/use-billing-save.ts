"use client";

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { sendJson } from "./billing-http";
import { costPayload, type CostValues, type PrepaymentValues } from "./billing-calc";

/** Delay of the automatic save after the last change (ms). */
export const AUTOSAVE_DELAY = 1500;

/** Result of the last save: stored, failed, or refused because an amount is unreadable. */
export type SaveState = "saved" | "error" | "invalid" | null;

/** Values the save sends. */
export interface SaveValues {
  costs: CostValues;
  prepayments: PrepaymentValues;
}

/**
 * Stores all filled cost and prepayment amounts. Every request is an upsert keyed by category or
 * unit, so repeating the save never creates duplicates.
 */
export async function saveAll(id: string, values: SaveValues): Promise<boolean> {
  const fallback = "Fehler beim Speichern";
  const costs = Object.entries(values.costs)
    .filter(([, v]) => v.totalAmount !== null)
    .map(([categoryId, v]) => sendJson(`/api/billing-periods/${id}/costs`, "POST", costPayload(categoryId, v), fallback));
  const prepayments = Object.entries(values.prepayments)
    .filter((entry): entry is [string, number] => entry[1] !== null && entry[1] !== undefined)
    .map(([unitId, monthlyAmount]) =>
      sendJson(`/api/billing-periods/${id}/prepayments`, "POST", { unitId, monthlyAmount }, fallback),
    );
  const results = await Promise.all([...costs, ...prepayments]);
  return results.every((r) => r.ok);
}

/**
 * Manual and debounced automatic saving of a billing period. Never runs two saves at once: a save
 * requested meanwhile runs once more with the latest values afterwards, also after the page
 * unmounted. While an amount field holds unreadable text nothing is saved, also not when the page
 * is left. Otherwise a pending automatic save is still sent when the page unmounts. Nothing is
 * scheduled after the unmount. A save that fails after the unmount is lost without notice, since no
 * page is left to show it; the send after leaving is a best effort, not a guarantee.
 */
export function useBillingSave(id: string, values: SaveValues) {
  const [saving, setSaving] = useState(false);
  const [pending, setPending] = useState(false);
  const [state, setState] = useState<SaveState>(null);
  const [lastSavedAt, setLastSavedAt] = useState<Date | null>(null);
  const [problems, setProblems] = useState<ReadonlySet<string>>(new Set());
  const valuesRef = useRef(values);
  const problemsRef = useRef(problems);
  const running = useRef(false);
  const again = useRef(false);
  const mounted = useRef(true);
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Saves read the latest committed values, also from timers and unmount.
  useLayoutEffect(() => {
    valuesRef.current = values;
  }, [values]);

  /** Sends the latest values until no further save was requested; sets no state. */
  const drain = useCallback(async (): Promise<boolean> => {
    running.current = true;
    let ok = true;
    do {
      again.current = false;
      ok = await saveAll(id, valuesRef.current);
    } while (again.current);
    running.current = false;
    return ok;
  }, [id]);

  /** Saves now (cancelling a pending automatic save). */
  const save = useCallback(async () => {
    if (!mounted.current) return;
    if (debounce.current) clearTimeout(debounce.current);
    debounce.current = null;
    if (problemsRef.current.size > 0) {
      setState("invalid");
      return;
    }
    // From here on the change is on its way: a running save takes it over.
    setPending(false);
    if (running.current) {
      again.current = true;
      return;
    }
    setSaving(true);
    setState(null);
    const ok = await drain();
    if (!mounted.current) return;
    setSaving(false);
    setState(ok ? "saved" : "error");
    if (ok) setLastSavedAt(new Date());
  }, [drain]);

  /** Schedules an automatic save after the last change. */
  const schedule = useCallback(() => {
    if (!mounted.current) return;
    if (debounce.current) clearTimeout(debounce.current);
    setPending(true);
    debounce.current = setTimeout(() => void save(), AUTOSAVE_DELAY);
  }, [save]);

  /** Marks an amount field as unreadable (or fine again); fixing the last one schedules a save. */
  const reportProblem = useCallback(
    (key: string, bad: boolean) => {
      const had = problemsRef.current.has(key);
      if (had === bad) return;
      const next = new Set(problemsRef.current);
      if (bad) next.add(key);
      else next.delete(key);
      problemsRef.current = next;
      setProblems(next);
      // Fields withdraw their problem while the page unmounts; that must not start a save.
      if (next.size === 0 && mounted.current) {
        setState((s) => (s === "invalid" ? null : s));
        schedule();
      }
    },
    [schedule],
  );

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (!debounce.current) return;
      clearTimeout(debounce.current);
      debounce.current = null;
      if (problemsRef.current.size > 0) return;
      // A running save sends the latest values once more; otherwise start one.
      if (running.current) again.current = true;
      else void drain();
    };
  }, [drain]);

  return { saving, pending, state, lastSavedAt, problems, save, schedule, reportProblem };
}
