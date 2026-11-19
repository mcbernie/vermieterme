"use client";

import { useRef, useState } from "react";
import type { RentChangeDialogValue } from "@/components/rent-change-dialog";
import type { VpiSuggestion } from "@/types";
import { vpiReason } from "./vpi-text";

/** Expected failure of the adjustment with a message for the user. */
class ApplyError extends Error {}

const PUT_FAILED =
  "Die Mietanpassung ist angelegt, der VPI-Bezugswert konnte nicht gespeichert werden. Erneut versuchen speichert nur den Bezugswert.";

/**
 * State and action for applying a VPI rent adjustment: creates the rent change, updates the tenant's
 * reference VPI and remembers which tenants were adjusted in this session.
 * @param refetch Reloads the dashboard data after a successful adjustment.
 */
export function useVpiAdjustment(refetch: () => Promise<void>) {
  const [target, setTarget] = useState<VpiSuggestion | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Tenants whose rent change was already created; a retry then only repeats the PUT (no duplicate POST).
  const createdRef = useRef<Set<string>>(new Set());
  const [appliedIds, setAppliedIds] = useState<Set<string>>(new Set());

  /** Fails by throwing, so the dialog stays open with its inputs; the message is also kept in `error`. */
  async function apply(suggestion: VpiSuggestion, value: RentChangeDialogValue) {
    setError(null);
    const fail = (message: string): never => {
      setError(message);
      throw new ApplyError(message);
    };
    try {
      if (!createdRef.current.has(suggestion.tenantId)) {
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
        createdRef.current.add(suggestion.tenantId);
      }
      let putOk = false;
      try {
        const put = await fetch(`/api/tenants/${suggestion.tenantId}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            indexReferenceValue: suggestion.currentVpi,
            indexReferenceDate: value.effectiveDate,
          }),
        });
        putOk = put.ok;
      } catch (err) {
        console.error("Failed to save the VPI reference value:", err);
      }
      if (!putOk) {
        await refetch();
        return fail(PUT_FAILED);
      }
      createdRef.current.delete(suggestion.tenantId);
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
