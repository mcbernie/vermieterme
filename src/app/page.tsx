"use client";

import Link from "next/link";
import { Nav } from "@/components/nav";
import { formatDate } from "@/lib/format";
import { getBillingStatus } from "@/lib/billing";
import { useMultiFetch } from "@/hooks/use-fetch";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import type {
  PropertyWithCount,
  BillingPeriodWithProperty,
  LandlordInfo,
} from "@/types";

interface DashboardData {
  properties: PropertyWithCount[];
  billingPeriods: BillingPeriodWithProperty[];
  landlord: LandlordInfo;
}

export default function DashboardPage() {
  const { data, loading } = useMultiFetch<DashboardData>({
    properties: "/api/properties",
    billingPeriods: "/api/billing-periods",
    landlord: "/api/settings",
  });

  const properties = data.properties ?? [];
  const billingPeriods = data.billingPeriods ?? [];
  const landlord = data.landlord;

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
                    return (
                      <Link
                        key={bp.id}
                        href={`/billing/${bp.id}`}
                        className="block rounded-xl border border-zinc-200 bg-white p-6 shadow-sm transition-colors hover:border-zinc-300"
                      >
                        <div className="flex items-center justify-between">
                          <div>
                            <p className="font-medium text-zinc-900">
                              {bp.property?.street}, {bp.property?.city}
                            </p>
                            <p className="mt-1 text-sm text-zinc-500">
                              {formatDate(bp.startDate)} &ndash;{" "}
                              {formatDate(bp.endDate)}
                            </p>
                          </div>
                          <div className="flex items-center gap-3">
                            {bp.billingDate && (
                              <span className="text-sm text-zinc-500">
                                Abrechnungsdatum: {formatDate(bp.billingDate)}
                              </span>
                            )}
                            <span
                              className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium ${status.className}`}
                            >
                              {status.label}
                            </span>
                          </div>
                        </div>
                      </Link>
                    );
                  })}
                </div>
              )}
            </div>
          </>
        )}
      </main>
    </>
  );
}
