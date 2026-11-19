"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { BillingPeriodDetail, CostCategory } from "@/types";
import { loadJson, sendJson } from "./billing-http";
import {
  DEFAULT_COST_VALUE,
  costPayload,
  mergeCosts,
  mergePrepayments,
  seedValues,
  type CostFormValue,
  type CostValues,
  type PrepaymentValues,
} from "./billing-calc";
import type { DirtyKeys } from "./use-billing-save";

/** Part of the unsaved amounts a load needs: the saved state at its start and the keys to keep. */
type LoadDirty = Pick<DirtyKeys, "keys" | "settled">;

/** No unsaved amounts: a reload takes everything from the server. */
const NOTHING_DIRTY: LoadDirty = { keys: () => new Set(), settled: () => 0 };

/** Load state of the detail page. */
export type DetailStatus = "loading" | "ready" | "notFound" | "failed";

/**
 * Billing period detail: loading, editable values and the single-position mutations.
 * @param id Billing period id.
 * @param dirty Amounts with unsaved changes (from the save); only these survive a reload, flags and
 *   all other amounts come from the server.
 */
export function useBillingDetail(id: string, dirty: LoadDirty = NOTHING_DIRTY) {
  const [period, setPeriod] = useState<BillingPeriodDetail | null>(null);
  const [categories, setCategories] = useState<CostCategory[]>([]);
  const [costs, setCosts] = useState<CostValues>({});
  const [prepayments, setPrepayments] = useState<PrepaymentValues>({});
  const [status, setStatus] = useState<DetailStatus>("loading");
  const [actionError, setActionError] = useState<string | null>(null);
  const request = useRef(0);
  const live = useRef(new Set<AbortController>());
  const disposed = useRef(false);

  /**
   * Loads the period and the cost categories and seeds the form values; amounts with unsaved changes
   * on the page are kept. The request is aborted on unmount (no new load starts afterwards) and a
   * response is dropped when a newer load started. Returns false when loading failed.
   */
  const load = useCallback(
    async (controllers: Set<AbortController>): Promise<boolean> => {
      if (disposed.current) return true;
      const current = ++request.current;
      // Changes not yet confirmed stored at this point may be missing from the answer, however their
      // save ends; so the saved state counts, not the latest change.
      const since = dirty.settled();
      const ctrl = new AbortController();
      controllers.add(ctrl);
      const [bp, cc] = await Promise.all([
        loadJson<BillingPeriodDetail>(`/api/billing-periods/${id}`, ctrl.signal),
        loadJson<CostCategory[]>("/api/cost-categories", ctrl.signal),
      ]);
      controllers.delete(ctrl);
      if (current !== request.current || ctrl.signal.aborted) return true;
      if (!bp.ok || !cc.ok) {
        const failed: DetailStatus = !bp.ok && bp.status === 404 ? "notFound" : "failed";
        setStatus((s) => (s === "ready" ? s : failed));
        return false;
      }
      const seeded = seedValues(bp.data);
      setPeriod(bp.data);
      setCategories(cc.data);
      // Amounts not confirmed stored when the load started survive it (they are saved on their own).
      const unsaved = dirty.keys(since);
      setCosts((prev) => mergeCosts(seeded.costs, prev, unsaved));
      setPrepayments((prev) => mergePrepayments(seeded.prepayments, prev, unsaved));
      setStatus("ready");
      return true;
    },
    [id, dirty],
  );

  /** Reloads after a mutation. */
  const reload = useCallback(() => load(live.current), [load]);

  /** Loads again after a failed load, showing the loading state meanwhile. */
  function retry() {
    setStatus("loading");
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

  /** Runs a single write, reports a failure on the page and reloads afterwards. */
  async function writeAndReload(url: string, body: unknown, failure: string, reloadOnError: boolean) {
    setActionError(null);
    const result = await sendJson(url, "POST", body, failure);
    if (!result.ok) setActionError(result.error);
    if (result.ok || reloadOnError) {
      if (!(await reload())) setActionError("Die Abrechnung konnte nicht neu geladen werden.");
    }
  }

  /** Changes one field of a cost position locally. */
  function patchCost(categoryId: string, patch: Partial<CostFormValue>) {
    setCosts((prev) => ({ ...prev, [categoryId]: { ...DEFAULT_COST_VALUE, ...prev[categoryId], ...patch } }));
  }

  /** Shows or hides a cost category in this period (stored at once). */
  async function toggleEnabled(categoryId: string, enabled: boolean) {
    patchCost(categoryId, { enabled });
    await writeAndReload(
      `/api/billing-periods/${id}/costs`,
      { costCategoryId: categoryId, enabled },
      "Die Kostenart konnte nicht umgeschaltet werden.",
      true,
    );
  }

  /** Sets (or with `null` removes) the distribution key override of a category (stored at once). */
  async function setOverride(categoryId: string, override: string | null) {
    patchCost(categoryId, { distributionKeyOverride: override });
    await writeAndReload(
      `/api/billing-periods/${id}/costs`,
      { costCategoryId: categoryId, distributionKeyOverride: override },
      "Der Verteilerschlüssel konnte nicht geändert werden.",
      true,
    );
  }

  /** Marks a cost position as reviewed, storing its current amounts. */
  async function confirmCost(categoryId: string) {
    const value = costs[categoryId];
    if (!value) return;
    await writeAndReload(
      `/api/billing-periods/${id}/costs`,
      { ...costPayload(categoryId, value), reviewed: true },
      "Die Position konnte nicht bestätigt werden.",
      false,
    );
  }

  /** Marks a prepayment as reviewed, storing its current monthly amount. */
  async function confirmPrepayment(unitId: string) {
    const value = prepayments[unitId];
    if (value === null || value === undefined) return;
    await writeAndReload(
      `/api/billing-periods/${id}/prepayments`,
      { unitId, monthlyAmount: value, reviewed: true },
      "Die Vorauszahlung konnte nicht bestätigt werden.",
      false,
    );
  }

  /** Stores the sent or paid date (`''` clears it). */
  async function setDate(field: "sentDate" | "paidDate", value: string) {
    setActionError(null);
    const result = await sendJson(
      `/api/billing-periods/${id}`,
      "PUT",
      { [field]: value || null },
      "Das Datum konnte nicht gespeichert werden.",
    );
    if (result.ok) setPeriod((prev) => (prev ? { ...prev, [field]: value || null } : prev));
    else setActionError(result.error);
  }

  return {
    status,
    period,
    categories,
    costs,
    prepayments,
    actionError,
    reload,
    retry,
    patchCost,
    setPrepayment: (unitId: string, value: number | null) => setPrepayments((prev) => ({ ...prev, [unitId]: value })),
    toggleEnabled,
    setOverride,
    confirmCost,
    confirmPrepayment,
    setDate,
  };
}
