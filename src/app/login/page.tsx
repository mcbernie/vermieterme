"use client";

import { signIn, getProviders, useSession } from "next-auth/react";
import { useRouter, useSearchParams } from "next/navigation";
import { useState, useEffect, Suspense } from "react";

function LogoIcon({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      className={className}
    >
      <defs>
        <linearGradient id="loginLogoBg" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#b91c1c" />
          <stop offset="100%" stopColor="#7f1d1d" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="6" fill="url(#loginLogoBg)" />
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

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [hasApple, setHasApple] = useState(false);
  const { status } = useSession();
  const callbackUrl = searchParams.get("callbackUrl") || "/";

  useEffect(() => {
    if (status === "authenticated") {
      router.push(callbackUrl);
    }
  }, [status, callbackUrl, router]);

  useEffect(() => {
    getProviders().then((providers) => {
      if (providers?.apple) setHasApple(true);
    });
  }, []);

  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    setError("");

    const formData = new FormData(e.currentTarget);
    const result = await signIn("credentials", {
      email: formData.get("email"),
      password: formData.get("password"),
      redirect: false,
    });

    if (result?.error) {
      setError("Ungültige Anmeldedaten");
      setLoading(false);
    } else {
      router.push(callbackUrl);
      router.refresh();
    }
  }

  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-zinc-50">
      <div className="w-full max-w-sm overflow-hidden rounded-xl border border-zinc-200 bg-white shadow-sm">
        <div className="border-t-4 border-t-red-700" />
        <div className="p-8">
          <div className="mb-8 flex flex-col items-center gap-3">
            <LogoIcon className="h-12 w-12" />
            <div className="text-center">
              <h1
                className="text-2xl font-bold text-red-800"
                style={{
                  fontFamily:
                    "'Palatino Linotype', 'Book Antiqua', Palatino, Georgia, serif",
                }}
              >
                VermieterMe
              </h1>
              <p
                className="mt-1 text-[10px] tracking-[0.2em] text-red-800/50"
                style={{ fontFamily: "Georgia, serif" }}
              >
                BETRIEBSKOSTEN · MIETOBJEKTE
              </p>
            </div>
          </div>

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label
                htmlFor="email"
                className="block text-sm font-medium text-zinc-700"
              >
                E-Mail
              </label>
              <input
                id="email"
                name="email"
                type="email"
                required
                autoComplete="email"
                className="mt-1 block w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-red-500 focus:outline-none focus:ring-1 focus:ring-red-500"
              />
            </div>
            <div>
              <label
                htmlFor="password"
                className="block text-sm font-medium text-zinc-700"
              >
                Passwort
              </label>
              <input
                id="password"
                name="password"
                type="password"
                required
                autoComplete="current-password"
                className="mt-1 block w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-red-500 focus:outline-none focus:ring-1 focus:ring-red-500"
              />
            </div>

            {error && <p className="text-sm text-red-600">{error}</p>}

            <button
              type="submit"
              disabled={loading}
              className="w-full rounded-lg bg-red-700 py-2.5 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
            >
              {loading ? "Anmelden..." : "Anmelden"}
            </button>
          </form>

          {hasApple && (
            <>
              <div className="relative my-6">
                <div className="absolute inset-0 flex items-center">
                  <div className="w-full border-t border-zinc-200" />
                </div>
                <div className="relative flex justify-center text-xs">
                  <span className="bg-white px-2 text-zinc-400">oder</span>
                </div>
              </div>

              <button
                onClick={() => signIn("apple", { callbackUrl })}
                className="flex w-full items-center justify-center gap-2 rounded-lg border border-zinc-300 py-2.5 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
              >
                <svg
                  className="h-5 w-5"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path d="M17.05 20.28c-.98.95-2.05.88-3.08.4-1.09-.5-2.08-.48-3.24 0-1.44.62-2.2.44-3.06-.4C2.79 15.25 3.51 7.59 9.05 7.31c1.35.07 2.29.74 3.08.8 1.18-.24 2.31-.93 3.57-.84 1.51.12 2.65.72 3.4 1.8-3.12 1.87-2.38 5.98.48 7.13-.57 1.5-1.31 2.99-2.54 4.09zM12.03 7.25c-.15-2.23 1.66-4.07 3.74-4.25.29 2.58-2.34 4.5-3.74 4.25z" />
                </svg>
                Mit Apple anmelden
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

export default function LoginPage() {
  return (
    <Suspense>
      <LoginForm />
    </Suspense>
  );
}
