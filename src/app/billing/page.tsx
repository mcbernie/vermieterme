"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { formatDate, formatCurrency } from "@/lib/format";
import { getBillingStatus, calculateBillingTotals, getUnreviewedCount } from "@/lib/billing";
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
  const properties = useMemo(() => data.properties ?? [], [data.properties]);

  const [showNewForm, setShowNewForm] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [createError, setCreateError] = useState<string | null>(null);
  const [form, setForm] = useState({
    propertyId: "",
    startDate: "",
    endDate: "",
    billingDate: "",
    copyFromId: "",
  });

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setCreateError(null);
    if (!form.propertyId || !form.startDate || !form.endDate) return;
    const res = await apiPost("/api/billing-periods", {
      ...form,
      billingDate: form.billingDate || null,
      copyFromId: form.copyFromId || null,
    });
    if (res.ok) {
      setForm({ propertyId: "", startDate: "", endDate: "", billingDate: "", copyFromId: "" });
      setShowNewForm(false);
      await refetch();
    } else {
      const data = await res.json().catch(() => null);
      setCreateError(data?.error || "Fehler beim Erstellen der Abrechnung");
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

  // Determine which properties need a new billing period for the previous year
  const previousYearSuggestions = useMemo(() => {
    const currentYear = new Date().getFullYear();
    const previousYear = currentYear - 1;
    const suggestions: Array<{
      property: Property;
      previousYear: number;
      sourcePeriod: BillingPeriodWithProperty;
    }> = [];

    for (const prop of properties) {
      const propPeriods = billingPeriods.filter((bp) => bp.propertyId === prop.id);

      // Check if a period covering the previous year already exists
      const hasPreviousYear = propPeriods.some((bp) => {
        const startYear = new Date(bp.startDate).getFullYear();
        const endYear = new Date(bp.endDate).getFullYear();
        return startYear === previousYear || endYear === previousYear;
      });

      if (hasPreviousYear) continue;

      // Find the most recent period to copy from (likely the year before)
      const sorted = propPeriods.sort(
        (a, b) => new Date(b.startDate).getTime() - new Date(a.startDate).getTime()
      );

      if (sorted.length > 0) {
        suggestions.push({
          property: prop,
          previousYear,
          sourcePeriod: sorted[0],
        });
      }
    }

    return suggestions;
  }, [properties, billingPeriods]);

  async function handleQuickCreate(propertyId: string, sourceId: string, previousYear: number) {
    setCreateError(null);
    const res = await apiPost("/api/billing-periods", {
      propertyId,
      startDate: `${previousYear}-01-01`,
      endDate: `${previousYear}-12-31`,
      billingDate: null,
      copyFromId: sourceId,
    });
    if (res.ok) {
      await refetch();
    } else {
      const data = await res.json().catch(() => null);
      setCreateError(data?.error || "Fehler beim Erstellen");
    }
  }

  const propertyOptions: ComboboxOption[] = properties.map((prop) => ({
    value: prop.id,
    label: `${prop.street}, ${prop.city}`,
  }));

  const previousPeriodOptions: ComboboxOption[] = useMemo(() => {
    if (!form.propertyId) return [];
    return billingPeriods
      .filter((bp) => bp.propertyId === form.propertyId)
      .map((bp) => ({
        value: bp.id,
        label: `${formatDate(bp.startDate)} – ${formatDate(bp.endDate)}`,
      }));
  }, [form.propertyId, billingPeriods]);

  function handleCopyFrom(copyFromId: string) {
    const source = billingPeriods.find((bp) => bp.id === copyFromId);
    if (!source) {
      setForm((prev) => ({ ...prev, copyFromId }));
      return;
    }
    const shiftYear = (dateStr: string) => {
      const d = new Date(dateStr);
      d.setFullYear(d.getFullYear() + 1);
      return d.toISOString().split("T")[0];
    };
    setForm((prev) => ({
      ...prev,
      copyFromId,
      startDate: shiftYear(source.startDate),
      endDate: shiftYear(source.endDate),
    }));
  }

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-2xl font-bold text-zinc-900">Abrechnungen</h1>
          <button
            onClick={() => {
              setShowNewForm(!showNewForm);
              setForm({ propertyId: "", startDate: "", endDate: "", billingDate: "", copyFromId: "" });
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
              {form.propertyId && previousPeriodOptions.length > 0 && (
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Vorjahr übernehmen
                  </label>
                  <Combobox
                    options={previousPeriodOptions}
                    value={form.copyFromId}
                    onChange={handleCopyFrom}
                    placeholder="Vorjahr wählen (optional)..."
                    searchPlaceholder="Abrechnung suchen..."
                  />
                </div>
              )}
            </div>
            {createError && (
              <div className="mt-4 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
                {createError}
              </div>
            )}
            <div className="mt-4 flex gap-2">
              <button
                type="submit"
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
              >
                Erstellen
              </button>
              <button
                type="button"
                onClick={() => { setShowNewForm(false); setCreateError(null); }}
                className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
              >
                Abbrechen
              </button>
            </div>
          </form>
        )}

        {/* Vorjahres-Vorschläge */}
        {!loading && previousYearSuggestions.length > 0 && (
          <div className="mb-6 space-y-3">
            {previousYearSuggestions.map((s) => (
              <div
                key={s.property.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-zinc-200 bg-zinc-50 p-4"
              >
                <div className="text-sm text-zinc-700">
                  <span className="font-medium">{s.property.street}, {s.property.city}</span>
                  {" — "}
                  Noch keine Abrechnung f&uuml;r {s.previousYear} vorhanden.
                  Daten aus der letzten Abrechnung ({formatDate(s.sourcePeriod.startDate)} &ndash; {formatDate(s.sourcePeriod.endDate)}) &uuml;bernehmen?
                </div>
                <button
                  onClick={() => handleQuickCreate(s.property.id, s.sourcePeriod.id, s.previousYear)}
                  className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
                >
                  Abrechnung {s.previousYear} erstellen
                </button>
              </div>
            ))}
          </div>
        )}

        {createError && !showNewForm && (
          <div className="mb-6 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
            {createError}
          </div>
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
                    const hasCosts = bp.costs && bp.costs.length > 0;
                    const totals = hasCosts
                      ? calculateBillingTotals(
                          bp.costs!,
                          bp.prepayments ?? [],
                          bp.startDate,
                          bp.endDate
                        )
                      : null;
                    const unreviewedCount = getUnreviewedCount(
                      bp.costs ?? [],
                      bp.prepayments ?? []
                    );
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
                          <div className="flex flex-col items-end gap-1">
                            <span
                              className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium ${status.className}`}
                            >
                              {status.label}
                            </span>
                            {unreviewedCount > 0 && (
                              <span className="inline-flex rounded-full bg-amber-100 px-2.5 py-0.5 text-xs font-medium text-amber-700">
                                {unreviewedCount} Positionen ungeprüft
                              </span>
                            )}
                          </div>
                        </div>

                        <div className="mb-4 space-y-1 text-sm text-zinc-600">
                          {bp.billingDate && (
                            <p>
                              <span className="text-zinc-500">Abrechnungsdatum:</span>{" "}
                              {formatDate(bp.billingDate)}
                            </p>
                          )}
                          {bp.sentDate && (
                            <p>
                              <span className="text-zinc-500">Versendet:</span>{" "}
                              {formatDate(bp.sentDate)}
                            </p>
                          )}
                          {bp.paidDate && (
                            <p>
                              <span className="text-zinc-500">Bezahlt:</span>{" "}
                              {formatDate(bp.paidDate)}
                            </p>
                          )}
                          {bp._count && bp._count.costs > 0 && (
                            <p>
                              <span className="text-zinc-500">Kostenarten:</span>{" "}
                              {bp._count.costs}
                            </p>
                          )}
                        </div>

                        {totals && (
                          <div className="mb-4 space-y-1 rounded-lg bg-zinc-50 p-3 text-sm">
                            <p className="text-zinc-600">
                              <span className="text-zinc-500">Kosten gesamt:</span>{" "}
                              {formatCurrency(totals.totalCosts)}
                            </p>
                            <p className="text-zinc-600">
                              <span className="text-zinc-500">Ihr Anteil:</span>{" "}
                              {formatCurrency(totals.totalUnitCosts)}
                            </p>
                            <p className="text-zinc-600">
                              <span className="text-zinc-500">Vorauszahlungen:</span>{" "}
                              {formatCurrency(totals.totalPrepayment)}
                            </p>
                            <p
                              className={`font-medium ${
                                totals.difference < 0
                                  ? "text-red-600"
                                  : "text-green-600"
                              }`}
                            >
                              <span
                                className={
                                  totals.difference < 0
                                    ? "text-red-500"
                                    : "text-green-500"
                                }
                              >
                                {totals.difference < 0
                                  ? "Nachzahlung:"
                                  : "Erstattung:"}
                              </span>{" "}
                              {formatCurrency(Math.abs(totals.difference))}
                            </p>
                          </div>
                        )}

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
