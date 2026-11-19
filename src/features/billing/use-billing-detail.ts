"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { BillingPeriodDetail, CostCategory } from "@/types";
import { loadJson, sendJson } from "./billing-http";
import {
  DEFAULT_COST_VALUE,
  costPayload,
  seedValues,
  type CostFormValue,
  type CostValues,
  type PrepaymentValues,
} from "./billing-calc";

/** Load state of the detail page. */
export type DetailStatus = "loading" | "ready" | "notFound" | "failed";

/** Billing period detail: loading, editable values and the single-position mutations. */
export function useBillingDetail(id: string) {
  const [period, setPeriod] = useState<BillingPeriodDetail | null>(null);
  const [categories, setCategories] = useState<CostCategory[]>([]);
  const [costs, setCosts] = useState<CostValues>({});
  const [prepayments, setPrepayments] = useState<PrepaymentValues>({});
  const [status, setStatus] = useState<DetailStatus>("loading");
  const [actionError, setActionError] = useState<string | null>(null);
  const request = useRef(0);
  const live = useRef(new Set<AbortController>());

  /**
   * Loads the period and the cost categories and re-seeds the form values. The request is aborted on
   * unmount and a response is dropped when a newer load started. Returns false when loading failed.
   */
  const load = useCallback(
    async (controllers: Set<AbortController>): Promise<boolean> => {
      const current = ++request.current;
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
      setCosts(seeded.costs);
      setPrepayments(seeded.prepayments);
      setStatus("ready");
      return true;
    },
    [id],
  );

  /** Reloads after a mutation or on retry. */
  const reload = useCallback(() => load(live.current), [load]);

  useEffect(() => {
    const controllers = live.current;
    void load(controllers);
    return () => {
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
    patchCost,
    setPrepayment: (unitId: string, value: number | null) => setPrepayments((prev) => ({ ...prev, [unitId]: value })),
    toggleEnabled,
    setOverride,
    confirmCost,
    confirmPrepayment,
    setDate,
  };
}
