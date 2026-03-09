"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { formatCurrency, formatDate } from "@/lib/format";
import { calculateMEAAmount, getMonthsInPeriod, getDaysInPeriod } from "@/lib/billing";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import type { BillingPeriodDetail, CostCategory, Tenant, UnitWithTenants } from "@/types";

export default function BillingDetailPage() {
  const params = useParams();
  const id = params.id as string;

  const [billingPeriod, setBillingPeriod] = useState<BillingPeriodDetail | null>(
    null
  );
  const [costCategories, setCostCategories] = useState<CostCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<string | null>(null);

  const [costValues, setCostValues] = useState<
    Record<string, { totalAmount: string; unitAmount: string }>
  >({});

  const [prepaymentValues, setPrepaymentValues] = useState<
    Record<string, string>
  >({});

  const debounceRef = useRef<NodeJS.Timeout | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const [bpRes, ccRes] = await Promise.all([
        fetch(`/api/billing-periods/${id}`),
        fetch("/api/cost-categories"),
      ]);

      if (bpRes.ok) {
        const bp: BillingPeriodDetail = await bpRes.json();
        setBillingPeriod(bp);

        const costs: Record<
          string,
          { totalAmount: string; unitAmount: string }
        > = {};
        bp.costs.forEach((cost) => {
          costs[cost.costCategoryId] = {
            totalAmount: cost.totalAmount.toString(),
            unitAmount: cost.unitAmount?.toString() || "",
          };
        });
        setCostValues(costs);

        const prepayments: Record<string, string> = {};
        bp.prepayments.forEach((pp) => {
          prepayments[pp.unitId] = pp.monthlyAmount.toString();
        });
        setPrepaymentValues(prepayments);
      }

      if (ccRes.ok) {
        setCostCategories(await ccRes.json());
      }
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleSaveRef = useRef(handleSave);
  handleSaveRef.current = handleSave;

  const triggerAutoSave = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      handleSaveRef.current();
    }, 1500);
  }, []);

  function handleCostChange(
    categoryId: string,
    field: "totalAmount" | "unitAmount",
    value: string
  ) {
    setCostValues((prev) => ({
      ...prev,
      [categoryId]: {
        totalAmount: prev[categoryId]?.totalAmount || "",
        unitAmount: prev[categoryId]?.unitAmount || "",
        [field]: value,
      },
    }));
    triggerAutoSave();
  }

  function handlePrepaymentChange(unitId: string, value: string) {
    setPrepaymentValues((prev) => ({
      ...prev,
      [unitId]: value,
    }));
    triggerAutoSave();
  }

  async function handleSave() {
    if (!billingPeriod) return;
    setSaving(true);
    setSaveStatus(null);

    try {
      const costPromises = Object.entries(costValues)
        .filter(([, val]) => val.totalAmount !== "")
        .map(([categoryId, val]) =>
          fetch(`/api/billing-periods/${id}/costs`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              costCategoryId: categoryId,
              totalAmount: parseFloat(val.totalAmount) || 0,
              unitAmount: val.unitAmount ? parseFloat(val.unitAmount) : null,
            }),
          })
        );

      const prepaymentPromises = Object.entries(prepaymentValues)
        .filter(([, val]) => val !== "")
        .map(([unitId, val]) =>
          fetch(`/api/billing-periods/${id}/prepayments`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              unitId,
              monthlyAmount: parseFloat(val) || 0,
            }),
          })
        );

      const results = await Promise.all([
        ...costPromises,
        ...prepaymentPromises,
      ]);
      const allOk = results.every((r) => r.ok);

      if (allOk) {
        setSaveStatus("Gespeichert");
        setTimeout(() => setSaveStatus(null), 2000);
      } else {
        setSaveStatus("Fehler beim Speichern");
      }
    } catch (error) {
      console.error("Failed to save:", error);
      setSaveStatus("Fehler beim Speichern");
    } finally {
      setSaving(false);
    }
  }

  function calculateUnitCosts(unit: UnitWithTenants): number {
    let total = 0;
    costCategories.forEach((cat) => {
      const costVal = costValues[cat.id];
      if (!costVal || !costVal.totalAmount) return;
      const totalAmount = parseFloat(costVal.totalAmount) || 0;

      if (cat.distributionKey === "MEA") {
        total += calculateMEAAmount(totalAmount, unit.shares, billingPeriod!.property.totalShares);
      } else {
        const unitAmount = costVal.unitAmount
          ? parseFloat(costVal.unitAmount)
          : 0;
        total += unitAmount;
      }
    });
    return total;
  }

  function calculateUnitPrepayment(unitId: string): number {
    const monthly = parseFloat(prepaymentValues[unitId] || "0") || 0;
    return monthly * getMonthsInPeriod(billingPeriod!.startDate, billingPeriod!.endDate);
  }

  function getCurrentTenant(unit: UnitWithTenants): Tenant | undefined {
    return unit.tenants?.find((t) => !t.moveOutDate);
  }

  function getTotalCosts(): number {
    if (!billingPeriod) return 0;
    return billingPeriod.property.units.reduce(
      (sum, unit) => sum + calculateUnitCosts(unit),
      0
    );
  }

  function getTotalPrepayments(): number {
    if (!billingPeriod) return 0;
    return billingPeriod.property.units.reduce(
      (sum, unit) => sum + calculateUnitPrepayment(unit.id),
      0
    );
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

  if (!billingPeriod) {
    return (
      <>
        <Nav />
        <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <EmptyState message="Abrechnung nicht gefunden.">
            <Link
              href="/billing"
              className="mt-2 inline-block text-sm font-medium text-zinc-900 hover:text-zinc-700"
            >
              Zurück zur Übersicht
            </Link>
          </EmptyState>
        </main>
      </>
    );
  }

  const property = billingPeriod.property;
  const units = property.units || [];
  const monthsInPeriod = getMonthsInPeriod(billingPeriod.startDate, billingPeriod.endDate);
  const daysInPeriod = getDaysInPeriod(billingPeriod.startDate, billingPeriod.endDate);

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        {/* Header */}
        <div className="mb-6 flex flex-wrap items-center justify-between gap-4">
          <div>
            <Link
              href="/billing"
              className="mb-2 inline-block text-sm text-zinc-500 hover:text-zinc-700"
            >
              &larr; Zurück zur Übersicht
            </Link>
            <h1 className="text-2xl font-bold text-zinc-900">
              Abrechnung: {property.street}, {property.city}
            </h1>
            <p className="mt-1 text-sm text-zinc-500">
              Zeitraum: {formatDate(billingPeriod.startDate)} &ndash;{" "}
              {formatDate(billingPeriod.endDate)} ({monthsInPeriod} Monate,{" "}
              {daysInPeriod} Tage)
            </p>
            {billingPeriod.billingDate && (
              <p className="text-sm text-zinc-500">
                Abrechnungsdatum: {formatDate(billingPeriod.billingDate)}
              </p>
            )}
          </div>
          <div className="flex items-center gap-3">
            {saveStatus && (
              <span
                className={`text-sm ${
                  saveStatus === "Gespeichert"
                    ? "text-green-600"
                    : "text-red-600"
                }`}
              >
                {saveStatus}
              </span>
            )}
            <button
              onClick={handleSave}
              disabled={saving}
              className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
            >
              {saving ? "Speichern..." : "Speichern"}
            </button>
            <a
              href={`/api/billing-periods/${id}/pdf`}
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
            >
              PDF erstellen
            </a>
          </div>
        </div>

        {/* Section 1: Kosten */}
        <section className="mb-8">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">Kosten</h2>
          {costCategories.length === 0 ? (
            <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
              <p className="text-sm text-zinc-500">
                Keine Kostenarten definiert. Bitte legen Sie zuerst Kostenarten
                in den{" "}
                <Link
                  href="/settings"
                  className="font-medium text-zinc-900 hover:text-zinc-700"
                >
                  Einstellungen
                </Link>{" "}
                an.
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white shadow-sm">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-zinc-200 bg-zinc-50">
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Kostenart
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Verteilerschlüssel
                    </th>
                    <th className="w-40 px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Gesamtbetrag
                    </th>
                    <th className="w-44 px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Anteil Wohnung
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-200">
                  {costCategories.map((cat) => {
                    const costVal = costValues[cat.id] || {
                      totalAmount: "",
                      unitAmount: "",
                    };
                    const isMEA = cat.distributionKey === "MEA";
                    const needsUnitAmount =
                      cat.distributionKey === "laut Bescheid" ||
                      cat.distributionKey === "siehe Anlage";

                    let meaDisplay = "";
                    if (isMEA && costVal.totalAmount && units.length > 0) {
                      const amounts = units.map(
                        (u) =>
                          `${u.name}: ${formatCurrency(
                            calculateMEAAmount(
                              parseFloat(costVal.totalAmount) || 0,
                              u.shares,
                              property.totalShares
                            )
                          )}`
                      );
                      meaDisplay = amounts.join(", ");
                    }

                    return (
                      <tr key={cat.id}>
                        <td className="px-4 py-3 text-sm text-zinc-900">
                          {cat.name}
                        </td>
                        <td className="px-4 py-3 text-sm text-zinc-600">
                          {cat.distributionKey}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <div className="flex justify-end">
                            <input
                              type="number"
                              step="0.01"
                              value={costVal.totalAmount}
                              onChange={(e) =>
                                handleCostChange(
                                  cat.id,
                                  "totalAmount",
                                  e.target.value
                                )
                              }
                              placeholder="0,00"
                              className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                            />
                          </div>
                        </td>
                        <td className="px-4 py-3 text-right">
                          {needsUnitAmount ? (
                            <div className="flex justify-end">
                              <input
                                type="number"
                                step="0.01"
                                value={costVal.unitAmount}
                                onChange={(e) =>
                                  handleCostChange(
                                    cat.id,
                                    "unitAmount",
                                    e.target.value
                                  )
                                }
                                placeholder="0,00"
                                className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                              />
                            </div>
                          ) : isMEA ? (
                            <div className="text-sm text-zinc-500">
                              {meaDisplay || "Automatisch (MEA)"}
                            </div>
                          ) : (
                            <div className="text-sm text-zinc-400">
                              &ndash;
                            </div>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </section>

        {/* Section 2: Vorauszahlungen */}
        <section className="mb-8">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Vorauszahlungen
          </h2>
          {units.length === 0 ? (
            <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
              <p className="text-sm text-zinc-500">
                Keine Wohnungen für dieses Objekt vorhanden.
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white shadow-sm">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-zinc-200 bg-zinc-50">
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Wohnung
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Mieter
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Anteile
                    </th>
                    <th className="w-40 px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Monatlich
                    </th>
                    <th className="w-40 px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Gesamt ({monthsInPeriod} Mon.)
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-200">
                  {units.map((unit) => {
                    const tenant = getCurrentTenant(unit);
                    const monthlyVal = prepaymentValues[unit.id] || "";
                    const totalPrepayment =
                      (parseFloat(monthlyVal) || 0) * monthsInPeriod;

                    return (
                      <tr key={unit.id}>
                        <td className="px-4 py-3 text-sm text-zinc-900">
                          {unit.name}
                        </td>
                        <td className="px-4 py-3 text-sm text-zinc-600">
                          {tenant
                            ? `${tenant.firstName} ${tenant.lastName}`
                            : "Kein Mieter"}
                        </td>
                        <td className="px-4 py-3 text-sm text-zinc-600">
                          {unit.shares} / {property.totalShares}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <div className="flex justify-end">
                            <input
                              type="number"
                              step="0.01"
                              value={monthlyVal}
                              onChange={(e) =>
                                handlePrepaymentChange(
                                  unit.id,
                                  e.target.value
                                )
                              }
                              placeholder="0,00"
                              className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                            />
                          </div>
                        </td>
                        <td className="px-4 py-3 text-right text-sm font-medium text-zinc-900">
                          {formatCurrency(totalPrepayment)}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </section>

        {/* Section 3: Zusammenfassung */}
        <section>
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Zusammenfassung
          </h2>
          {units.length === 0 ? (
            <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
              <p className="text-sm text-zinc-500">
                Keine Wohnungen vorhanden.
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white shadow-sm">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-zinc-200 bg-zinc-50">
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Wohnung
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Mieter
                    </th>
                    <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Kosten
                    </th>
                    <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Vorauszahlungen
                    </th>
                    <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Ergebnis
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-200">
                  {units.map((unit) => {
                    const tenant = getCurrentTenant(unit);
                    const unitCosts = calculateUnitCosts(unit);
                    const unitPrepayment = calculateUnitPrepayment(unit.id);
                    const result = unitCosts - unitPrepayment;

                    return (
                      <tr key={unit.id}>
                        <td className="px-4 py-3 text-sm text-zinc-900">
                          {unit.name}
                        </td>
                        <td className="px-4 py-3 text-sm text-zinc-600">
                          {tenant
                            ? `${tenant.firstName} ${tenant.lastName}`
                            : "Kein Mieter"}
                        </td>
                        <td className="px-4 py-3 text-right text-sm text-zinc-900">
                          {formatCurrency(unitCosts)}
                        </td>
                        <td className="px-4 py-3 text-right text-sm text-zinc-900">
                          {formatCurrency(unitPrepayment)}
                        </td>
                        <td
                          className={`px-4 py-3 text-right text-sm font-semibold ${
                            result > 0 ? "text-red-600" : "text-green-600"
                          }`}
                        >
                          {result > 0
                            ? `Nachzahlung: ${formatCurrency(result)}`
                            : result < 0
                              ? `Erstattung: ${formatCurrency(Math.abs(result))}`
                              : formatCurrency(0)}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
                <tfoot>
                  <tr className="border-t-2 border-zinc-300 bg-zinc-50">
                    <td
                      className="px-4 py-3 text-sm font-semibold text-zinc-900"
                      colSpan={2}
                    >
                      Gesamt
                    </td>
                    <td className="px-4 py-3 text-right text-sm font-semibold text-zinc-900">
                      {formatCurrency(getTotalCosts())}
                    </td>
                    <td className="px-4 py-3 text-right text-sm font-semibold text-zinc-900">
                      {formatCurrency(getTotalPrepayments())}
                    </td>
                    <td
                      className={`px-4 py-3 text-right text-sm font-bold ${
                        getTotalCosts() - getTotalPrepayments() > 0
                          ? "text-red-600"
                          : "text-green-600"
                      }`}
                    >
                      {formatCurrency(
                        Math.abs(getTotalCosts() - getTotalPrepayments())
                      )}
                    </td>
                  </tr>
                </tfoot>
              </table>
            </div>
          )}
        </section>

        {/* Bottom Actions */}
        <div className="mt-8 flex items-center justify-between">
          <Link
            href="/billing"
            className="text-sm text-zinc-500 hover:text-zinc-700"
          >
            &larr; Zurück zur Übersicht
          </Link>
          <div className="flex items-center gap-3">
            {saveStatus && (
              <span
                className={`text-sm ${
                  saveStatus === "Gespeichert"
                    ? "text-green-600"
                    : "text-red-600"
                }`}
              >
                {saveStatus}
              </span>
            )}
            <button
              onClick={handleSave}
              disabled={saving}
              className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
            >
              {saving ? "Speichern..." : "Speichern"}
            </button>
            <a
              href={`/api/billing-periods/${id}/pdf`}
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
            >
              PDF erstellen
            </a>
          </div>
        </div>
      </main>
    </>
  );
}
