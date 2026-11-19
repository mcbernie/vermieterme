"use client";

import { useState } from "react";
import type { RentChangeDialogValue } from "@/components/rent-change-dialog";
import type { VpiSuggestion } from "@/types";
import { vpiReason } from "./vpi-text";

/**
 * State and action for applying a VPI rent adjustment: creates the rent change, updates the tenant's
 * reference VPI and remembers which tenants were adjusted in this session.
 * @param refetch Reloads the dashboard data after a successful adjustment.
 */
export function useVpiAdjustment(refetch: () => Promise<void>) {
  const [target, setTarget] = useState<VpiSuggestion | null>(null);
  const [appliedIds, setAppliedIds] = useState<Set<string>>(new Set());

  async function apply(suggestion: VpiSuggestion, value: RentChangeDialogValue) {
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
      if (!res.ok) return;
      await fetch(`/api/tenants/${suggestion.tenantId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          indexReferenceValue: suggestion.currentVpi,
          indexReferenceDate: value.effectiveDate,
        }),
      });
      setAppliedIds((prev) => new Set(prev).add(suggestion.tenantId));
      await refetch();
    } catch (error) {
      console.error("Failed to apply VPI adjustment:", error);
    }
  }

  return { target, setTarget, appliedIds, apply };
}
