"use client";

import { useCallback, useEffect, useState } from "react";
import type { PropertyWithUnits, RentChangeWithUnit } from "@/types";

/** Payload of a rent change as the API expects it. */
export interface RentChangePayload {
  unitId: string;
  type: "rent" | "prepayment";
  amount: number;
  effectiveDate: string;
  reason: string | null;
}

/** Sends a JSON request and reports whether it succeeded; errors are logged. */
async function send(url: string, method: string, body?: RentChangePayload): Promise<boolean> {
  try {
    const res = await fetch(url, {
      method,
      headers: body ? { "Content-Type": "application/json" } : undefined,
      body: body ? JSON.stringify(body) : undefined,
    });
    return res.ok;
  } catch (error) {
    console.error(`Request ${method} ${url} failed:`, error);
    return false;
  }
}

/** Loads rent changes and properties (with units) and offers create, update and delete. */
export function useRentChanges() {
  const [rentChanges, setRentChanges] = useState<RentChangeWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    try {
      const res = await fetch("/api/rent-changes");
      if (res.ok) setRentChanges(await res.json());
    } catch (error) {
      console.error("Failed to fetch rent changes:", error);
    }
  }, []);

  useEffect(() => {
    async function load() {
      try {
        const [, propsRes] = await Promise.all([reload(), fetch("/api/properties")]);
        if (propsRes.ok) {
          const props: PropertyWithUnits[] = await propsRes.json();
          const details = await Promise.all(props.map((p) => fetch(`/api/properties/${p.id}`)));
          const ok = details.filter((r) => r.ok);
          setProperties(await Promise.all(ok.map((r) => r.json())));
        }
      } catch (error) {
        console.error("Failed to fetch data:", error);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [reload]);

  /** Runs a mutation and reloads the list when it succeeded. */
  async function mutate(url: string, method: string, body?: RentChangePayload): Promise<boolean> {
    const ok = await send(url, method, body);
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
