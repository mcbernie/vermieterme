import Link from "next/link";

export default function NotFound() {
  return (
    <div className="flex min-h-[80vh] flex-col items-center justify-center px-4 text-center">
      <div className="relative mb-8">
        {/* Animated house with bouncing roof */}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 120 120"
          className="h-40 w-40"
        >
          {/* House body */}
          <rect
            x="25"
            y="55"
            width="70"
            height="50"
            rx="3"
            fill="#fef2f2"
            stroke="#b91c1c"
            strokeWidth="2"
          />
          {/* Door */}
          <rect x="50" y="72" width="20" height="33" rx="2" fill="#b91c1c" opacity="0.8" />
          <circle cx="66" cy="90" r="1.5" fill="white" />
          {/* Windows */}
          <rect x="32" y="64" width="14" height="14" rx="1" fill="white" stroke="#b91c1c" strokeWidth="1.5" />
          <line x1="39" y1="64" x2="39" y2="78" stroke="#b91c1c" strokeWidth="0.8" />
          <line x1="32" y1="71" x2="46" y2="71" stroke="#b91c1c" strokeWidth="0.8" />
          <rect x="74" y="64" width="14" height="14" rx="1" fill="white" stroke="#b91c1c" strokeWidth="1.5" />
          <line x1="81" y1="64" x2="81" y2="78" stroke="#b91c1c" strokeWidth="0.8" />
          <line x1="74" y1="71" x2="88" y2="71" stroke="#b91c1c" strokeWidth="0.8" />
          {/* Roof - bouncing animation */}
          <polygon
            points="10,58 60,18 110,58"
            fill="#b91c1c"
            className="origin-bottom"
          >
            <animateTransform
              attributeName="transform"
              type="translate"
              values="0,0; 0,-6; 0,0"
              dur="2s"
              repeatCount="indefinite"
              keyTimes="0; 0.5; 1"
              calcMode="spline"
              keySplines="0.4 0 0.2 1; 0.4 0 0.2 1"
            />
          </polygon>
          {/* Question mark floating out of chimney */}
          <rect x="78" y="22" width="10" height="20" rx="1" fill="#7f1d1d" />
          <text
            x="83"
            y="18"
            textAnchor="middle"
            fontFamily="Georgia, serif"
            fontSize="18"
            fontWeight="bold"
            fill="#b91c1c"
          >
            <animate
              attributeName="y"
              values="18; 5; 18"
              dur="3s"
              repeatCount="indefinite"
            />
            <animate
              attributeName="opacity"
              values="1; 0.3; 1"
              dur="3s"
              repeatCount="indefinite"
            />
            ?
          </text>
        </svg>
      </div>

      <h1
        className="mb-2 text-6xl font-bold text-red-700"
        style={{ fontFamily: "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif" }}
      >
        404
      </h1>
      <h2
        className="mb-4 text-xl font-semibold text-zinc-700"
        style={{ fontFamily: "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif" }}
      >
        Hier wohnt niemand
      </h2>
      <p className="mb-8 max-w-md text-sm text-zinc-500">
        Die gesuchte Seite konnte leider nicht gefunden werden.
        Vielleicht ist der Mieter schon ausgezogen?
      </p>
      <Link
        href="/"
        className="rounded-lg bg-red-700 px-6 py-2.5 text-sm font-medium text-white hover:bg-red-800 transition-colors"
      >
        Zurück zum Dashboard
      </Link>
    </div>
  );
}
