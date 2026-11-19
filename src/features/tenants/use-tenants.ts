"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { PropertyWithUnits, RentChangeWithUnit, TenantWithUnit } from "@/types";
import { apiResult, isArray, UNEXPECTED_ANSWER, type ApiResult } from "@/lib/api-client";
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

/** Lists of the page; each is loaded and replaced on its own. */
type ListKey = "tenants" | "properties" | "rentChanges";

/** Error text of a failed result, or null. */
function errorOf(result: ApiResult<unknown>): MutationError {
  return result.ok ? null : result.error;
}

/** Accepts a property detail with its units. */
function isPropertyWithUnits(data: unknown): data is PropertyWithUnits {
  return typeof data === "object" && data !== null && Array.isArray((data as { units?: unknown }).units);
}

/** Loads all properties with their units (one detail request per property). */
async function loadProperties(): Promise<ApiResult<PropertyWithUnits[]>> {
  const list = await apiResult<{ id: string }[]>("/api/properties", { accept: isArray });
  if (!list.ok) return list;
  const details = await Promise.all(
    list.data.map((p) => apiResult(`/api/properties/${p.id}`, { accept: isPropertyWithUnits })),
  );
  const data: PropertyWithUnits[] = [];
  for (const d of details) {
    if (!d.ok) return d;
    data.push(d.data);
  }
  return { ok: true, data };
}

/** Results of all requests of the page. */
interface Snapshot {
  tenants: ApiResult<TenantWithUnit[]>;
  properties: ApiResult<PropertyWithUnits[]>;
  rentChanges: ApiResult<RentChangeWithUnit[]>;
}

/** Loads tenants, properties with units and rent changes in parallel; never rejects. */
async function loadSnapshot(): Promise<Snapshot> {
  try {
    const [tenants, properties, rentChanges] = await Promise.all([
      apiResult<TenantWithUnit[]>("/api/tenants", { accept: isArray }),
      loadProperties(),
      apiResult<RentChangeWithUnit[]>("/api/rent-changes", { accept: isArray }),
    ]);
    return { tenants, properties, rentChanges };
  } catch {
    const failed = { ok: false as const, error: UNEXPECTED_ANSWER };
    return { tenants: failed, properties: failed, rentChanges: failed };
  }
}

/** Load error per list (null = the list is current). */
type ListErrors = Record<ListKey, string | null>;

const NO_ERRORS: ListErrors = { tenants: null, properties: null, rentChanges: null };

/**
 * Data and mutations of the tenants page: tenants, properties with units (for the unit select) and
 * rent changes. Load errors are kept per list and `error` shows the first one, so a later successful
 * load of a list clears its error; mutations return their own error text.
 * Every list only takes the answer of its latest request, so a slow early answer never overwrites
 * a newer one.
 */
export function useTenants() {
  const [tenants, setTenants] = useState<TenantWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [rentChanges, setRentChanges] = useState<RentChangeWithUnit[]>([]);
  const [loading, setLoading] = useState(true);
  const [errors, setErrors] = useState<ListErrors>(NO_ERRORS);
  const error = errors.tenants ?? errors.properties ?? errors.rentChanges;
  // Number of the latest request per list (and of the latest full load).
  const latest = useRef({ tenants: 0, properties: 0, rentChanges: 0, all: 0 });

  /** Starts a request for the given lists and returns a check whether a list is still current. */
  const begin = useCallback((keys: ListKey[]) => {
    const tickets = Object.fromEntries(keys.map((k) => [k, ++latest.current[k]])) as Record<ListKey, number>;
    return (key: ListKey) => tickets[key] === latest.current[key];
  }, []);

  /** Starts a full load: takes tickets for all lists and returns a check for each list. */
  const beginAll = useCallback(() => {
    const current = begin(["tenants", "properties", "rentChanges"]);
    const run = ++latest.current.all;
    return { current, isLatestRun: () => run === latest.current.all };
  }, [begin]);

  /** Applies the answers of a full load that are still current. */
  const apply = useCallback((snapshot: Snapshot, ticket: ReturnType<typeof beginAll>) => {
    if (snapshot.tenants.ok && ticket.current("tenants")) setTenants(snapshot.tenants.data);
    if (snapshot.properties.ok && ticket.current("properties")) setProperties(snapshot.properties.data);
    if (snapshot.rentChanges.ok && ticket.current("rentChanges")) setRentChanges(snapshot.rentChanges.data);
    // Only lists without a newer request report their result (an outdated error is dropped).
    const keys = (Object.keys(NO_ERRORS) as ListKey[]).filter((key) => ticket.current(key));
    setErrors((prev) => ({ ...prev, ...Object.fromEntries(keys.map((key) => [key, errorOf(snapshot[key])])) }));
    if (ticket.isLatestRun()) setLoading(false);
  }, []);

  useEffect(() => {
    // Initial load (the state starts as "loading").
    const ticket = beginAll();
    void loadSnapshot().then((snapshot) => apply(snapshot, ticket));
  }, [beginAll, apply]);

  /** Loads everything again, e.g. after a load error. */
  async function reload() {
    setLoading(true);
    const ticket = beginAll();
    apply(await loadSnapshot(), ticket);
  }

  /** Reloads one list after a successful change; a failure shows up as page error, a success clears it. */
  const refresh = useCallback(
    async <T,>(key: ListKey, url: string, setData: (data: T[]) => void) => {
      const current = begin([key]);
      const res = await apiResult<T[]>(url, { accept: isArray });
      if (!current(key)) return;
      if (res.ok) setData(res.data);
      setErrors((prev) => ({ ...prev, [key]: errorOf(res) }));
    },
    [begin],
  );

  /** Creates (no id) or updates a tenant and reloads the list. Resolves to the save error or null. */
  const saveTenant = useCallback(
    async (form: TenantFormValues, id?: string): Promise<MutationError> => {
      const body = formToPayload(form);
      const res = id
        ? await apiResult(`/api/tenants/${id}`, { method: "PUT", json: body })
        : await apiResult("/api/tenants", { method: "POST", json: body });
      if (!res.ok) return res.error;
      await refresh("tenants", "/api/tenants", setTenants);
      return null;
    },
    [refresh],
  );

  /** Deletes a tenant and reloads the list. */
  const deleteTenant = useCallback(
    async (id: string): Promise<MutationError> => {
      const res = await apiResult(`/api/tenants/${id}`, { method: "DELETE" });
      if (!res.ok) return res.error;
      await refresh("tenants", "/api/tenants", setTenants);
      return null;
    },
    [refresh],
  );

  /** Records a rent or prepayment change and reloads the rent changes. */
  const saveAdjustment = useCallback(
    async (adjustment: RentAdjustment): Promise<MutationError> => {
      const res = await apiResult("/api/rent-changes", { method: "POST", json: adjustment });
      if (!res.ok) return res.error;
      await refresh("rentChanges", "/api/rent-changes", setRentChanges);
      return null;
    },
    [refresh],
  );

  // The tenant list itself failed (an empty table then means "unknown", not "no tenants").
  const tenantsError = errors.tenants;
  return { tenants, properties, rentChanges, loading, error, tenantsError, reload, saveTenant, deleteTenant, saveAdjustment };
}
