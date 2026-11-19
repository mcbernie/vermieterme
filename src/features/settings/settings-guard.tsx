"use client";

import { createContext, useCallback, useContext, useEffect, useId, useMemo, useState, type ReactNode } from "react";
import { useRouter } from "next/navigation";
import { useUnsavedChangesGuard } from "@zweiund40gmbh/ui";
import { inAppLinkTarget } from "./leave-link";

/** Leave check of the settings area: asks only while a sub page reports unsaved changes. */
type ConfirmLeave = () => Promise<boolean>;

interface GuardContext {
  setDirty: (id: string, dirty: boolean) => void;
  confirmLeave: ConfirmLeave;
}

const Context = createContext<GuardContext | null>(null);

/**
 * Collects the unsaved-changes state of the settings sub pages. While any part is dirty:
 * - closing or reloading the tab warns (`beforeunload`),
 * - `confirmLeave` asks before switching the sub page,
 * - a plain click on any in-app link outside the settings (sidebar, brand, user menu, imprint) is caught in
 *   the capture phase and only followed via `router.push` after the user chose to discard.
 *
 * Known gap: the browser's back/forward buttons (`popstate`) are not caught, also between the settings sub
 * pages. Intercepting them would need history tricks (sentinel entries) that fight the Next.js router, so
 * changes are lost without asking there (see #37).
 */
export function SettingsGuardProvider({ children }: { children: ReactNode }) {
  const [dirtyParts, setDirtyParts] = useState<ReadonlySet<string>>(() => new Set());
  const dirty = dirtyParts.size > 0;
  const { confirmLeave } = useUnsavedChangesGuard({ when: dirty });
  const router = useRouter();

  useEffect(() => {
    if (!dirty) return;
    /** Holds back an in-app link click and follows it only after the leave check. */
    const onClick = (e: MouseEvent) => {
      const href = inAppLinkTarget(e, window.location);
      if (href === null) return;
      e.preventDefault();
      e.stopPropagation();
      void confirmLeave().then((leave) => {
        if (leave) router.push(href);
      });
    };
    document.addEventListener("click", onClick, true);
    return () => document.removeEventListener("click", onClick, true);
  }, [dirty, confirmLeave, router]);

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
