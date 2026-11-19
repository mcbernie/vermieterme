"use client";

import { ErrorState } from "@zweiund40gmbh/ui";
import Link from "next/link";
import { useEffect } from "react";

/** Error boundary of the app routes: neutral message (no internals), retry and a way back to the dashboard. */
export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <main className="flex min-h-[60vh] flex-col items-center justify-center gap-4 px-4 text-center">
      <ErrorState
        title="Etwas ist schiefgelaufen"
        error={error}
        formatError={() => "Ein unerwarteter Fehler ist aufgetreten. Versuchen Sie es erneut oder kehren Sie zum Dashboard zurück."}
        onRetry={reset}
      />
      <Link href="/" className="text-sm text-accentstrong hover:underline">
        Zum Dashboard
      </Link>
    </main>
  );
}
