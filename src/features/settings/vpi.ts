import type { VpiEntry } from "@/types";
import { requestJson, SessionExpiredError } from "./request";
import { LOAD_FAILED } from "./use-load";

/** German month names, index 0 = January. */
export const MONTH_NAMES = [
  "Januar",
  "Februar",
  "März",
  "April",
  "Mai",
  "Juni",
  "Juli",
  "August",
  "September",
  "Oktober",
  "November",
  "Dezember",
] as const;

/** Month options for a `Select` (value = month number as text). */
export const MONTH_OPTIONS = MONTH_NAMES.map((label, index) => ({ value: String(index + 1), label }));

/** VPI values of one base year, newest first. */
export interface VpiGroup {
  baseYear: number;
  entries: VpiEntry[];
}

/** Groups entries by base year (newest base year first) and sorts each group by year and month, descending. */
export function groupByBaseYear(entries: VpiEntry[]): VpiGroup[] {
  const groups = new Map<number, VpiEntry[]>();
  for (const entry of entries) {
    const list = groups.get(entry.baseYear) ?? [];
    list.push(entry);
    groups.set(entry.baseYear, list);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => b - a)
    .map(([baseYear, list]) => ({
      baseYear,
      entries: [...list].sort((a, b) => (b.year !== a.year ? b.year - a.year : b.month - a.month)),
    }));
}

/** Loads all VPI entries. */
export const loadVpiEntries = (signal: AbortSignal) => requestJson<VpiEntry[]>("/api/vpi", LOAD_FAILED, { signal });

/** One VPI value as the API expects it. */
export interface VpiPayload {
  baseYear: number;
  year: number;
  month: number;
  value: number;
}

/** Saves (creates or replaces) one VPI value; the API upserts by year, month and base year. */
export function saveVpiEntry(payload: VpiPayload): Promise<unknown> {
  return requestJson("/api/vpi", "Fehler beim Speichern", { method: "POST", json: payload });
}

/**
 * Fetches the values of a base year from the Bundesbank and stores each one. Because the API upserts,
 * a retry does not create duplicates. Single failed values are skipped, an expired session stops the run.
 * @returns Number of stored values.
 */
export async function importFromBundesbank(baseYear: number): Promise<number> {
  const data = await requestJson<{ entries: { year: number; month: number; value: number }[] }>(
    "/api/vpi/fetch",
    "Fehler beim Abrufen der VPI-Daten",
    { method: "POST", json: { baseYear } },
  );
  let saved = 0;
  for (const entry of data.entries) {
    try {
      await saveVpiEntry({ baseYear, year: entry.year, month: entry.month, value: entry.value });
      saved++;
    } catch (error) {
      if (error instanceof SessionExpiredError) throw error;
    }
  }
  return saved;
}
