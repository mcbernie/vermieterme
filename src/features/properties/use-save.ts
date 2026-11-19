"use client";

import { useState } from "react";
import { useAction } from "@/components/form/use-action";

/** Shown when a save fails. */
const SAVE_FAILED = "Speichern fehlgeschlagen. Bitte versuchen Sie es erneut.";

/**
 * Pending and error state for a save action in a dialog, built on `useAction`: ignores a second call
 * while one is running and reports a failed save. After success it stays pending, since the dialog closes.
 */
export function useSave() {
  const { busy, error, run: runAction } = useAction(SAVE_FAILED);
  const [done, setDone] = useState(false);

  /** Runs `action`; calls `onDone` when it returns true, otherwise shows an error. */
  async function run(action: () => Promise<boolean>, onDone: () => void) {
    const ok = await runAction(async () => {
      if (!(await action())) throw new Error("save failed");
      // Set together with the end of `busy`, so the dialog never shows a ready state in between.
      setDone(true);
    });
    if (ok) onDone();
  }

  return { saving: busy || done, error, run };
}
