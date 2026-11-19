"use client";

import { useEffect } from "react";

/**
 * Last-resort error boundary. It replaces the root layout (own `<html>`, no providers, no library
 * components), so it only uses plain elements and the CSS variables of the tokens.
 */
export default function GlobalError({
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
    <html lang="de">
      <body style={{ margin: 0, background: "var(--bg, #f4f4f4)", color: "var(--ink, #161616)" }}>
        <main
          role="alert"
          style={{
            minHeight: "100vh",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 12,
            padding: 16,
            textAlign: "center",
            font: "14px/1.5 system-ui, sans-serif",
          }}
        >
          <h1 style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>Kritischer Fehler</h1>
          <p style={{ margin: 0, maxWidth: 400, color: "var(--ink2, #525252)" }}>
            Die Anwendung konnte nicht geladen werden. Bitte laden Sie die Seite neu.
          </p>
          <button
            type="button"
            onClick={reset}
            style={{
              border: 0,
              padding: "8px 16px",
              font: "inherit",
              fontWeight: 500,
              cursor: "pointer",
              background: "var(--accent-fill, #00939d)",
              color: "var(--on-accent, #fff)",
            }}
          >
            Seite neu laden
          </button>
        </main>
      </body>
    </html>
  );
}
