import type { NavGroup } from "@zweiund40gmbh/ui";

/** Sidebar navigation of the signed-in area. */
export const NAV_GROUPS: NavGroup[] = [
  {
    id: "main",
    items: [
      { id: "dashboard", label: "Übersicht", href: "/", icon: "chart" },
      { id: "properties", label: "Objekte", href: "/properties", icon: "building" },
      { id: "tenants", label: "Mieter", href: "/tenants", icon: "users" },
      { id: "rent-changes", label: "Mietanpassungen", href: "/rent-changes", icon: "history" },
      { id: "billing", label: "Abrechnungen", href: "/billing", icon: "doc" },
    ],
  },
  {
    id: "admin",
    heading: "Verwaltung",
    items: [{ id: "settings", label: "Einstellungen", href: "/settings", icon: "key" }],
  },
];

/** Returns the id of the navigation entry for a path: the longest matching `href`, `/` only for itself. */
export function navKeyFor(pathname: string): string | undefined {
  let best: { id: string; len: number } | undefined;
  for (const item of NAV_GROUPS.flatMap((g) => g.items)) {
    const match = item.href === "/" ? pathname === "/" : pathname === item.href || pathname.startsWith(`${item.href}/`);
    if (match && (!best || item.href.length > best.len)) best = { id: item.id, len: item.href.length };
  }
  return best?.id;
}
