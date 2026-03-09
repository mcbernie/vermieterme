"use client";

import { useState, useCallback } from "react";

interface MutationOptions {
  onSuccess?: (data: unknown) => void;
  onError?: (error: string) => void;
}

export function useApiMutation<TData = unknown>(
  url: string,
  method: "POST" | "PUT" | "DELETE" = "POST"
) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mutate = useCallback(
    async (data?: TData, options?: MutationOptions) => {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch(url, {
          method,
          headers: { "Content-Type": "application/json" },
          body: data !== undefined ? JSON.stringify(data) : undefined,
        });

        if (!res.ok) {
          const errData = await res.json().catch(() => ({}));
          const msg = errData.error || `Fehler (${res.status})`;
          setError(msg);
          options?.onError?.(msg);
          return null;
        }

        const result = await res.json().catch(() => ({}));
        options?.onSuccess?.(result);
        return result;
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Netzwerkfehler";
        setError(msg);
        options?.onError?.(msg);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [url, method]
  );

  return { mutate, loading, error };
}

export function apiPost(url: string, data: unknown) {
  return fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
}

export function apiPut(url: string, data: unknown) {
  return fetch(url, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
}

export function apiDelete(url: string) {
  return fetch(url, { method: "DELETE" });
}
