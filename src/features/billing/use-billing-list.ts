"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { BillingPeriodWithProperty, Property } from "@/types";
import { loadJson, sendJson, type WriteResult } from "./billing-http";
import type { PreviousYearSuggestion } from "./billing-list-utils";

/** Values of the "new billing period" form as the API expects them. */
export interface CreatePeriodValues {
  propertyId: string;
  startDate: string;
  endDate: string;
  billingDate: string | null;
  copyFromId: string | null;
}

/** Loaded list data. */
interface ListData {
  periods: BillingPeriodWithProperty[];
  properties: Property[];
}

/** Loads periods and properties; `null` when one of them failed. */
async function loadList(signal?: AbortSignal): Promise<ListData | null> {
  const [periods, properties] = await Promise.all([
    loadJson<BillingPeriodWithProperty[]>("/api/billing-periods", signal),
    loadJson<Property[]>("/api/properties", signal),
  ]);
  if (!periods.ok || !properties.ok) return null;
  return { periods: periods.data, properties: properties.data };
}

/** Data and mutations of the billing overview. */
export function useBillingList() {
  const [data, setData] = useState<ListData>({ periods: [], properties: [] });
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [creatingFor, setCreatingFor] = useState<string | null>(null);
  const request = useRef(0);
  const live = useRef(new Set<AbortController>());
  const disposed = useRef(false);
  const quickRunning = useRef(false);

  /** Stores a load result unless a newer load started meanwhile. */
  const apply = useCallback((current: number, result: ListData | null) => {
    if (current !== request.current) return;
    setLoadFailed(!result);
    if (result) setData(result);
    setLoading(false);
  }, []);

  /**
   * Loads the list; the request is aborted on unmount (no new load starts afterwards) and an outdated
   * response is dropped.
   */
  const load = useCallback(
    async (controllers: Set<AbortController>) => {
      if (disposed.current) return;
      const current = ++request.current;
      const ctrl = new AbortController();
      controllers.add(ctrl);
      const result = await loadList(ctrl.signal);
      controllers.delete(ctrl);
      if (!ctrl.signal.aborted) apply(current, result);
    },
    [apply],
  );

  /** Reloads after a mutation. */
  const reload = useCallback(() => load(live.current), [load]);

  /** Loads again after a failed load, showing the loading state meanwhile. */
  function retry() {
    setLoading(true);
    return reload();
  }

  useEffect(() => {
    const controllers = live.current;
    disposed.current = false;
    void load(controllers);
    return () => {
      disposed.current = true;
      controllers.forEach((c) => c.abort());
      controllers.clear();
    };
  }, [load]);

  /** Creates a billing period from the form and reloads the list. */
  async function create(values: CreatePeriodValues): Promise<WriteResult> {
    const result = await sendJson("/api/billing-periods", "POST", values, "Fehler beim Erstellen der Abrechnung");
    if (result.ok) await reload();
    return result;
  }

  /**
   * Creates the suggested previous-year period, copying the latest one. A second call while one is
   * running is ignored; the server rejects overlapping periods, so a retry never creates a duplicate.
   */
  async function quickCreate(s: PreviousYearSuggestion) {
    if (quickRunning.current) return;
    quickRunning.current = true;
    setCreatingFor(s.property.id);
    setActionError(null);
    const result = await sendJson(
      "/api/billing-periods",
      "POST",
      {
        propertyId: s.property.id,
        startDate: `${s.previousYear}-01-01`,
        endDate: `${s.previousYear}-12-31`,
        billingDate: null,
        copyFromId: s.sourcePeriod.id,
      },
      "Fehler beim Erstellen",
    );
    if (result.ok) await reload();
    else setActionError(result.error);
    quickRunning.current = false;
    setCreatingFor(null);
  }

  /** Deletes a billing period and reloads the list; a failure is shown as page notice. */
  async function remove(id: string) {
    setActionError(null);
    const result = await sendJson(
      `/api/billing-periods/${id}`,
      "DELETE",
      undefined,
      "Die Abrechnung konnte nicht gelöscht werden.",
    );
    if (result.ok) await reload();
    else setActionError(result.error);
  }

  return {
    periods: data.periods,
    properties: data.properties,
    loading,
    loadFailed,
    actionError,
    creatingFor,
    reload,
    retry,
    create,
    quickCreate,
    remove,
  };
}
