"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import type { VpiEntry } from "@/types";

const MONTH_NAMES = [
  "Januar",
  "Februar",
  "März",
  "April",
  "Mai",
  "Juni",
  "Juli",
  "August",
  "September",
  "Oktober",
  "November",
  "Dezember",
];

const MONTH_OPTIONS: ComboboxOption[] = MONTH_NAMES.map((name, idx) => ({
  value: String(idx + 1),
  label: name,
}));

export default function VpiPage() {
  const [entries, setEntries] = useState<VpiEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [deleteEntryId, setDeleteEntryId] = useState<string | null>(null);
  const [fetching, setFetching] = useState(false);
  const [fetchBaseYear, setFetchBaseYear] = useState("2020");
  const [fetchError, setFetchError] = useState<string | null>(null);

  const [form, setForm] = useState({
    baseYear: "",
    year: "",
    month: "",
    value: "",
  });

  useEffect(() => {
    fetchEntries();
  }, []);

  async function fetchEntries() {
    try {
      const res = await fetch("/api/vpi");
      if (res.ok) {
        setEntries(await res.json());
      }
    } catch (error) {
      console.error("Failed to fetch VPI entries:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setStatus(null);

    try {
      const res = await fetch("/api/vpi", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          baseYear: parseInt(form.baseYear),
          year: parseInt(form.year),
          month: parseInt(form.month),
          value: parseFloat(form.value),
        }),
      });

      if (res.ok) {
        setForm({ baseYear: form.baseYear, year: form.year, month: "", value: "" });
        setStatus("Gespeichert");
        setTimeout(() => setStatus(null), 2000);
        await fetchEntries();
      } else {
        setStatus("Fehler beim Speichern");
      }
    } catch (error) {
      console.error("Failed to save VPI entry:", error);
      setStatus("Fehler beim Speichern");
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(id: string) {
    try {
      const res = await fetch(`/api/vpi/${id}`, { method: "DELETE" });
      if (res.ok) {
        await fetchEntries();
      }
    } catch (error) {
      console.error("Failed to delete VPI entry:", error);
    }
  }

  async function handleAutoFetch() {
    setFetching(true);
    setFetchError(null);

    try {
      const res = await fetch("/api/vpi/fetch", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ baseYear: parseInt(fetchBaseYear) }),
      });

      const data = await res.json();

      if (!res.ok) {
        setFetchError(data.error || "Fehler beim Abrufen");
        return;
      }

      // Save each fetched entry
      let savedCount = 0;
      for (const entry of data.entries) {
        const saveRes = await fetch("/api/vpi", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            baseYear: parseInt(fetchBaseYear),
            year: entry.year,
            month: entry.month,
            value: entry.value,
          }),
        });
        if (saveRes.ok) savedCount++;
      }

      setStatus(`${savedCount} VPI-Werte erfolgreich importiert`);
      setTimeout(() => setStatus(null), 4000);
      await fetchEntries();
    } catch (error) {
      console.error("Auto-fetch failed:", error);
      setFetchError("Fehler beim Abrufen der VPI-Daten");
    } finally {
      setFetching(false);
    }
  }

  // Group entries by baseYear, sorted by baseYear desc
  const grouped = entries.reduce<Record<number, VpiEntry[]>>((acc, entry) => {
    if (!acc[entry.baseYear]) {
      acc[entry.baseYear] = [];
    }
    acc[entry.baseYear].push(entry);
    return acc;
  }, {});

  const sortedBaseYears = Object.keys(grouped)
    .map(Number)
    .sort((a, b) => b - a);

  // Sort entries within each group by year desc, month desc
  for (const baseYear of sortedBaseYears) {
    grouped[baseYear].sort((a, b) => {
      if (b.year !== a.year) return b.year - a.year;
      return b.month - a.month;
    });
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
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        {/* Breadcrumb & Header */}
        <div className="mb-8">
          <p className="mb-1 text-sm text-zinc-500">
            <Link href="/settings" className="hover:text-zinc-700 hover:underline">
              Einstellungen
            </Link>
            {" / "}
            <span className="text-zinc-700">VPI-Verwaltung</span>
          </p>
          <h1 className="text-2xl font-bold text-zinc-900">VPI-Verwaltung</h1>
        </div>

        {/* Info Box */}
        <div className="mb-8 rounded-xl border border-blue-200 bg-blue-50 p-4 text-sm text-blue-800">
          Der Verbraucherpreisindex (VPI) wird vom Statistischen Bundesamt
          veröffentlicht und dient als Grundlage für Indexmietanpassungen.
          Tragen Sie hier die VPI-Werte ein, die Sie für Ihre Mietverträge
          benötigen.
        </div>

        {/* Auto-Fetch */}
        <section className="mb-8">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            VPI automatisch abrufen
          </h2>
          <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
            <p className="mb-4 text-sm text-zinc-600">
              VPI-Werte automatisch von der Deutschen Bundesbank laden.
              Unterstützte Basisjahre: 2010, 2015, 2020.
            </p>
            <div className="flex flex-wrap items-end gap-3">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Basisjahr
                </label>
                <input
                  type="number"
                  value={fetchBaseYear}
                  onChange={(e) => setFetchBaseYear(e.target.value)}
                  className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <button
                onClick={handleAutoFetch}
                disabled={fetching}
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
              >
                {fetching ? "Lade..." : "Von Bundesbank laden"}
              </button>
            </div>
            {fetchError && (
              <p className="mt-3 text-sm text-red-600">{fetchError}</p>
            )}
            {status && (
              <p className="mt-3 text-sm text-green-600">{status}</p>
            )}
          </div>
        </section>

        {/* Add/Update Form */}
        <section className="mb-10">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            VPI-Eintrag hinzufügen
          </h2>
          <form
            onSubmit={handleSubmit}
            className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
          >
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Basisjahr
                </label>
                <input
                  type="number"
                  required
                  value={form.baseYear}
                  onChange={(e) =>
                    setForm({ ...form, baseYear: e.target.value })
                  }
                  placeholder="z.B. 2020"
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Jahr
                </label>
                <input
                  type="number"
                  required
                  value={form.year}
                  onChange={(e) =>
                    setForm({ ...form, year: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Monat
                </label>
                <Combobox
                  options={MONTH_OPTIONS}
                  value={form.month}
                  onChange={(val) => setForm({ ...form, month: val })}
                  placeholder="Monat wählen..."
                  required
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  VPI-Wert
                </label>
                <input
                  type="number"
                  step="0.1"
                  required
                  value={form.value}
                  onChange={(e) =>
                    setForm({ ...form, value: e.target.value })
                  }
                  placeholder="z.B. 117.4"
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
            </div>
            <div className="mt-4 flex items-center gap-3">
              <button
                type="submit"
                disabled={saving}
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
              >
                {saving ? "Speichern..." : "Speichern"}
              </button>
              {status && (
                <span
                  className={`text-sm ${
                    status === "Gespeichert"
                      ? "text-green-600"
                      : "text-red-600"
                  }`}
                >
                  {status}
                </span>
              )}
            </div>
          </form>
        </section>

        {/* Entries Table */}
        <section>
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Vorhandene VPI-Einträge
          </h2>

          {entries.length === 0 ? (
            <EmptyState message="Noch keine VPI-Einträge vorhanden. Fügen Sie Ihren ersten Eintrag über das Formular oben hinzu." />
          ) : (
            <div className="space-y-6">
              {sortedBaseYears.map((baseYear) => (
                <div key={baseYear}>
                  <div className="mb-2 flex items-center gap-2">
                    <h3 className="text-sm font-semibold text-zinc-700">
                      Basisjahr: {baseYear}
                    </h3>
                    <span className="inline-flex rounded-full bg-zinc-100 px-2.5 py-0.5 text-xs font-medium text-zinc-600">
                      {grouped[baseYear].length}{" "}
                      {grouped[baseYear].length === 1 ? "Eintrag" : "Einträge"}
                    </span>
                  </div>
                  <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white shadow-sm">
                    <table className="w-full">
                      <thead>
                        <tr className="border-b border-zinc-200 bg-zinc-50">
                          <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                            Monat/Jahr
                          </th>
                          <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                            VPI-Wert
                          </th>
                          <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                            Aktionen
                          </th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-zinc-200">
                        {grouped[baseYear].map((entry) => (
                          <tr key={entry.id}>
                            <td className="px-4 py-3 text-sm text-zinc-900">
                              {MONTH_NAMES[entry.month - 1]} {entry.year}
                            </td>
                            <td className="px-4 py-3 text-sm text-zinc-900">
                              {entry.value}
                            </td>
                            <td className="px-4 py-3 text-right">
                              <button
                                onClick={() => setDeleteEntryId(entry.id)}
                                className="rounded-lg border border-red-200 px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-50"
                              >
                                Löschen
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>

        <ConfirmDialog
          open={deleteEntryId !== null}
          onOpenChange={(open) => !open && setDeleteEntryId(null)}
          onConfirm={() => {
            if (deleteEntryId) handleDelete(deleteEntryId);
          }}
          title="VPI-Eintrag löschen"
          description="Möchten Sie diesen VPI-Eintrag wirklich löschen?"
        />
      </main>
    </>
  );
}
