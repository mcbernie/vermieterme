"use client";

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <html lang="de">
      <body style={{ fontFamily: "ui-sans-serif, system-ui, -apple-system, sans-serif", margin: 0, background: "#fafafa" }}>
        <div style={{
          minHeight: "100vh",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "1rem",
          textAlign: "center",
        }}>
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 120 120"
            style={{ width: 160, height: 160, marginBottom: 32 }}
          >
            <g>
              <animateTransform
                attributeName="transform"
                type="translate"
                values="0,0; -2,0; 2,0; -1,0; 1,0; 0,0"
                dur="0.5s"
                repeatCount="indefinite"
              />
              <rect x="25" y="55" width="70" height="50" rx="3" fill="#fef2f2" stroke="#b91c1c" strokeWidth="2" />
              <rect x="50" y="72" width="20" height="33" rx="2" fill="#b91c1c" opacity="0.8" />
              <rect x="32" y="64" width="14" height="14" rx="1" fill="white" stroke="#b91c1c" strokeWidth="1.5" />
              <rect x="74" y="64" width="14" height="14" rx="1" fill="white" stroke="#b91c1c" strokeWidth="1.5" />
              <line x1="76" y1="66" x2="83" y2="73" stroke="#b91c1c" strokeWidth="0.6" />
              <line x1="83" y1="66" x2="79" y2="75" stroke="#b91c1c" strokeWidth="0.6" />
              <polygon points="10,58 60,18 110,58" fill="#b91c1c" />
              <rect x="78" y="22" width="10" height="20" rx="1" fill="#7f1d1d" />
            </g>
            <circle cx="83" cy="15" r="3" fill="#9ca3af" opacity="0.4">
              <animate attributeName="cy" values="15; 2" dur="2s" repeatCount="indefinite" />
              <animate attributeName="opacity" values="0.4; 0" dur="2s" repeatCount="indefinite" />
              <animate attributeName="r" values="3; 6" dur="2s" repeatCount="indefinite" />
            </circle>
            <g>
              <animate attributeName="opacity" values="1; 0.5; 1" dur="1.5s" repeatCount="indefinite" />
              <polygon points="60,0 52,14 68,14" fill="#f59e0b" stroke="#b45309" strokeWidth="0.8" />
              <text x="60" y="12.5" textAnchor="middle" fontFamily="Arial, sans-serif" fontSize="10" fontWeight="bold" fill="#b45309">!</text>
            </g>
          </svg>

          <h1 style={{
            fontSize: "1.8rem",
            fontWeight: 700,
            color: "#b91c1c",
            marginBottom: 8,
            fontFamily: "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif",
          }}>
            Kritischer Fehler
          </h1>
          <p style={{ fontSize: "0.875rem", color: "#71717a", maxWidth: 400, marginBottom: 24 }}>
            Die Anwendung konnte nicht geladen werden. Bitte laden Sie die Seite neu.
          </p>
          <button
            onClick={reset}
            style={{
              background: "#b91c1c",
              color: "white",
              border: "none",
              borderRadius: 8,
              padding: "10px 24px",
              fontSize: "0.875rem",
              fontWeight: 500,
              cursor: "pointer",
            }}
          >
            Seite neu laden
          </button>
        </div>
      </body>
    </html>
  );
}
