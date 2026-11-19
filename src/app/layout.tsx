import type { Metadata, Viewport } from "next";
import type { ReactNode } from "react";
import { PRODUCT_NAME, THEME_STORAGE_KEY } from "@/lib/brand";
import { Providers } from "./providers";
import "./globals.css";

export const metadata: Metadata = {
  title: { default: PRODUCT_NAME, template: `%s · ${PRODUCT_NAME}` },
  description: "Verwaltung und Betriebskostenabrechnung von Mietobjekten",
  icons: {
    icon: [{ url: "/favicon.svg", type: "image/svg+xml" }],
    apple: "/vermieterme-icon.svg",
  },
};

export const viewport: Viewport = { colorScheme: "light dark" };

/** Root layout: global styles and client providers (system fonts, no external requests). */
export default function RootLayout({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="de" suppressHydrationWarning>
      <head>
        {/* Applies the stored colour scheme before the first paint (file, not inline: strict CSP stays possible). */}
        {/* eslint-disable-next-line @next/next/no-sync-scripts -- must block rendering, otherwise the wrong scheme flashes */}
        <script src="/theme-init.js" data-storage-key={THEME_STORAGE_KEY} />
      </head>
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
