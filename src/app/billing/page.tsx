"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { formatDate } from "@/lib/format";
import { getBillingStatus } from "@/lib/billing";
import { useMultiFetch } from "@/hooks/use-fetch";
import { apiPost, apiDelete } from "@/hooks/use-api-mutation";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { OptionalDateInput } from "@/components/ui/optional-date-input";
import { EmptyState } from "@/components/ui/empty-state";
import type { Property, BillingPeriodWithProperty } from "@/types";

interface PageData {
  billingPeriods: BillingPeriodWithProperty[];
  properties: Property[];
}

export default function BillingPage() {
  const { data, loading, refetch } = useMultiFetch<PageData>({
    billingPeriods: "/api/billing-periods",
    properties: "/api/properties",
  });

  const billingPeriods = useMemo(() => data.billingPeriods ?? [], [data.billingPeriods]);
  const properties = data.properties ?? [];

  const [showNewForm, setShowNewForm] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [form, setForm] = useState({
    propertyId: "",
    startDate: "",
    endDate: "",
    billingDate: "",
  });

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!form.propertyId || !form.startDate || !form.endDate) return;
    const res = await apiPost("/api/billing-periods", {
      ...form,
      billingDate: form.billingDate || null,
    });
    if (res.ok) {
      setForm({ propertyId: "", startDate: "", endDate: "", billingDate: "" });
      setShowNewForm(false);
      await refetch();
    }
  }

  async function handleDelete(id: string) {
    const res = await apiDelete(`/api/billing-periods/${id}`);
    if (res.ok) await refetch();
  }

  const grouped = useMemo(() => {
    const groups: Record<
      string,
      { property: BillingPeriodWithProperty["property"]; periods: BillingPeriodWithProperty[] }
    > = {};
    for (const bp of billingPeriods) {
      if (!groups[bp.propertyId]) {
        groups[bp.propertyId] = { property: bp.property, periods: [] };
      }
      groups[bp.propertyId].periods.push(bp);
    }
    return Object.entries(groups);
  }, [billingPeriods]);

  const propertyOptions: ComboboxOption[] = properties.map((prop) => ({
    value: prop.id,
    label: `${prop.street}, ${prop.city}`,
  }));

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-2xl font-bold text-zinc-900">Abrechnungen</h1>
          <button
            onClick={() => {
              setShowNewForm(!showNewForm);
              setForm({ propertyId: "", startDate: "", endDate: "", billingDate: "" });
            }}
            className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
          >
            Neue Abrechnung
          </button>
        </div>

        {showNewForm && (
          <form
            onSubmit={handleCreate}
            className="mb-6 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
          >
            <h3 className="mb-4 text-lg font-semibold text-zinc-900">
              Neue Abrechnung erstellen
            </h3>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Objekt
                </label>
                <Combobox
                  options={propertyOptions}
                  value={form.propertyId}
                  onChange={(val) => setForm({ ...form, propertyId: val })}
                  placeholder="Objekt wählen..."
                  searchPlaceholder="Objekt suchen..."
                  required
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Beginn
                </label>
                <input
                  type="date"
                  required
                  value={form.startDate}
                  onChange={(e) => setForm({ ...form, startDate: e.target.value })}
                  className={`w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500 ${!form.startDate ? "text-zinc-400" : ""}`}
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Ende
                </label>
                <input
                  type="date"
                  required
                  value={form.endDate}
                  onChange={(e) => setForm({ ...form, endDate: e.target.value })}
                  className={`w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500 ${!form.endDate ? "text-zinc-400" : ""}`}
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Abrechnungsdatum
                </label>
                <OptionalDateInput
                  value={form.billingDate}
                  onChange={(v) => setForm({ ...form, billingDate: v })}
                  placeholder="Nicht gesetzt (optional)"
                />
              </div>
            </div>
            <div className="mt-4 flex gap-2">
              <button
                type="submit"
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
              >
                Erstellen
              </button>
              <button
                type="button"
                onClick={() => setShowNewForm(false)}
                className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
              >
                Abbrechen
              </button>
            </div>
          </form>
        )}

        {loading ? (
          <Loading />
        ) : billingPeriods.length === 0 ? (
          <EmptyState message="Noch keine Abrechnungen vorhanden. Erstellen Sie Ihre erste Abrechnung." />
        ) : (
          <div className="space-y-8">
            {grouped.map(([propertyId, { property, periods }]) => (
              <section key={propertyId}>
                <div className="mb-3 flex items-baseline gap-2">
                  <h2 className="text-lg font-semibold text-zinc-900">
                    {property.street}
                  </h2>
                  <span className="text-sm text-zinc-500">
                    {property.zip} {property.city}
                  </span>
                </div>
                <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                  {periods.map((bp) => {
                    const status = getBillingStatus(bp);
                    return (
                      <div
                        key={bp.id}
                        className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
                      >
                        <div className="mb-4 flex items-start justify-between">
                          <h3 className="font-semibold text-zinc-900">
                            {formatDate(bp.startDate)} &ndash;{" "}
                            {formatDate(bp.endDate)}
                          </h3>
                          <span
                            className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium ${status.className}`}
                          >
                            {status.label}
                          </span>
                        </div>

                        <div className="mb-4 space-y-1 text-sm text-zinc-600">
                          {bp.billingDate && (
                            <p>
                              <span className="text-zinc-500">Abrechnungsdatum:</span>{" "}
                              {formatDate(bp.billingDate)}
                            </p>
                          )}
                          {bp._count && bp._count.costs > 0 && (
                            <p>
                              <span className="text-zinc-500">Kostenarten:</span>{" "}
                              {bp._count.costs}
                            </p>
                          )}
                        </div>

                        <div className="flex gap-2">
                          <Link
                            href={`/billing/${bp.id}`}
                            className="rounded-lg bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-800"
                          >
                            Bearbeiten
                          </Link>
                          <button
                            onClick={() => setDeleteTarget(bp.id)}
                            className="rounded-lg border border-red-200 px-3 py-1.5 text-xs font-medium text-red-600 hover:bg-red-50"
                          >
                            Löschen
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </section>
            ))}
          </div>
        )}
        <ConfirmDialog
          open={deleteTarget !== null}
          onOpenChange={(open) => !open && setDeleteTarget(null)}
          onConfirm={() => {
            if (deleteTarget) handleDelete(deleteTarget);
          }}
          title="Abrechnung löschen"
          description="Möchten Sie diese Abrechnung wirklich löschen?"
        />
      </main>
    </>
  );
}
