"use client";

import { useState } from "react";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { formatDate, formatCurrency } from "@/lib/format";
import { getBillingStatus, calculateBillingTotals } from "@/lib/billing";
import { useMultiFetch } from "@/hooks/use-fetch";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type {
  PropertyWithCount,
  BillingPeriodWithProperty,
  LandlordInfo,
  VpiSuggestion,
} from "@/types";

interface DashboardData {
  properties: PropertyWithCount[];
  billingPeriods: BillingPeriodWithProperty[];
  landlord: LandlordInfo;
  vpiSuggestions: VpiSuggestion[];
}

export default function DashboardPage() {
  const [applyTarget, setApplyTarget] = useState<VpiSuggestion | null>(null);
  const [appliedIds, setAppliedIds] = useState<Set<string>>(new Set());

  const { data, loading, refetch } = useMultiFetch<DashboardData>({
    properties: "/api/properties",
    billingPeriods: "/api/billing-periods",
    landlord: "/api/settings",
    vpiSuggestions: "/api/vpi/suggestions",
  });

  const properties = data.properties ?? [];
  const billingPeriods = data.billingPeriods ?? [];
  const landlord = data.landlord;
  const vpiSuggestions = data.vpiSuggestions ?? [];

  async function applyVpiAdjustment(suggestion: VpiSuggestion) {
    setApplyTarget(null);
    try {
      // 1. Create rent change with new amount
      const res = await fetch("/api/rent-changes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          unitId: suggestion.unitId,
          type: "rent",
          amount: suggestion.suggestedRent,
          effectiveDate: new Date().toISOString().split("T")[0],
          reason: `VPI-Anpassung (${suggestion.referenceVpi} → ${suggestion.currentVpi}, ${suggestion.percentageChange > 0 ? "+" : ""}${suggestion.percentageChange.toFixed(2)}%)`,
        }),
      });

      if (res.ok) {
        // 2. Update tenant reference VPI value + date
        await fetch(`/api/tenants/${suggestion.tenantId}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            indexReferenceValue: suggestion.currentVpi,
            indexReferenceDate: new Date().toISOString().split("T")[0],
          }),
        });

        setAppliedIds((prev) => new Set(prev).add(suggestion.tenantId));
        await refetch();
      }
    } catch (error) {
      console.error("Failed to apply VPI adjustment:", error);
    }
  }

  const totalUnits = properties.reduce(
    (sum, p) => sum + (p._count?.units || 0),
    0
  );

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div className="mb-8">
          <h1 className="text-2xl font-bold text-zinc-900">Dashboard</h1>
          {landlord && (
            <p className="mt-1 text-sm text-zinc-500">
              Willkommen, {landlord.name}
            </p>
          )}
        </div>

        {loading ? (
          <Loading />
        ) : (
          <>
            {/* Summary Cards */}
            <div className="mb-8 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
              <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
                <p className="text-sm font-medium text-zinc-500">
                  Anzahl Objekte
                </p>
                <p className="mt-2 text-3xl font-bold text-zinc-900">
                  {properties.length}
                </p>
                <Link
                  href="/properties"
                  className="mt-3 inline-block text-sm text-zinc-600 hover:text-zinc-900"
                >
                  Alle anzeigen &rarr;
                </Link>
              </div>

              <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
                <p className="text-sm font-medium text-zinc-500">
                  Anzahl Wohnungen
                </p>
                <p className="mt-2 text-3xl font-bold text-zinc-900">
                  {totalUnits}
                </p>
                <Link
                  href="/properties"
                  className="mt-3 inline-block text-sm text-zinc-600 hover:text-zinc-900"
                >
                  Objekte verwalten &rarr;
                </Link>
              </div>

              <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
                <p className="text-sm font-medium text-zinc-500">
                  Aktuelle Abrechnungen
                </p>
                <p className="mt-2 text-3xl font-bold text-zinc-900">
                  {billingPeriods.length}
                </p>
                <Link
                  href="/billing"
                  className="mt-3 inline-block text-sm text-zinc-600 hover:text-zinc-900"
                >
                  Alle anzeigen &rarr;
                </Link>
              </div>
            </div>

            {/* Quick Actions */}
            <div className="mb-8">
              <h2 className="mb-4 text-lg font-semibold text-zinc-900">
                Schnellzugriff
              </h2>
              <div className="flex flex-wrap gap-3">
                <Link
                  href="/billing"
                  className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
                >
                  Neue Abrechnung erstellen
                </Link>
                <Link
                  href="/properties"
                  className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
                >
                  Objekt hinzufügen
                </Link>
                <Link
                  href="/tenants"
                  className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
                >
                  Mieter verwalten
                </Link>
              </div>
            </div>

            {/* VPI Rent Adjustment Suggestions */}
            {vpiSuggestions.length > 0 && (
              <div className="mb-8">
                <div className="mb-4 flex items-center justify-between">
                  <h2 className="text-lg font-semibold text-zinc-900">
                    Mietanpassungs-Hinweise
                  </h2>
                  <Link
                    href="/settings/vpi"
                    className="text-sm text-zinc-600 hover:text-zinc-900"
                  >
                    VPI-Werte verwalten &rarr;
                  </Link>
                </div>
                <div className="space-y-3">
                  {vpiSuggestions.map((s) => (
                    <div
                      key={s.tenantId}
                      className={`rounded-xl border border-zinc-200 bg-white p-6 shadow-sm border-l-4 ${
                        s.eligible
                          ? "border-l-green-500"
                          : "border-l-amber-400"
                      }`}
                    >
                      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                        <div className="min-w-0 flex-1">
                          <p className="font-medium text-zinc-900">
                            {s.tenantName}
                          </p>
                          <p className="mt-0.5 text-sm text-zinc-500">
                            {s.unitName} &middot; {s.propertyAddress}
                          </p>

                          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
                            <span className="text-zinc-600">
                              Miete: {formatCurrency(s.currentRent)} &rarr;{" "}
                              <span className="font-medium text-zinc-900">
                                {formatCurrency(s.suggestedRent)}
                              </span>
                            </span>
                            <span className="text-zinc-400">|</span>
                            <span className="text-zinc-600">
                              VPI: {s.referenceVpi} &rarr; {s.currentVpi} (
                              {s.percentageChange > 0 ? "+" : ""}
                              {s.percentageChange.toFixed(2)}%)
                            </span>
                            <span className="text-zinc-400">|</span>
                            <span className="text-zinc-500">
                              Basis: {s.baseYear}
                            </span>
                          </div>
                        </div>

                        <div className="flex flex-shrink-0 items-center gap-2">
                          {s.eligible ? (
                            appliedIds.has(s.tenantId) ? (
                              <span className="inline-flex rounded-full bg-green-100 px-3 py-1 text-xs font-medium text-green-700">
                                Angepasst
                              </span>
                            ) : (
                              <button
                                onClick={() => setApplyTarget(s)}
                                className="rounded-lg bg-green-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-green-700"
                              >
                                Anpassung durchf&uuml;hren
                              </button>
                            )
                          ) : (
                            <span className="inline-flex rounded-full bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700">
                              Noch {Math.max(0, s.minMonths - s.monthsSinceLastAdjustment)} Mon. Wartezeit
                            </span>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Recent Billing Periods */}
            <div>
              <h2 className="mb-4 text-lg font-semibold text-zinc-900">
                Letzte Abrechnungen
              </h2>
              {billingPeriods.length === 0 ? (
                <EmptyState message="Noch keine Abrechnungen vorhanden.">
                  <Link
                    href="/billing"
                    className="mt-2 inline-block text-sm font-medium text-zinc-900 hover:text-zinc-700"
                  >
                    Erste Abrechnung erstellen &rarr;
                  </Link>
                </EmptyState>
              ) : (
                <div className="space-y-3">
                  {billingPeriods.slice(0, 5).map((bp) => {
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
                    return (
                      <Link
                        key={bp.id}
                        href={`/billing/${bp.id}`}
                        className="block rounded-xl border border-zinc-200 bg-white p-6 shadow-sm transition-colors hover:border-zinc-300"
                      >
                        <div className="flex items-start justify-between">
                          <div>
                            <p className="font-medium text-zinc-900">
                              {bp.property?.street}, {bp.property?.city}
                            </p>
                            <p className="mt-1 text-sm text-zinc-500">
                              {formatDate(bp.startDate)} &ndash;{" "}
                              {formatDate(bp.endDate)}
                            </p>
                          </div>
                          <span
                            className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium ${status.className}`}
                          >
                            {status.label}
                          </span>
                        </div>
                        {totals && (
                          <div className="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-sm">
                            <span className="text-zinc-500">
                              Kosten:{" "}
                              <span className="font-medium text-zinc-700">
                                {formatCurrency(totals.totalUnitCosts)}
                              </span>
                            </span>
                            <span className="text-zinc-500">
                              Vorauszahlungen:{" "}
                              <span className="font-medium text-zinc-700">
                                {formatCurrency(totals.totalPrepayment)}
                              </span>
                            </span>
                            <span
                              className={`font-medium ${
                                totals.difference < 0
                                  ? "text-red-600"
                                  : "text-green-600"
                              }`}
                            >
                              {totals.difference < 0
                                ? `Nachzahlung: ${formatCurrency(Math.abs(totals.difference))}`
                                : `Erstattung: ${formatCurrency(totals.difference)}`}
                            </span>
                          </div>
                        )}
                      </Link>
                    );
                  })}
                </div>
              )}
            </div>
          </>
        )}
        <ConfirmDialog
          open={applyTarget !== null}
          onOpenChange={(open) => !open && setApplyTarget(null)}
          onConfirm={() => {
            if (applyTarget) applyVpiAdjustment(applyTarget);
          }}
          title="Mietanpassung durchführen"
          description={
            applyTarget
              ? `Kaltmiete für ${applyTarget.tenantName} von ${formatCurrency(applyTarget.currentRent)} auf ${formatCurrency(applyTarget.suggestedRent)} anpassen (VPI ${applyTarget.percentageChange > 0 ? "+" : ""}${applyTarget.percentageChange.toFixed(2)}%)?`
              : ""
          }
        />
      </main>
    </>
  );
}
