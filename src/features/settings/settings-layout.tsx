"use client";

import type { ReactNode } from "react";
import { PageHeader } from "@zweiund40gmbh/ui";
import { SettingsGuardProvider } from "./settings-guard";
import { SettingsNav } from "./settings-nav";

/**
 * Frame of the settings area, rendered by `src/app/(app)/settings/layout.tsx`: title, the secondary
 * navigation and the unsaved-changes guard. It stays mounted while the sub page changes.
 */
export function SettingsFrame({ children }: { children: ReactNode }) {
  return (
    <SettingsGuardProvider>
      <PageHeader title="Einstellungen" tabs={<SettingsNav />} />
      <div className="grid gap-8">{children}</div>
    </SettingsGuardProvider>
  );
}

interface IntroProps {
  /** Explanation of the sub page. */
  description?: ReactNode;
  /** Buttons of the sub page (e.g. save). */
  actions?: ReactNode;
}

/** Explanation and actions at the top of a settings sub page. */
export function SettingsIntro({ description, actions }: IntroProps) {
  if (!description && !actions) return null;
  return (
    <div className="flex flex-wrap items-start justify-between gap-3">
      {description && <p className="max-w-3xl text-sm text-ink2">{description}</p>}
      {actions && <div className="ml-auto flex gap-2">{actions}</div>}
    </div>
  );
}
