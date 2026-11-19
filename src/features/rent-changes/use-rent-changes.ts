"use client";

import { useCallback, useEffect, useState } from "react";
import { apiLoad, apiSend } from "@/lib/api-client";
import type { PropertyWithUnits, RentChangeWithUnit } from "@/types";

/** Payload of a rent change as the API expects it. */
export interface RentChangePayload {
  unitId: string;
  type: "rent" | "prepayment";
  amount: number;
  effectiveDate: string;
  reason: string | null;
}

/** Loads rent changes and properties (with units) and offers create, update and delete. */
export function useRentChanges() {
  const [rentChanges, setRentChanges] = useState<RentChangeWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    const data = await apiLoad<RentChangeWithUnit[]>("/api/rent-changes");
    if (data) setRentChanges(data);
  }, []);

  useEffect(() => {
    async function load() {
      try {
        const [, props] = await Promise.all([reload(), apiLoad<PropertyWithUnits[]>("/api/properties")]);
        if (props) {
          const details = await Promise.all(props.map((p) => apiLoad<PropertyWithUnits>(`/api/properties/${p.id}`)));
          setProperties(details.filter((d): d is PropertyWithUnits => d !== null));
        }
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [reload]);

  /** Runs a mutation and reloads the list when it succeeded. */
  async function mutate(url: string, method: string, body?: RentChangePayload): Promise<boolean> {
    const ok = await apiSend(url, { method, json: body });
    if (ok) await reload();
    return ok;
  }

  return {
    rentChanges,
    properties,
    loading,
    create: (p: RentChangePayload) => mutate("/api/rent-changes", "POST", p),
    update: (id: string, p: RentChangePayload) => mutate(`/api/rent-changes/${id}`, "PUT", p),
    remove: (id: string) => mutate(`/api/rent-changes/${id}`, "DELETE"),
  };
}
