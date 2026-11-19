"use client";

import { useCallback, useEffect, useState } from "react";
import type { PropertyWithUnits, RentChangeWithUnit, TenantWithUnit } from "@/types";
import { requestJson, type RequestResult } from "./request";
import { formToPayload, type TenantFormValues } from "./tenant-form-values";
import type { RentChangeType } from "./tenant-helpers";

/** Values of a manual rent or prepayment change (amount in euros). */
export interface RentAdjustment {
  unitId: string;
  type: RentChangeType;
  amount: number;
  effectiveDate: string;
  reason: string | null;
}

/** Result of a mutation: null on success, otherwise the German error text. */
export type MutationError = string | null;

/** Error text of a failed result, or null. */
function errorOf(result: RequestResult<unknown>): MutationError {
  return result.ok ? null : result.error;
}

/** Loads all properties with their units (one detail request per property). */
async function loadProperties(): Promise<RequestResult<PropertyWithUnits[]>> {
  const list = await requestJson<PropertyWithUnits[]>("/api/properties");
  if (!list.ok) return list;
  const details = await Promise.all(list.data.map((p) => requestJson<PropertyWithUnits>(`/api/properties/${p.id}`)));
  const failed = details.find((d) => !d.ok);
  if (failed && !failed.ok) return failed;
  return { ok: true, data: details.flatMap((d) => (d.ok ? [d.data] : [])) };
}

/** Results of all requests of the page. */
interface Snapshot {
  tenants: RequestResult<TenantWithUnit[]>;
  properties: RequestResult<PropertyWithUnits[]>;
  rentChanges: RequestResult<RentChangeWithUnit[]>;
}

/** Loads tenants, properties with units and rent changes in parallel. */
async function loadSnapshot(): Promise<Snapshot> {
  const [tenants, properties, rentChanges] = await Promise.all([
    requestJson<TenantWithUnit[]>("/api/tenants"),
    loadProperties(),
    requestJson<RentChangeWithUnit[]>("/api/rent-changes"),
  ]);
  return { tenants, properties, rentChanges };
}

/** First error of a snapshot, or null when everything loaded. */
function firstError(snapshot: Snapshot): string | null {
  return errorOf(snapshot.tenants) ?? errorOf(snapshot.properties) ?? errorOf(snapshot.rentChanges);
}

/**
 * Data and mutations of the tenants page: tenants, properties with units (for the unit select) and
 * rent changes. Load errors are collected in `error`; mutations return their own error text.
 */
export function useTenants() {
  const [tenants, setTenants] = useState<TenantWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [rentChanges, setRentChanges] = useState<RentChangeWithUnit[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  /** Reloads one list after a successful change; a failure shows up as page error. */
  async function refresh<T>(url: string, setData: (data: T) => void) {
    const res = await requestJson<T>(url);
    if (res.ok) setData(res.data);
    else setError(res.error);
  }

  /** Applies a complete load: data that arrived and the first error. */
  const apply = useCallback((snapshot: Snapshot) => {
    if (snapshot.tenants.ok) setTenants(snapshot.tenants.data);
    if (snapshot.properties.ok) setProperties(snapshot.properties.data);
    if (snapshot.rentChanges.ok) setRentChanges(snapshot.rentChanges.data);
    setError(firstError(snapshot));
    setLoading(false);
  }, []);

  useEffect(() => {
    // Initial load (the state starts as "loading").
    void loadSnapshot().then(apply);
  }, [apply]);

  /** Loads everything again, e.g. after a load error. */
  async function reload() {
    setLoading(true);
    apply(await loadSnapshot());
  }

  /** Creates (no id) or updates a tenant and reloads the list. Resolves to the save error or null. */
  async function saveTenant(form: TenantFormValues, id?: string): Promise<MutationError> {
    const payload = formToPayload(form);
    const res = id
      ? await requestJson(`/api/tenants/${id}`, "PUT", payload)
      : await requestJson("/api/tenants", "POST", payload);
    if (!res.ok) return res.error;
    await refresh("/api/tenants", setTenants);
    return null;
  }

  /** Deletes a tenant and reloads the list. */
  async function deleteTenant(id: string): Promise<MutationError> {
    const res = await requestJson(`/api/tenants/${id}`, "DELETE");
    if (!res.ok) return res.error;
    await refresh("/api/tenants", setTenants);
    return null;
  }

  /** Records a rent or prepayment change and reloads the rent changes. */
  async function saveAdjustment(adjustment: RentAdjustment): Promise<MutationError> {
    const res = await requestJson("/api/rent-changes", "POST", adjustment);
    if (!res.ok) return res.error;
    await refresh("/api/rent-changes", setRentChanges);
    return null;
  }

  return { tenants, properties, rentChanges, loading, error, reload, saveTenant, deleteTenant, saveAdjustment };
}
