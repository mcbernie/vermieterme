"use client";

import { useCallback, useEffect, useState } from "react";
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

/** Sends a JSON request and reports whether it succeeded; errors are logged. */
async function send(url: string, method: string, body?: unknown): Promise<boolean> {
  try {
    const res = await fetch(url, {
      method,
      ...(body === undefined
        ? {}
        : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }),
    });
    return res.ok;
  } catch (error) {
    console.error(`Request ${method} ${url} failed:`, error);
    return false;
  }
}

/** Fetches JSON, returning null on any failure. */
async function load<T>(url: string): Promise<T | null> {
  try {
    const res = await fetch(url);
    return res.ok ? ((await res.json()) as T) : null;
  } catch (error) {
    console.error(`Failed to fetch ${url}:`, error);
    return null;
  }
}

/** Data and mutations for the properties page (properties, expanded detail, rent changes). */
export function useProperties() {
  const [properties, setProperties] = useState<PropertyWithCount[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<PropertyWithUnits | null>(null);
  const [rentChanges, setRentChanges] = useState<RentChange[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchProperties = useCallback(async () => {
    const data = await load<PropertyWithCount[]>("/api/properties");
    if (data) setProperties(data);
    setLoading(false);
  }, []);

  const fetchDetail = useCallback(async (id: string) => {
    const data = await load<PropertyWithUnits>(`/api/properties/${id}`);
    if (data) setDetail(data);
  }, []);

  useEffect(() => {
    void load<PropertyWithCount[]>("/api/properties").then((d) => {
      if (d) setProperties(d);
      setLoading(false);
    });
    void load<RentChange[]>("/api/rent-changes").then((d) => {
      if (d) setRentChanges(d);
    });
  }, []);

  /** Expands a property (loading its units) or collapses it when already open. */
  async function toggleExpand(id: string) {
    if (expandedId === id) {
      setExpandedId(null);
      setDetail(null);
      return;
    }
    setExpandedId(id);
    await fetchDetail(id);
  }

  /** Creates (no id) or updates a property. Returns true on success. */
  async function saveProperty(values: PropertyFormValues, id?: string): Promise<boolean> {
    const ok = id
      ? await send(`/api/properties/${id}`, "PUT", values)
      : await send("/api/properties", "POST", values);
    if (!ok) return false;
    await fetchProperties();
    if (id && expandedId === id) await fetchDetail(id);
    return true;
  }

  /** Deletes a property and collapses it if open. */
  async function deleteProperty(id: string) {
    if (!(await send(`/api/properties/${id}`, "DELETE"))) return;
    if (expandedId === id) {
      setExpandedId(null);
      setDetail(null);
    }
    await fetchProperties();
  }

  /** Creates (no unitId) or updates a unit of a property. Returns true on success. */
  async function saveUnit(propertyId: string, values: UnitFormValues, unitId?: string): Promise<boolean> {
    const body = { ...values, propertyId };
    const ok = unitId
      ? await send(`/api/units/${unitId}`, "PUT", body)
      : await send("/api/units", "POST", body);
    if (!ok) return false;
    await fetchDetail(propertyId);
    if (!unitId) await fetchProperties();
    return true;
  }

  /** Deletes a unit and refreshes the detail and counts. */
  async function deleteUnit(unitId: string, propertyId: string) {
    if (!(await send(`/api/units/${unitId}`, "DELETE"))) return;
    await fetchDetail(propertyId);
    await fetchProperties();
  }

  return {
    properties,
    expandedId,
    detail,
    rentChanges,
    loading,
    toggleExpand,
    saveProperty,
    deleteProperty,
    saveUnit,
    deleteUnit,
  };
}
