"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { apiLoad, apiSend } from "@/lib/api-client";
import type { PropertyWithCount, PropertyWithUnits, RentChange } from "@/types";

/** Form values of a property. */
export interface PropertyFormValues {
  street: string;
  zip: string;
  city: string;
  totalShares: number;
}

/** Form values of a unit. */
export interface UnitFormValues {
  name: string;
  floor: string;
  shares: number;
}

/** Arguments to create (no unitId) or update a unit. */
export interface SaveUnitArgs {
  propertyId: string;
  values: UnitFormValues;
  unitId?: string;
}

/** Sends a JSON request and reports whether it succeeded; failures are logged. */
const send = (url: string, method: string, json?: unknown) => apiSend(url, { method, json });

/** Fetches JSON, returning null on any failure or abort. */
const load = <T,>(url: string, signal?: AbortSignal) => apiLoad<T>(url, { signal });

/** Fetches the property list. */
const loadList = (signal?: AbortSignal) => load<PropertyWithCount[]>("/api/properties", signal);

/** Data and mutations for the properties page (properties, expanded detail, rent changes). */
export function useProperties() {
  const [properties, setProperties] = useState<PropertyWithCount[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [loadedDetail, setDetail] = useState<PropertyWithUnits | null>(null);
  const [rentChanges, setRentChanges] = useState<RentChange[]>([]);
  const [loading, setLoading] = useState(true);
  const [detailFailed, setDetailFailed] = useState(false);
  const detailRequest = useRef(0);
  const expandedRef = useRef<string | null>(null);
  const mounted = useRef(true);

  /** Stores a loaded property list; ignored after unmount. */
  const applyList = useCallback((data: PropertyWithCount[] | null) => {
    if (!mounted.current) return;
    if (data) setProperties(data);
    setLoading(false);
  }, []);

  /** Reloads the property list after a mutation. */
  const fetchProperties = useCallback(async () => applyList(await loadList()), [applyList]);

  /** Loads one property's units; a response is dropped when a newer request started meanwhile. */
  const fetchDetail = useCallback(async (id: string) => {
    const request = ++detailRequest.current;
    const data = await load<PropertyWithUnits>(`/api/properties/${id}`);
    if (request !== detailRequest.current || !mounted.current) return;
    setDetailFailed(!data);
    if (data) setDetail(data);
  }, []);

  /** Reloads the detail after a mutation, but only while that property is still expanded. */
  const refreshDetail = useCallback(
    async (id: string) => {
      if (expandedRef.current === id) await fetchDetail(id);
    },
    [fetchDetail],
  );

  /** Sets the expanded property in state and in the ref used by async callbacks. */
  const expand = useCallback((id: string | null) => {
    expandedRef.current = id;
    setExpandedId(id);
  }, []);

  useEffect(() => {
    mounted.current = true;
    const controller = new AbortController();
    void loadList(controller.signal).then((d) => {
      if (!controller.signal.aborted) applyList(d);
    });
    void load<RentChange[]>("/api/rent-changes", controller.signal).then((d) => {
      if (d && !controller.signal.aborted) setRentChanges(d);
    });
    return () => {
      mounted.current = false;
      controller.abort();
    };
  }, [applyList]);

  /** Detail is only exposed while it belongs to the expanded property. */
  const detail = loadedDetail && loadedDetail.id === expandedId ? loadedDetail : null;

  /** Expands a property (loading its units) or collapses it when already open. */
  async function toggleExpand(id: string) {
    detailRequest.current++;
    setDetail(null);
    setDetailFailed(false);
    if (expandedId === id) {
      expand(null);
      return;
    }
    expand(id);
    await fetchDetail(id);
  }

  /** Creates (no id) or updates a property. Returns true on success. */
  async function saveProperty(values: PropertyFormValues, id?: string): Promise<boolean> {
    const ok = id
      ? await send(`/api/properties/${id}`, "PUT", values)
      : await send("/api/properties", "POST", values);
    if (!ok) return false;
    await fetchProperties();
    if (id) await refreshDetail(id);
    return true;
  }

  /** Deletes a property and collapses it if open. */
  async function deleteProperty(id: string) {
    if (!(await send(`/api/properties/${id}`, "DELETE"))) return;
    if (expandedId === id) {
      detailRequest.current++;
      expand(null);
      setDetail(null);
    }
    await fetchProperties();
  }

  /** Creates or updates a unit of a property. Returns true on success. */
  async function saveUnit({ propertyId, values, unitId }: SaveUnitArgs): Promise<boolean> {
    const body = { ...values, propertyId };
    const ok = unitId
      ? await send(`/api/units/${unitId}`, "PUT", body)
      : await send("/api/units", "POST", body);
    if (!ok) return false;
    await refreshDetail(propertyId);
    if (!unitId) await fetchProperties();
    return true;
  }

  /** Deletes a unit and refreshes the detail and counts. */
  async function deleteUnit(unitId: string, propertyId: string) {
    if (!(await send(`/api/units/${unitId}`, "DELETE"))) return;
    await refreshDetail(propertyId);
    await fetchProperties();
  }

  return {
    properties,
    expandedId,
    detail,
    detailFailed,
    rentChanges,
    loading,
    toggleExpand,
    saveProperty,
    deleteProperty,
    saveUnit,
    deleteUnit,
  };
}
