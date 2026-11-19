"use client";

import { useState } from "react";
import type { RentChangeDialogValue } from "@/components/rent-change-dialog";
import type { VpiSuggestion } from "@/types";
import { vpiReason } from "./vpi-text";

/** Expected failure of the adjustment with a message for the user. */
class ApplyError extends Error {}

/**
 * State and action for applying a VPI rent adjustment: creates the rent change, updates the tenant's
 * reference VPI and remembers which tenants were adjusted in this session.
 * @param refetch Reloads the dashboard data after a successful adjustment.
 */
export function useVpiAdjustment(refetch: () => Promise<void>) {
  const [target, setTarget] = useState<VpiSuggestion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appliedIds, setAppliedIds] = useState<Set<string>>(new Set());

  /** Fails by throwing, so the dialog stays open with its inputs; the message is also kept in `error`. */
  async function apply(suggestion: VpiSuggestion, value: RentChangeDialogValue) {
    setError(null);
    const fail = (message: string): never => {
      setError(message);
      throw new ApplyError(message);
    };
    try {
      const res = await fetch("/api/rent-changes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          unitId: suggestion.unitId,
          type: "rent",
          amount: value.amount,
          effectiveDate: value.effectiveDate,
          reason: value.reason ?? vpiReason(suggestion),
        }),
      });
      if (!res.ok) {
        return fail("Die Mietanpassung konnte nicht angelegt werden.");
      }
      const put = await fetch(`/api/tenants/${suggestion.tenantId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          indexReferenceValue: suggestion.currentVpi,
          indexReferenceDate: value.effectiveDate,
        }),
      });
      if (!put.ok) {
        await refetch();
        return fail("Die Mietanpassung wurde angelegt, der Referenz-VPI des Mieters konnte aber nicht aktualisiert werden.");
      }
      setAppliedIds((prev) => new Set(prev).add(suggestion.tenantId));
      await refetch();
    } catch (err) {
      if (err instanceof ApplyError) throw err;
      console.error("Failed to apply VPI adjustment:", err);
      return fail("Die Mietanpassung konnte nicht durchgeführt werden.");
    }
  }

  return { target, setTarget, appliedIds, apply, error };
}
