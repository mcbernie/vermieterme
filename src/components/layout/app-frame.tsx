"use client";

import { AppShell, Header, Sidebar, ThemeToggle, UserMenu, type NavGroup } from "@zweiund40gmbh/ui";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { signOut, useSession } from "next-auth/react";
import type { ReactNode } from "react";
import { PRODUCT_NAME, THEME_STORAGE_KEY } from "@/lib/brand";
import { navKeyFor, NAV_GROUPS } from "./nav-items";

/** Brand of the header: accent square and product name. */
const BRAND = (
  <>
    <span aria-hidden="true" className="inline-block h-3.5 w-3.5" style={{ background: "var(--accent)" }} />
    {PRODUCT_NAME}
  </>
);

/** Sidebar footer: version, imprint and source code. */
function SidebarFooter() {
  return (
    <div className="flex flex-col gap-1 text-xs text-ink3">
      <span className="num">Version {process.env.APP_VERSION}</span>
      <span>
        <Link href="/impressum" className="hover:text-ink">Impressum</Link>
        {" · "}
        <a href="https://github.com/mcbernie/vermieterme" target="_blank" rel="noopener noreferrer" className="hover:text-ink">
          Quellcode
        </a>
      </span>
    </div>
  );
}

/**
 * Application frame of the signed-in area: header with brand and user menu (profile, colour scheme, sign-out),
 * sidebar navigation and the main area.
 */
export function AppFrame({ children }: { children: ReactNode }) {
  const { data: session } = useSession();
  const active = navKeyFor(usePathname());
  const groups: NavGroup[] = NAV_GROUPS;
  const name = session?.user?.name || session?.user?.email || "Angemeldet";
  return (
    <AppShell
      mainId="inhalt"
      header={
        <Header
          className="app-header"
          brand={BRAND}
          brandHref="/"
          actions={
            <UserMenu
              user={{ name, email: session?.user?.email ?? undefined }}
              items={[{ id: "profile", label: "Profil", icon: "user", href: "/profile" }]}
              onSignOut={() => signOut({ callbackUrl: "/login" })}
            >
              <ThemeToggle storageKey={THEME_STORAGE_KEY} />
            </UserMenu>
          }
        />
      }
      sidebar={<Sidebar groups={groups} isActive={(item) => item.id === active} footer={<SidebarFooter />} />}
    >
      <div className="mx-auto w-full max-w-7xl px-4 py-6 sm:px-6 lg:px-8">{children}</div>
    </AppShell>
  );
}
