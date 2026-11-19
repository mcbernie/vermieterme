"use client";

import type { MouseEvent } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useConfirmLeave } from "./settings-guard";

/** Sub pages of the settings area. */
export type SettingsTab = "general" | "vpi" | "pdf-template";

/** Route and label of each settings sub page, in display order. */
export const SETTINGS_PAGES: readonly { key: SettingsTab; href: string; label: string }[] = [
  { key: "general", href: "/settings", label: "Allgemein" },
  { key: "vpi", href: "/settings/vpi", label: "VPI-Werte" },
  { key: "pdf-template", href: "/settings/pdf-template", label: "PDF-Vorlage" },
];

/**
 * Sub page that belongs to a path: the path is the page's route or lies below it segment by segment
 * (`/settings/vpi/x`, but not `/settings/vpi-x`). Falls back to the general settings.
 */
export function settingsTabFor(pathname: string | null): SettingsTab {
  const match = SETTINGS_PAGES.find(
    (page) =>
      page.key !== "general" && pathname !== null && (pathname === page.href || pathname.startsWith(`${page.href}/`)),
  );
  return match?.key ?? "general";
}

/** Whether a click should open the link elsewhere (new tab/window), which the guard must not block. */
function opensElsewhere(e: MouseEvent) {
  return e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey;
}

const LINK_CLASS =
  "-mb-px inline-flex h-9 items-center border-b-2 border-transparent px-4 text-sm text-ink2 hover:text-ink " +
  "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent " +
  "aria-[current=page]:border-accent aria-[current=page]:font-semibold aria-[current=page]:text-ink";

/**
 * Secondary navigation of the settings area as links (`aria-current="page"` marks the open sub page).
 * Lives in the settings layout, so it stays mounted and keeps the focus when the sub page changes.
 * Arrow keys do not navigate; a click or Enter does. With unsaved changes the guard's capture listener has already held the click back
 * and asks, so `go` leaves it alone; without them `confirmLeave` resolves `true` at once.
 */
export function SettingsNav() {
  const active = settingsTabFor(usePathname());
  const router = useRouter();
  const confirmLeave = useConfirmLeave();

  async function go(e: MouseEvent<HTMLAnchorElement>, href: string, current: boolean) {
    if (e.defaultPrevented || opensElsewhere(e)) return;
    e.preventDefault();
    if (current) return;
    if (await confirmLeave()) router.push(href);
  }

  return (
    <nav aria-label="Bereiche der Einstellungen" className="flex overflow-x-auto border-b border-line">
      {SETTINGS_PAGES.map(({ key, href, label }) => {
        const current = key === active;
        return (
          <Link
            key={key}
            href={href}
            aria-current={current ? "page" : undefined}
            className={LINK_CLASS}
            onClick={(e) => void go(e, href, current)}
          >
            {label}
          </Link>
        );
      })}
    </nav>
  );
}
