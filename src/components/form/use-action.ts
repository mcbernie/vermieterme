"use client";

import { useCallback, useRef, useState } from "react";
import { errorText } from "@/lib/api-client";

/**
 * Busy and error state for one write action (save, delete, import). A second call while one is running is
 * ignored, so a double submit sends nothing twice. The error text comes from the failed request (a
 * `RequestError` of the API client), which lets an expired session show its own message.
 * @param fallback Error text when the failure carries no own message.
 */
export function useAction(fallback: string) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const running = useRef(false);

  /** Runs `action`; resolves `true` on success, `false` on failure or when another run is active. */
  const run = useCallback(
    async (action: () => Promise<void>): Promise<boolean> => {
      if (running.current) return false;
      running.current = true;
      setBusy(true);
      setError(null);
      try {
        await action();
        return true;
      } catch (err) {
        setError(errorText(err, fallback));
        return false;
      } finally {
        running.current = false;
        setBusy(false);
      }
    },
    [fallback],
  );

  /** Hides the current error, e.g. when a dialog is reopened. */
  const clearError = useCallback(() => setError(null), []);

  return { busy, error, run, clearError };
}
