"use client";

import type { ReactNode } from "react";
import { useRouter } from "next/navigation";
import { PageHeader, Tabs, tabPanelProps } from "@zweiund40gmbh/ui";

/** Sub pages of the settings area. */
export type SettingsTab = "general" | "vpi" | "pdf-template";

/** Route of each settings sub page. */
export const SETTINGS_ROUTES: Record<SettingsTab, string> = {
  general: "/settings",
  vpi: "/settings/vpi",
  "pdf-template": "/settings/pdf-template",
};

const TAB_ITEMS = [
  ["general", "Allgemein"],
  ["vpi", "VPI-Werte"],
  ["pdf-template", "PDF-Vorlage"],
] as const;

/** Base id of the tab bar; panels use `tabPanelProps(TABS_ID, key)`. */
const TABS_ID = "settings";

interface Props {
  /** Active sub page. */
  tab: SettingsTab;
  /** Explanation below the title. */
  description?: ReactNode;
  /** Buttons on the right of the header. */
  actions?: ReactNode;
  /** Content of the sub page. */
  children: ReactNode;
}

/**
 * Header of every settings sub page: title, tab bar for the sub pages (switching navigates to the route)
 * and the tab panel around the page content.
 */
export function SettingsLayout({ tab, description, actions, children }: Props) {
  const router = useRouter();
  return (
    <>
      <PageHeader
        title="Einstellungen"
        description={description}
        actions={actions}
        tabs={
          <Tabs
            id={TABS_ID}
            aria-label="Bereiche der Einstellungen"
            value={tab}
            items={TAB_ITEMS}
            onChange={(key) => {
              if (key !== tab) router.push(SETTINGS_ROUTES[key]);
            }}
          />
        }
      />
      <div {...tabPanelProps(TABS_ID, tab)} className="grid gap-8">
        {children}
      </div>
    </>
  );
}
