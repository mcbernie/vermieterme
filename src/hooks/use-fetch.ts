"use client";

import { useState, useEffect, useCallback, useRef } from "react";

export function useFetch<T>(url: string | null) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    if (!url) {
      setLoading(false);
      return;
    }
    try {
      setLoading(true);
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setData(await res.json());
      setError(null);
    } catch (err) {
      console.error(`Failed to fetch ${url}:`, err);
      setError(err instanceof Error ? err.message : "Fehler beim Laden");
    } finally {
      setLoading(false);
    }
  }, [url]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  return { data, loading, error, refetch: fetchData, setData };
}

export function useMultiFetch<T extends object>(
  urls: { [K in keyof T]: string }
) {
  const [data, setData] = useState<Partial<T>>({});
  const [loading, setLoading] = useState(true);

  const urlsRef = useRef(urls);
  urlsRef.current = urls;

  const fetchAll = useCallback(async () => {
    try {
      setLoading(true);
      const entries = Object.entries(urlsRef.current) as [keyof T, string][];
      const results = await Promise.all(
        entries.map(async ([key, url]) => {
          const res = await fetch(url);
          if (!res.ok) return [key, null] as const;
          return [key, await res.json()] as const;
        })
      );
      const newData: Partial<T> = {};
      for (const [key, value] of results) {
        if (value !== null) {
          (newData as Record<keyof T, unknown>)[key] = value;
        }
      }
      setData(newData);
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchAll();
  }, [fetchAll]);

  return { data, loading, refetch: fetchAll };
}
