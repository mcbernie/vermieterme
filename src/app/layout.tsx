import type { Metadata } from "next";
import Link from "next/link";
import { SessionProvider } from "@/components/session-provider";
import "./globals.css";

export const metadata: Metadata = {
  title: "VermieterMe - Betriebskostenabrechnung",
  description: "Verwaltung und Abrechnung von Mietobjekten",
  icons: {
    icon: [{ url: "/favicon.svg", type: "image/svg+xml" }],
    apple: "/vermieterme-icon.svg",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="de">
      <body className="flex min-h-screen flex-col font-sans antialiased">
        <SessionProvider>
          <div className="flex-1">{children}</div>
          <footer className="border-t border-zinc-200 bg-zinc-50 py-6 text-center text-xs text-zinc-400">
            <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
              <p>
                VermieterMe — entwickelt von Nicolas Wilms
                <span className="mx-2">·</span>
                <Link href="/impressum" className="hover:text-zinc-600 underline">
                  Impressum
                </Link>
                <span className="mx-2">·</span>
                <a
                  href="https://github.com/nicowilms/vermieterme"
                  className="hover:text-zinc-600 underline"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  GitHub
                </a>
              </p>
            </div>
          </footer>
        </SessionProvider>
      </body>
    </html>
  );
}
