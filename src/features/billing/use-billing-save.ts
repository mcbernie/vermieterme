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
 * Problem of an amount field that blocks saving: `true` = unreadable text (marked at the field),
 * `"unfinished"` = text without a digit while typing (not marked), `false` = none.
 */
export type AmountProblem = boolean | "unfinished";

/** Stored kind of a blocking field problem. */
type FieldProblem = "unreadable" | "unfinished";

/**
 * Keys of the amounts changed since the last successful save (see `costKey`/`prepaymentKey`). Every
 * change gets a revision, so a save only settles the changes it actually sent.
 */
export interface DirtyKeys {
  /** Records a change of an amount. */
  mark(key: string): void;
  /** Revision of the latest change; a save that sends the values of now settles up to it. */
  revision(): number;
  /** Marks the changes up to `revision` as stored; later changes stay unsaved. */
  settle(revision: number): void;
  /**
   * Keys with unsaved changes. With `since` (a revision taken when a load started) also the keys
   * changed after it, since the loaded data may predate their save.
   */
  keys(since?: number): ReadonlySet<string>;
}

/** Creates an empty set of unsaved amounts. */
export function createDirtyKeys(): DirtyKeys {
  // Revision of the latest change per key; saves run one after another, so `stored` only grows.
  const changes = new Map<string, number>();
  let latest = 0;
  let stored = 0;
  return {
    mark: (key) => void changes.set(key, ++latest),
    revision: () => latest,
    settle(revision) {
      stored = Math.max(stored, revision);
    },
    keys(since = Infinity) {
      const after = Math.min(stored, since);
      return new Set([...changes].filter(([, rev]) => rev > after).map(([key]) => key));
    },
  };
}

/** Result shown while fields block saving: only unreadable text is reported, unfinished text waits. */
function refusalOf(problems: ReadonlyMap<string, FieldProblem>): SaveState {
  return [...problems.values()].includes("unreadable") ? "invalid" : null;
}

/** Set of unsaved amounts shared by the page loader and the save, stable over the page's lifetime. */
export function useDirtyKeys(): DirtyKeys {
  const [dirty] = useState(createDirtyKeys);
  return dirty;
}

/**
 * Stores all filled cost and prepayment amounts. Every request is an upsert keyed by category or
 * unit, so repeating the save never creates duplicates.
 */
export async function saveAll(id: string, values: SaveValues): Promise<boolean> {
  const fallback = "Fehler beim Speichern";
  const url = `/api/billing-periods/${id}/costs`;
  const costs = Object.entries(values.costs)
    .filter(([, v]) => v.totalAmount !== null)
    .map(([categoryId, v]) => sendJson(url, "POST", costPayload(categoryId, v), fallback));
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
 * page is left to show it; the send after leaving is a best effort, not a guarantee. Closing the tab
 * or reloading cannot send anything, so the browser is asked to warn while changes are unsaved, also
 * after a failed save. Text without a digit only postpones the save silently; just unreadable text
 * refuses it with "invalid".
 * @param dirty Unsaved amounts, shared with the loader; marked by `schedule(key)`, settled by saves.
 */
export function useBillingSave(id: string, values: SaveValues, dirty?: DirtyKeys) {
  const [ownDirty] = useState(createDirtyKeys);
  const changes = dirty ?? ownDirty;
  const [saving, setSaving] = useState(false);
  const [pending, setPending] = useState(false);
  const [state, setState] = useState<SaveState>(null);
  const [lastSavedAt, setLastSavedAt] = useState<Date | null>(null);
  const [unsavedCount, setUnsavedCount] = useState(0);
  const [problems, setProblems] = useState<ReadonlyMap<string, FieldProblem>>(new Map());
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

  /**
   * Sends the latest values until no further save was requested; sets no state. A requested repeat is
   * not sent while a field holds unreadable text (the values may be an outdated intermediate state):
   * the result is then "invalid" and the change stays pending.
   */
  const drain = useCallback(async (): Promise<"saved" | "error" | "invalid"> => {
    running.current = true;
    let ok = true;
    do {
      again.current = false;
      // The values are read right now, so every change up to this revision is part of this send.
      const revision = changes.revision();
      ok = await saveAll(id, valuesRef.current);
      if (ok) changes.settle(revision);
    } while (again.current && problemsRef.current.size === 0);
    running.current = false;
    if (again.current) return "invalid";
    return ok ? "saved" : "error";
  }, [id, changes]);

  /** Saves now (cancelling a pending automatic save). */
  const save = useCallback(async () => {
    if (!mounted.current) return;
    if (debounce.current) clearTimeout(debounce.current);
    debounce.current = null;
    if (problemsRef.current.size > 0) {
      setState(refusalOf(problemsRef.current));
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
    const result = await drain();
    if (!mounted.current) return;
    setSaving(false);
    setUnsavedCount(changes.keys().size);
    setState(result === "invalid" ? refusalOf(problemsRef.current) : result);
    if (result === "invalid") setPending(true);
    if (result === "saved") setLastSavedAt(new Date());
  }, [drain, changes]);

  /**
   * Schedules an automatic save after the last change.
   * @param key Changed amount (`costKey`/`prepaymentKey`); it counts as unsaved until a save succeeds.
   */
  const schedule = useCallback(
    (key?: string) => {
      if (!mounted.current) return;
      if (key !== undefined) {
        changes.mark(key);
        setUnsavedCount(changes.keys().size);
      }
      if (debounce.current) clearTimeout(debounce.current);
      setPending(true);
      debounce.current = setTimeout(() => void save(), AUTOSAVE_DELAY);
    },
    [save, changes],
  );

  /** Reports the problem of an amount field (or none); fixing the last one schedules a save. */
  const reportProblem = useCallback(
    (key: string, problem: AmountProblem) => {
      const kind = problem === true ? "unreadable" : problem || undefined;
      if (problemsRef.current.get(key) === kind) return;
      const next = new Map(problemsRef.current);
      if (kind) next.set(key, kind);
      else next.delete(key);
      problemsRef.current = next;
      setProblems(next);
      // Fields withdraw their problem while the page unmounts; that must not start a save.
      if (!mounted.current) return;
      if (refusalOf(problemsRef.current) === null) setState((s) => (s === "invalid" ? null : s));
      if (next.size === 0) schedule();
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

  // Closing or reloading the tab sends nothing: warn while changes are unsaved or on their way.
  const unsaved = pending || saving || unsavedCount > 0;
  useEffect(() => {
    if (!unsaved) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [unsaved]);

  return { saving, pending, state, lastSavedAt, problems, save, schedule, reportProblem };
}
