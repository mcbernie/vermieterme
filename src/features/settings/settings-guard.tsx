"use client";

import { createContext, useCallback, useContext, useEffect, useId, useMemo, useState, type ReactNode } from "react";
import { useUnsavedChangesGuard } from "@zweiund40gmbh/ui";

/** Leave check of the settings area: asks only while a sub page reports unsaved changes. */
type ConfirmLeave = () => Promise<boolean>;

interface GuardContext {
  setDirty: (id: string, dirty: boolean) => void;
  confirmLeave: ConfirmLeave;
}

const Context = createContext<GuardContext | null>(null);

/**
 * Collects the unsaved-changes state of the settings sub pages. While any part is dirty, closing or
 * reloading the tab warns (`beforeunload`) and `confirmLeave` asks before switching the sub page.
 */
export function SettingsGuardProvider({ children }: { children: ReactNode }) {
  const [dirtyParts, setDirtyParts] = useState<ReadonlySet<string>>(() => new Set());
  const { confirmLeave } = useUnsavedChangesGuard({ when: dirtyParts.size > 0 });

  const setDirty = useCallback((id: string, dirty: boolean) => {
    setDirtyParts((prev) => {
      if (prev.has(id) === dirty) return prev;
      const next = new Set(prev);
      if (dirty) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);

  const value = useMemo(() => ({ setDirty, confirmLeave: () => confirmLeave() }), [setDirty, confirmLeave]);
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

/**
 * Reports whether the calling form has unsaved changes; cleared on unmount. Outside the settings area
 * (no provider) it does nothing.
 */
export function useSettingsDirty(dirty: boolean) {
  const setDirty = useContext(Context)?.setDirty;
  const id = useId();
  useEffect(() => {
    setDirty?.(id, dirty);
  }, [setDirty, id, dirty]);
  useEffect(() => () => setDirty?.(id, false), [setDirty, id]);
}

/** Asks before leaving when there are unsaved changes; resolves `true` when leaving is fine. */
export function useConfirmLeave(): ConfirmLeave {
  return useContext(Context)?.confirmLeave ?? (async () => true);
}
