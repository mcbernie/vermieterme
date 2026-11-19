"use client";

import { useId, type ReactNode } from "react";
import { ErrorState, InlineNotice, LoadingState } from "@zweiund40gmbh/ui";

interface SectionProps {
  title: string;
  /** Short explanation below the title. */
  description?: ReactNode;
  /** Buttons on the right of the title. */
  actions?: ReactNode;
  children: ReactNode;
}

/** Titled block of a settings or profile page; the title names the section for assistive tech. */
export function SettingsSection({ title, description, actions, children }: SectionProps) {
  const id = useId();
  return (
    <section aria-labelledby={id} className="grid gap-3">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 id={id} className="section-title">
            {title}
          </h2>
          {description && <p className="mt-1 text-sm text-ink2">{description}</p>}
        </div>
        {actions && <div className="flex gap-2">{actions}</div>}
      </div>
      {children}
    </section>
  );
}

interface GateProps {
  loading: boolean;
  error: string | null;
  /** Error of the last refresh after a change; the content stays visible. */
  refreshError?: string | null;
  onRetry: () => void;
  children: ReactNode;
}

/** Shows the loading or error state of a section, otherwise its content. */
export function LoadGate({ loading, error, refreshError, onRetry, children }: GateProps) {
  if (loading) return <LoadingState />;
  if (error) return <ErrorState error={error} onRetry={onRetry} />;
  return (
    <>
      {refreshError && <InlineNotice tone="warn">{refreshError}</InlineNotice>}
      {children}
    </>
  );
}
