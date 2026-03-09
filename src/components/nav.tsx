"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { signOut, useSession } from "next-auth/react";

const links = [
  { href: "/", label: "Dashboard" },
  { href: "/properties", label: "Objekte" },
  { href: "/tenants", label: "Mieter" },
  { href: "/billing", label: "Abrechnungen" },
  { href: "/settings", label: "Einstellungen" },
];

function LogoIcon({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      className={className}
    >
      <defs>
        <linearGradient id="navLogoBg" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#b91c1c" />
          <stop offset="100%" stopColor="#7f1d1d" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="6" fill="url(#navLogoBg)" />
      <polygon points="4,15 16,5 28,15" fill="white" opacity={0.95} />
      <rect x="6" y="14" width="20" height="14" fill="white" opacity={0.95} rx="1" />
      <rect x="13" y="20" width="6" height="8" fill="#b91c1c" rx="1" />
      <rect x="8" y="17" width="4" height="4" fill="#b91c1c" rx="0.5" opacity={0.3} />
      <rect x="8" y="17" width="4" height="4" fill="none" stroke="#b91c1c" strokeWidth="0.8" rx="0.5" />
      <rect x="20" y="17" width="4" height="4" fill="#b91c1c" rx="0.5" opacity={0.3} />
      <rect x="20" y="17" width="4" height="4" fill="none" stroke="#b91c1c" strokeWidth="0.8" rx="0.5" />
      <circle cx="24" cy="10" r="5" fill="#7f1d1d" stroke="white" strokeWidth="1" />
      <text
        x="24"
        y="13.5"
        textAnchor="middle"
        fontFamily="Georgia, serif"
        fontSize="6"
        fontWeight="bold"
        fill="white"
      >
        €
      </text>
    </svg>
  );
}

export function Nav() {
  const pathname = usePathname();
  const { data: session } = useSession();

  return (
    <nav className="border-t-4 border-t-red-700 border-b border-b-zinc-200 bg-white shadow-sm">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between">
          <div className="flex items-center gap-8">
            <Link href="/" className="flex items-center gap-2.5">
              <LogoIcon className="h-8 w-8" />
              <div>
                <span
                  className="text-lg font-bold tracking-tight text-red-700"
                  style={{ fontFamily: "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif" }}
                >
                  VermieterMe
                </span>
                <p
                  className="hidden text-[10px] tracking-widest text-red-800/60 sm:block"
                  style={{ fontFamily: "Georgia, serif" }}
                >
                  BETRIEBSKOSTEN · MIETOBJEKTE
                </p>
              </div>
            </Link>
            <div className="hidden md:flex md:gap-1">
              {links.map((link) => {
                const isActive =
                  link.href === "/"
                    ? pathname === "/"
                    : pathname.startsWith(link.href);
                return (
                  <Link
                    key={link.href}
                    href={link.href}
                    className={`rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                      isActive
                        ? "bg-red-50 text-red-700"
                        : "text-zinc-600 hover:bg-zinc-50 hover:text-zinc-900"
                    }`}
                  >
                    {link.label}
                  </Link>
                );
              })}
            </div>
          </div>
          <div className="flex items-center gap-4">
            {session?.user && (
              <Link
                href="/profile"
                className="hidden items-center gap-2 text-sm text-zinc-600 hover:text-zinc-900 sm:flex"
              >
                <div className="flex h-7 w-7 items-center justify-center rounded-full bg-red-100 text-xs font-medium text-red-700">
                  {(session.user.name || session.user.email || "?").charAt(0).toUpperCase()}
                </div>
                <span className="max-w-[120px] truncate">
                  {session.user.name || session.user.email}
                </span>
              </Link>
            )}
            <button
              onClick={() => signOut({ callbackUrl: "/login" })}
              className="rounded-md px-3 py-1.5 text-sm text-zinc-500 hover:bg-zinc-100 hover:text-zinc-700"
            >
              Abmelden
            </button>
          </div>
        </div>
      </div>
    </nav>
  );
}
