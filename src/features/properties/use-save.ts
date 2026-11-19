"use client";

import { useRef, useState } from "react";

/**
 * Pending and error state for a save action in a dialog: ignores a second call
 * while one is running and reports a failed save.
 */
export function useSave() {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const running = useRef(false);

  /** Runs `action`; calls `onDone` when it returns true, otherwise shows an error. */
  async function run(action: () => Promise<boolean>, onDone: () => void) {
    if (running.current) return;
    running.current = true;
    setSaving(true);
    setError(null);
    let ok = false;
    try {
      ok = await action();
    } catch (error) {
      console.error("Save failed:", error);
    } finally {
      running.current = false;
    }
    if (ok) {
      onDone();
      return;
    }
    setSaving(false);
    setError("Speichern fehlgeschlagen. Bitte versuchen Sie es erneut.");
  }

  return { saving, error, run };
}
