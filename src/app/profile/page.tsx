"use client";

import { useEffect, useState } from "react";
import { useSession } from "next-auth/react";
import { Nav } from "@/components/nav";
import { Loading } from "@/components/ui/loading";
import { PASSWORD_MIN_LENGTH } from "@/lib/constants";
import type { AppUser } from "@/types";

export default function ProfilePage() {
  const { update: updateSession } = useSession();
  const [profile, setProfile] = useState<AppUser | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  const [name, setName] = useState("");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  useEffect(() => {
    fetchProfile();
  }, []);

  async function fetchProfile() {
    try {
      const res = await fetch("/api/profile");
      if (res.ok) {
        const data = await res.json();
        setProfile(data);
        setName(data.name || "");
      }
    } catch (error) {
      console.error("Failed to fetch profile:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSaveName(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setStatus(null);

    try {
      const res = await fetch("/api/profile", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      });

      if (res.ok) {
        setStatus({ type: "success", message: "Name gespeichert" });
        await updateSession({ name });
        await fetchProfile();
      } else {
        const data = await res.json();
        setStatus({
          type: "error",
          message: data.error || "Fehler beim Speichern",
        });
      }
    } catch {
      setStatus({ type: "error", message: "Fehler beim Speichern" });
    } finally {
      setSaving(false);
      setTimeout(() => setStatus(null), 3000);
    }
  }

  async function handleChangePassword(e: React.FormEvent) {
    e.preventDefault();
    if (newPassword !== confirmPassword) {
      setStatus({
        type: "error",
        message: "Passwörter stimmen nicht überein",
      });
      return;
    }
    if (newPassword.length < PASSWORD_MIN_LENGTH) {
      setStatus({
        type: "error",
        message: `Passwort muss mindestens ${PASSWORD_MIN_LENGTH} Zeichen lang sein`,
      });
      return;
    }

    setSaving(true);
    setStatus(null);

    try {
      const res = await fetch("/api/profile", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ currentPassword, newPassword }),
      });

      if (res.ok) {
        setStatus({ type: "success", message: "Passwort geändert" });
        setCurrentPassword("");
        setNewPassword("");
        setConfirmPassword("");
      } else {
        const data = await res.json();
        setStatus({
          type: "error",
          message: data.error || "Fehler beim Ändern des Passworts",
        });
      }
    } catch {
      setStatus({ type: "error", message: "Fehler beim Speichern" });
    } finally {
      setSaving(false);
      setTimeout(() => setStatus(null), 3000);
    }
  }

  if (loading) {
    return (
      <>
        <Nav />
        <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <Loading />
        </main>
      </>
    );
  }

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-3xl px-4 py-8 sm:px-6 lg:px-8">
        <h1 className="mb-8 text-2xl font-bold text-zinc-900">Mein Profil</h1>

        {status && (
          <div
            className={`mb-6 rounded-lg px-4 py-3 text-sm ${
              status.type === "success"
                ? "bg-green-50 text-green-700"
                : "bg-red-50 text-red-700"
            }`}
          >
            {status.message}
          </div>
        )}

        {/* Profile Info */}
        <section className="mb-8">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Persönliche Daten
          </h2>
          <form
            onSubmit={handleSaveName}
            className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
          >
            <div className="mb-6 flex items-center gap-4">
              <div className="flex h-14 w-14 items-center justify-center rounded-full bg-red-100 text-xl font-semibold text-red-700">
                {(profile?.name || profile?.email || "?")
                  .charAt(0)
                  .toUpperCase()}
              </div>
              <div>
                <p className="font-medium text-zinc-900">
                  {profile?.name || "Kein Name"}
                </p>
                <p className="text-sm text-zinc-500">{profile?.email}</p>
              </div>
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Name
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-red-500 focus:outline-none focus:ring-1 focus:ring-red-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  E-Mail
                </label>
                <input
                  type="email"
                  value={profile?.email || ""}
                  disabled
                  className="w-full rounded-lg border border-zinc-200 bg-zinc-50 px-3 py-2 text-sm text-zinc-500"
                />
              </div>
            </div>

            <div className="mt-4">
              <button
                type="submit"
                disabled={saving}
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
              >
                {saving ? "Speichern..." : "Speichern"}
              </button>
            </div>
          </form>
        </section>

        {/* Change Password */}
        <section>
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Passwort ändern
          </h2>
          <form
            onSubmit={handleChangePassword}
            className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
          >
            <div className="grid gap-4">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Aktuelles Passwort
                </label>
                <input
                  type="password"
                  required
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  autoComplete="current-password"
                  className="w-full max-w-sm rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-red-500 focus:outline-none focus:ring-1 focus:ring-red-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Neues Passwort
                </label>
                <input
                  type="password"
                  required
                  minLength={PASSWORD_MIN_LENGTH}
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  autoComplete="new-password"
                  className="w-full max-w-sm rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-red-500 focus:outline-none focus:ring-1 focus:ring-red-500"
                />
                <p className="mt-1 text-xs text-zinc-400">
                  Mindestens 8 Zeichen
                </p>
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Neues Passwort bestätigen
                </label>
                <input
                  type="password"
                  required
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  autoComplete="new-password"
                  className="w-full max-w-sm rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-red-500 focus:outline-none focus:ring-1 focus:ring-red-500"
                />
              </div>
            </div>

            <div className="mt-4">
              <button
                type="submit"
                disabled={saving}
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
              >
                {saving ? "Ändern..." : "Passwort ändern"}
              </button>
            </div>
          </form>
        </section>
      </main>
    </>
  );
}
