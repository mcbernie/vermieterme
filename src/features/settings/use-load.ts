"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText } from "./request";

/** Fallback text when loading fails without an own message. */
export const LOAD_FAILED = "Daten konnten nicht geladen werden.";

/** Fallback text when reloading after a change fails. */
export const REFRESH_FAILED = "Die Anzeige konnte nicht aktualisiert werden. Bitte laden Sie die Seite neu.";

/**
 * Loads data for a page: aborts on unmount, drops responses of older requests and keeps the shown data
 * when a later refresh fails (that failure is reported separately as `refreshError`).
 * @param load Stable loader (module-level function); receives the abort signal.
 */
export function useLoad<T>(load: (signal: AbortSignal) => Promise<T>) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const controller = useRef<AbortController | null>(null);

  /** Starts a request; `refresh` keeps the current data visible while it runs. */
  const fetchData = useCallback(
    async (refresh: boolean) => {
      controller.current?.abort();
      const current = new AbortController();
      controller.current = current;
      // Every new request aborts the previous one, so an aborted signal marks both an outdated answer
      // and an answer after unmount; such answers are dropped even when the loader ignores the signal.
      try {
        const result = await load(current.signal);
        if (current.signal.aborted) return;
        setData(result);
        setError(null);
        setRefreshError(null);
      } catch (err) {
        if (current.signal.aborted) return;
        if (refresh) setRefreshError(errorText(err, REFRESH_FAILED));
        else setError(errorText(err, LOAD_FAILED));
      } finally {
        if (!current.signal.aborted) setLoading(false);
      }
    },
    [load],
  );

  useEffect(() => {
    void fetchData(false);
    return () => controller.current?.abort();
  }, [fetchData]);

  /** Loads again after an error, showing the loading state. */
  const retry = useCallback(() => {
    setLoading(true);
    setError(null);
    void fetchData(false);
  }, [fetchData]);

  /** Reloads after a change while the current data stays visible. */
  const refresh = useCallback(() => fetchData(true), [fetchData]);

  return { data, error, refreshError, loading, retry, refresh, setData };
}
