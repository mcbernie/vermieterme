"use client";

import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { SessionProvider } from "next-auth/react";
import Link from "next/link";
import type { ReactNode } from "react";

/** Client providers: session, UI library (German, `next/link`) and confirm dialogs. */
export function Providers({ children }: { children: ReactNode }) {
  return (
    <SessionProvider>
      <UiProvider locale="de-DE" linkComponent={Link}>
        <ConfirmProvider>{children}</ConfirmProvider>
      </UiProvider>
    </SessionProvider>
  );
}
