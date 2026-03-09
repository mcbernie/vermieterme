"use client";

import { useEffect } from "react";

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
    <div className="flex min-h-[80vh] flex-col items-center justify-center px-4 text-center">
      <div className="relative mb-8">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 120 120"
          className="h-40 w-40"
        >
          {/* House body - shaking */}
          <g>
            <animateTransform
              attributeName="transform"
              type="translate"
              values="0,0; -2,0; 2,0; -1,0; 1,0; 0,0"
              dur="0.5s"
              repeatCount="indefinite"
            />
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
            {/* Door - ajar */}
            <rect x="50" y="72" width="20" height="33" rx="2" fill="#b91c1c" opacity="0.8" />
            <rect x="50" y="72" width="12" height="33" rx="2" fill="#7f1d1d" opacity="0.5" />
            {/* Windows - one cracked */}
            <rect x="32" y="64" width="14" height="14" rx="1" fill="white" stroke="#b91c1c" strokeWidth="1.5" />
            <line x1="39" y1="64" x2="39" y2="78" stroke="#b91c1c" strokeWidth="0.8" />
            <line x1="32" y1="71" x2="46" y2="71" stroke="#b91c1c" strokeWidth="0.8" />
            <rect x="74" y="64" width="14" height="14" rx="1" fill="white" stroke="#b91c1c" strokeWidth="1.5" />
            {/* Crack in right window */}
            <line x1="76" y1="66" x2="83" y2="73" stroke="#b91c1c" strokeWidth="0.6" />
            <line x1="83" y1="66" x2="79" y2="75" stroke="#b91c1c" strokeWidth="0.6" />
            {/* Roof */}
            <polygon points="10,58 60,18 110,58" fill="#b91c1c" />
            {/* Chimney with smoke */}
            <rect x="78" y="22" width="10" height="20" rx="1" fill="#7f1d1d" />
          </g>
          {/* Smoke puffs */}
          <circle cx="83" cy="15" r="3" fill="#9ca3af" opacity="0.4">
            <animate attributeName="cy" values="15; 2" dur="2s" repeatCount="indefinite" />
            <animate attributeName="opacity" values="0.4; 0" dur="2s" repeatCount="indefinite" />
            <animate attributeName="r" values="3; 6" dur="2s" repeatCount="indefinite" />
          </circle>
          <circle cx="86" cy="12" r="2.5" fill="#9ca3af" opacity="0.3">
            <animate attributeName="cy" values="12; -1" dur="2.5s" repeatCount="indefinite" />
            <animate attributeName="opacity" values="0.3; 0" dur="2.5s" repeatCount="indefinite" />
            <animate attributeName="r" values="2.5; 5" dur="2.5s" repeatCount="indefinite" />
          </circle>
          {/* Warning sign */}
          <g>
            <animate
              attributeName="opacity"
              values="1; 0.5; 1"
              dur="1.5s"
              repeatCount="indefinite"
            />
            <polygon points="60,0 52,14 68,14" fill="#f59e0b" stroke="#b45309" strokeWidth="0.8" />
            <text
              x="60"
              y="12.5"
              textAnchor="middle"
              fontFamily="Arial, sans-serif"
              fontSize="10"
              fontWeight="bold"
              fill="#b45309"
            >
              !
            </text>
          </g>
        </svg>
      </div>

      <h1
        className="mb-2 text-4xl font-bold text-red-700"
        style={{ fontFamily: "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif" }}
      >
        Etwas ist schiefgelaufen
      </h1>
      <h2
        className="mb-4 text-lg font-semibold text-zinc-700"
        style={{ fontFamily: "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif" }}
      >
        Ein unerwarteter Fehler ist aufgetreten
      </h2>
      <p className="mb-8 max-w-md text-sm text-zinc-500">
        Keine Sorge — das Haus steht noch. Versuchen Sie es erneut
        oder kehren Sie zum Dashboard zurück.
      </p>
      <div className="flex gap-3">
        <button
          onClick={reset}
          className="rounded-lg bg-red-700 px-6 py-2.5 text-sm font-medium text-white hover:bg-red-800 transition-colors"
        >
          Erneut versuchen
        </button>
        {/* eslint-disable-next-line @next/next/no-html-link-for-pages */}
        <a
          href="/"
          className="rounded-lg border border-zinc-300 px-6 py-2.5 text-sm font-medium text-zinc-700 hover:bg-zinc-50 transition-colors"
        >
          Zum Dashboard
        </a>
      </div>
    </div>
  );
}
