"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { EyeIcon, EyeOffIcon } from "lucide-react";
import { Nav } from "@/components/nav";
import { formatCurrency, formatDate } from "@/lib/format";
import {
  calculateAllocationAmount,
  calculateMEAAmount,
  dayAfter,
  getDaysInPeriod,
  getMonthsInPeriod,
  suggestNextPrepayment,
} from "@/lib/billing";
import { DISTRIBUTION_KEYS } from "@/lib/constants";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import { OptionalDateInput } from "@/components/ui/optional-date-input";
import { Combobox } from "@/components/ui/combobox";
import { DocumentUpload } from "@/components/document-upload";
import {
  RentChangeDialog,
  type RentChangeDialogValue,
} from "@/components/rent-change-dialog";
import type { BillingPeriodDetail, CostCategory, Tenant, UnitWithTenants } from "@/types";

interface NkSuggestionTarget {
  unitId: string;
  unitName: string;
  tenantName: string;
  currentMonthly: number;
  shortfall: number;
  suggested: number;
}

interface CostFormValue {
  totalAmount: string;
  unitAmount: string;
  ownerAmount: string;
  tenantAmountOverride: string;
  enabled: boolean;
  distributionKeyOverride: string | null;
}

const DEFAULT_COST_VALUE: CostFormValue = {
  totalAmount: "",
  unitAmount: "",
  ownerAmount: "",
  tenantAmountOverride: "",
  enabled: true,
  distributionKeyOverride: null,
};

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

  const [costValues, setCostValues] = useState<Record<string, CostFormValue>>(
    {}
  );

  const [prepaymentValues, setPrepaymentValues] = useState<
    Record<string, string>
  >({});

  const [nkAdjustTarget, setNkAdjustTarget] = useState<NkSuggestionTarget | null>(
    null
  );
  const [nkAppliedUnitIds, setNkAppliedUnitIds] = useState<Set<string>>(new Set());

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

        const costs: Record<string, CostFormValue> = {};
        bp.costs.forEach((cost) => {
          costs[cost.costCategoryId] = {
            totalAmount: cost.totalAmount.toString(),
            unitAmount: cost.unitAmount?.toString() || "",
            ownerAmount: cost.ownerAmount?.toString() || "",
            tenantAmountOverride: cost.tenantAmountOverride?.toString() || "",
            enabled: cost.enabled !== false,
            distributionKeyOverride: cost.distributionKeyOverride ?? null,
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
    field: "totalAmount" | "unitAmount" | "ownerAmount" | "tenantAmountOverride",
    value: string
  ) {
    setCostValues((prev) => ({
      ...prev,
      [categoryId]: {
        ...DEFAULT_COST_VALUE,
        ...(prev[categoryId] ?? {}),
        [field]: value,
      },
    }));
    triggerAutoSave();
  }

  async function handleToggleCostEnabled(categoryId: string, enabled: boolean) {
    const current = costValues[categoryId] ?? DEFAULT_COST_VALUE;
    setCostValues((prev) => ({
      ...prev,
      [categoryId]: { ...current, enabled },
    }));
    try {
      await fetch(`/api/billing-periods/${id}/costs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          costCategoryId: categoryId,
          enabled,
        }),
      });
      await fetchData();
    } catch (error) {
      console.error("Failed to toggle cost:", error);
    }
  }

  async function handleDistributionKeyOverride(
    categoryId: string,
    override: string | null
  ) {
    const current = costValues[categoryId] ?? DEFAULT_COST_VALUE;
    setCostValues((prev) => ({
      ...prev,
      [categoryId]: { ...current, distributionKeyOverride: override },
    }));
    try {
      await fetch(`/api/billing-periods/${id}/costs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          costCategoryId: categoryId,
          distributionKeyOverride: override,
        }),
      });
      await fetchData();
    } catch (error) {
      console.error("Failed to update distribution key override:", error);
    }
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
              ownerAmount: val.ownerAmount ? parseFloat(val.ownerAmount) : null,
              tenantAmountOverride: val.tenantAmountOverride
                ? parseFloat(val.tenantAmountOverride)
                : null,
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

  async function handleDateChange(field: "billingDate" | "sentDate" | "paidDate", value: string) {
    try {
      const res = await fetch(`/api/billing-periods/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ [field]: value || null }),
      });
      if (res.ok) {
        setBillingPeriod((prev) =>
          prev ? { ...prev, [field]: value || null } : prev
        );
      }
    } catch (error) {
      console.error(`Failed to save ${field}:`, error);
    }
  }

  async function handleConfirmCost(costCategoryId: string) {
    const costVal = costValues[costCategoryId];
    if (!costVal) return;
    try {
      const res = await fetch(`/api/billing-periods/${id}/costs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          costCategoryId,
          totalAmount: parseFloat(costVal.totalAmount) || 0,
          unitAmount: costVal.unitAmount ? parseFloat(costVal.unitAmount) : null,
          ownerAmount: costVal.ownerAmount ? parseFloat(costVal.ownerAmount) : null,
          tenantAmountOverride: costVal.tenantAmountOverride
            ? parseFloat(costVal.tenantAmountOverride)
            : null,
          reviewed: true,
        }),
      });
      if (res.ok) {
        await fetchData();
      }
    } catch (error) {
      console.error("Failed to confirm cost:", error);
    }
  }

  async function handleConfirmPrepayment(unitId: string) {
    const val = prepaymentValues[unitId];
    if (!val) return;
    try {
      const res = await fetch(`/api/billing-periods/${id}/prepayments`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          unitId,
          monthlyAmount: parseFloat(val) || 0,
          reviewed: true,
        }),
      });
      if (res.ok) {
        await fetchData();
      }
    } catch (error) {
      console.error("Failed to confirm prepayment:", error);
    }
  }

  async function applyNkAdjustment(
    target: NkSuggestionTarget,
    value: RentChangeDialogValue
  ) {
    try {
      const res = await fetch("/api/rent-changes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          unitId: target.unitId,
          type: "prepayment",
          amount: value.amount,
          effectiveDate: value.effectiveDate,
          reason:
            value.reason ??
            `NK-Anpassung nach Abrechnung (Nachzahlung ${target.shortfall.toFixed(2)} €)`,
        }),
      });
      if (res.ok) {
        setNkAppliedUnitIds((prev) => new Set(prev).add(target.unitId));
      }
    } catch (error) {
      console.error("Failed to create prepayment adjustment:", error);
    }
  }

  function calculateUnitCosts(unit: UnitWithTenants): number {
    let total = 0;
    costCategories.forEach((cat) => {
      const costVal = costValues[cat.id];
      if (!costVal || !costVal.totalAmount) return;
      if (costVal.enabled === false) return;
      if (cat.apportionable === false) return;
      const totalAmount = parseFloat(costVal.totalAmount) || 0;
      const effectiveKey = costVal.distributionKeyOverride ?? cat.distributionKey;
      const tenantAmountOverride = costVal.tenantAmountOverride
        ? parseFloat(costVal.tenantAmountOverride)
        : null;

      if (tenantAmountOverride !== null && billingPeriod!.property.accountingMode === "weg") {
        total += tenantAmountOverride;
        return;
      }

      if (effectiveKey === "MEA") {
        total += calculateMEAAmount(totalAmount, unit.shares, billingPeriod!.property.totalShares);
      } else if (effectiveKey !== "laut Bescheid" && effectiveKey !== "siehe Anlage") {
        const allocationKey = unit.allocationKeys?.find(
          (key) => key.key === effectiveKey
        );
        total += allocationKey
          ? calculateAllocationAmount(
              totalAmount,
              allocationKey.unitValue,
              allocationKey.totalValue
            )
          : 0;
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
  const allocationKeyNames = Array.from(
    new Set(
      units.flatMap((unit) =>
        (unit.allocationKeys ?? []).map((allocationKey) => allocationKey.key)
      )
    )
  );
  const monthsInPeriod = getMonthsInPeriod(billingPeriod.startDate, billingPeriod.endDate);
  const daysInPeriod = getDaysInPeriod(billingPeriod.startDate, billingPeriod.endDate);
  const isWegMode = property.accountingMode === "weg";

  // Disabled cost positions are excluded from review tracking — they aren't
  // part of this period's calculation, so they shouldn't show as "ungeprüft".
  const activeCosts = billingPeriod.costs.filter((c) => c.enabled !== false);
  const totalItems = activeCosts.length + billingPeriod.prepayments.length;
  const reviewedItems =
    activeCosts.filter((c) => c.reviewed).length +
    billingPeriod.prepayments.filter((p) => p.reviewed).length;
  const allReviewed = totalItems > 0 && reviewedItems === totalItems;

  const costReviewMap: Record<string, boolean> = {};
  billingPeriod.costs.forEach((cost) => {
    costReviewMap[cost.costCategoryId] = cost.reviewed;
  });

  const prepaymentReviewMap: Record<string, boolean> = {};
  billingPeriod.prepayments.forEach((pp) => {
    prepaymentReviewMap[pp.unitId] = pp.reviewed;
  });

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
            <div className="mt-2 flex flex-wrap gap-4">
              <div className="w-48">
                <label className="mb-1 block text-xs font-medium text-zinc-500">
                  Abrechnungsdatum
                </label>
                <OptionalDateInput
                  value={billingPeriod.billingDate || ""}
                  onChange={(value) => handleDateChange("billingDate", value)}
                />
              </div>
              <div className="w-48">
                <label className="mb-1 block text-xs font-medium text-zinc-500">
                  Versendet am
                </label>
                <OptionalDateInput
                  value={billingPeriod.sentDate || ""}
                  onChange={(value) => handleDateChange("sentDate", value)}
                />
              </div>
              <div className="w-48">
                <label className="mb-1 block text-xs font-medium text-zinc-500">
                  Bezahlt am
                </label>
                <OptionalDateInput
                  value={billingPeriod.paidDate || ""}
                  onChange={(value) => handleDateChange("paidDate", value)}
                />
              </div>
            </div>
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

        {/* Review Progress Banner */}
        {totalItems > 0 && (
          <div
            className={`mb-6 rounded-xl border px-4 py-3 ${
              allReviewed
                ? "border-green-200 bg-green-50 text-green-800"
                : "border-amber-200 bg-amber-50 text-amber-800"
            }`}
          >
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">
                {reviewedItems} von {totalItems} Positionen geprüft
              </span>
              {allReviewed && (
                <span className="text-xs font-medium">Alle geprüft</span>
              )}
            </div>
            <div className="mt-2 h-2 w-full overflow-hidden rounded-full bg-white/60">
              <div
                className={`h-full rounded-full transition-all ${
                  allReviewed ? "bg-green-500" : "bg-amber-400"
                }`}
                style={{
                  width: `${totalItems > 0 ? (reviewedItems / totalItems) * 100 : 0}%`,
                }}
              />
            </div>
          </div>
        )}

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
                    <th className="w-12 px-2 py-3 text-center text-xs font-medium uppercase text-zinc-500">
                      <span className="sr-only">Aktiv</span>
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Kostenart
                    </th>
                    <th className="w-56 px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
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
                    const costVal = costValues[cat.id] || DEFAULT_COST_VALUE;
                    const enabled = costVal.enabled !== false;
                    const apportionable = cat.apportionable !== false;
                    const effectiveKey =
                      costVal.distributionKeyOverride ?? cat.distributionKey;
                    const isMEA = effectiveKey === "MEA";
                    const isManual =
                      effectiveKey === "laut Bescheid" ||
                      effectiveKey === "siehe Anlage";
                    const isCustomAllocationKey = !isMEA && !isManual;
                    const needsUnitAmount =
                      effectiveKey === "laut Bescheid" ||
                      effectiveKey === "siehe Anlage";
                    const tenantAmountOverride = costVal.tenantAmountOverride
                      ? parseFloat(costVal.tenantAmountOverride)
                      : null;

                    let calculatedDisplay = "";
                    if (
                      apportionable &&
                      isWegMode &&
                      tenantAmountOverride !== null
                    ) {
                      calculatedDisplay = formatCurrency(tenantAmountOverride);
                    } else if (apportionable && costVal.totalAmount && units.length > 0) {
                      const amounts = units.map(
                        (u) => {
                          const totalAmount =
                            parseFloat(costVal.totalAmount) || 0;
                          let amount = 0;
                          if (isMEA) {
                            amount = calculateMEAAmount(
                              totalAmount,
                              u.shares,
                              property.totalShares
                            );
                          } else if (isCustomAllocationKey) {
                            const allocationKey = u.allocationKeys?.find(
                              (key) => key.key === effectiveKey
                            );
                            amount = allocationKey
                              ? calculateAllocationAmount(
                                  totalAmount,
                                  allocationKey.unitValue,
                                  allocationKey.totalValue
                                )
                              : 0;
                          }

                          return `${u.name}: ${formatCurrency(amount)}`;
                        }
                      );
                      calculatedDisplay = amounts.join(", ");
                    }

                    const isUnreviewed =
                      enabled && costReviewMap[cat.id] === false;
                    const overridden =
                      costVal.distributionKeyOverride !== null &&
                      costVal.distributionKeyOverride !== undefined;

                    const distributionOptions = [
                      {
                        value: "__default__",
                        label: `Standard (${cat.distributionKey})`,
                      },
                      ...DISTRIBUTION_KEYS.map((k) => ({
                        value: k,
                        label: k,
                      })),
                      ...allocationKeyNames
                        .filter((key) => !DISTRIBUTION_KEYS.includes(key))
                        .map((key) => ({
                          value: key,
                          label: key,
                        })),
                    ];

                    return (
                      <tr
                        key={cat.id}
                        className={
                          !enabled
                            ? "bg-zinc-50/60 text-zinc-400"
                            : !apportionable
                              ? "bg-zinc-50"
                            : isUnreviewed
                              ? "border-l-4 border-amber-400 bg-amber-50/50"
                              : ""
                        }
                      >
                        <td className="px-2 py-3 text-center">
                          <button
                            type="button"
                            onClick={() =>
                              handleToggleCostEnabled(cat.id, !enabled)
                            }
                            title={
                              enabled
                                ? "In dieser Abrechnung ausblenden"
                                : "In dieser Abrechnung einblenden"
                            }
                            className="rounded p-1 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-600"
                          >
                            {enabled ? (
                              <EyeIcon className="h-4 w-4" />
                            ) : (
                              <EyeOffIcon className="h-4 w-4" />
                            )}
                          </button>
                        </td>
                        <td className="px-4 py-3 text-sm">
                          <div className="flex items-center gap-2">
                            <span className={!enabled ? "line-through" : ""}>
                              {cat.name}
                            </span>
                            {!apportionable && (
                              <span className="rounded bg-zinc-100 px-2 py-0.5 text-xs font-medium text-zinc-600">
                                nicht umlagefähig
                              </span>
                            )}
                            {cat.requiresAttachment && (
                              <span className="rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                                Anlage
                              </span>
                            )}
                            {isUnreviewed && (
                              <button
                                type="button"
                                onClick={() => handleConfirmCost(cat.id)}
                                className="rounded border border-amber-300 bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700 hover:bg-amber-200"
                              >
                                Bestätigen
                              </button>
                            )}
                          </div>
                        </td>
                        <td className="px-4 py-3 text-sm">
                          <Combobox
                            options={distributionOptions}
                            value={
                              overridden
                                ? costVal.distributionKeyOverride!
                                : "__default__"
                            }
                            onChange={(v) =>
                              handleDistributionKeyOverride(
                                cat.id,
                                v === "__default__" ? null : v
                              )
                            }
                          />
                          {overridden && (
                            <p className="mt-1 text-xs text-amber-700">
                              Abweichend (Standard: {cat.distributionKey})
                            </p>
                          )}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <div className="space-y-2">
                            <div className="flex justify-end">
                              <input
                                type="number"
                                step="0.01"
                                value={costVal.totalAmount}
                                disabled={!enabled}
                                onChange={(e) =>
                                  handleCostChange(
                                    cat.id,
                                    "totalAmount",
                                    e.target.value
                                  )
                                }
                                placeholder="0,00"
                                className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500 disabled:bg-zinc-100 disabled:text-zinc-400"
                              />
                            </div>
                            {isWegMode && (
                              <div>
                                <label className="mb-1 block text-right text-xs text-zinc-500">
                                  Ihr Anteil
                                </label>
                                <div className="flex justify-end">
                                  <input
                                    type="number"
                                    step="0.01"
                                    value={costVal.ownerAmount}
                                    disabled={!enabled}
                                    onChange={(e) =>
                                      handleCostChange(
                                        cat.id,
                                        "ownerAmount",
                                        e.target.value
                                      )
                                    }
                                    placeholder="0,00"
                                    className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500 disabled:bg-zinc-100 disabled:text-zinc-400"
                                  />
                                </div>
                              </div>
                            )}
                          </div>
                        </td>
                        <td className="px-4 py-3 text-right">
                          {isWegMode && apportionable ? (
                            <div className="space-y-2">
                              <label className="block text-right text-xs text-zinc-500">
                                davon umlagefähig
                              </label>
                              <div className="flex justify-end">
                                <input
                                  type="number"
                                  step="0.01"
                                  value={costVal.tenantAmountOverride}
                                  disabled={!enabled}
                                  onChange={(e) =>
                                    handleCostChange(
                                      cat.id,
                                      "tenantAmountOverride",
                                      e.target.value
                                    )
                                  }
                                  placeholder="automatisch"
                                  className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500 disabled:bg-zinc-100 disabled:text-zinc-400"
                                />
                              </div>
                              <div className="text-sm text-zinc-500">
                                {calculatedDisplay || "Automatisch"}
                              </div>
                            </div>
                          ) : !apportionable ? (
                            <div className="text-sm text-zinc-400">
                              Nicht berechnet
                            </div>
                          ) : needsUnitAmount ? (
                            <div className="flex justify-end">
                              <input
                                type="number"
                                step="0.01"
                                value={costVal.unitAmount}
                                disabled={!enabled}
                                onChange={(e) =>
                                  handleCostChange(
                                    cat.id,
                                    "unitAmount",
                                    e.target.value
                                  )
                                }
                                placeholder="0,00"
                                className="w-32 rounded-lg border border-zinc-300 px-3 py-2 text-right text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500 disabled:bg-zinc-100 disabled:text-zinc-400"
                              />
                            </div>
                          ) : isMEA || isCustomAllocationKey ? (
                            <div className="text-sm text-zinc-500">
                              {calculatedDisplay ||
                                `Automatisch (${effectiveKey})`}
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
                    const isPrepaymentUnreviewed =
                      prepaymentReviewMap[unit.id] === false;

                    return (
                      <tr
                        key={unit.id}
                        className={
                          isPrepaymentUnreviewed
                            ? "border-l-4 border-amber-400 bg-amber-50/50"
                            : ""
                        }
                      >
                        <td className="px-4 py-3 text-sm text-zinc-900">
                          <div className="flex items-center gap-2">
                            {unit.name}
                            {isPrepaymentUnreviewed && (
                              <button
                                type="button"
                                onClick={() =>
                                  handleConfirmPrepayment(unit.id)
                                }
                                className="rounded border border-amber-300 bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700 hover:bg-amber-200"
                              >
                                Bestätigen
                              </button>
                            )}
                          </div>
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

        {/* Section 3b: NK-Anpassungs-Empfehlungen (nur bei Nachzahlung) */}
        {(() => {
          const recommendations = units
            .map((unit) => {
              const tenant = getCurrentTenant(unit);
              const monthly = parseFloat(prepaymentValues[unit.id] || "0") || 0;
              const unitCosts = calculateUnitCosts(unit);
              const unitPrepayment = calculateUnitPrepayment(unit.id);
              const shortfall = unitCosts - unitPrepayment;
              const suggested = suggestNextPrepayment(monthly, shortfall);
              if (suggested == null) return null;
              return {
                unit,
                tenant,
                monthly,
                shortfall,
                suggested,
              };
            })
            .filter((r): r is NonNullable<typeof r> => r !== null);

          if (recommendations.length === 0) return null;

          return (
            <section className="mt-8">
              <h2 className="mb-4 text-lg font-semibold text-zinc-900">
                Empfohlene NK-Vorauszahlungs-Anpassungen
              </h2>
              <div className="space-y-3">
                {recommendations.map(
                  ({ unit, tenant, monthly, shortfall, suggested }) => {
                    const tenantName = tenant
                      ? `${tenant.firstName} ${tenant.lastName}`
                      : "Kein Mieter";
                    const applied = nkAppliedUnitIds.has(unit.id);
                    return (
                      <div
                        key={unit.id}
                        className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm border-l-4 border-l-amber-400"
                      >
                        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                          <div className="min-w-0 flex-1">
                            <p className="font-medium text-zinc-900">
                              {tenantName}
                              <span className="ml-2 text-sm font-normal text-zinc-500">
                                {unit.name}
                              </span>
                            </p>
                            <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
                              <span className="text-zinc-600">
                                Nachzahlung:{" "}
                                <span className="font-medium text-red-600">
                                  {formatCurrency(shortfall)}
                                </span>
                              </span>
                              <span className="text-zinc-400">|</span>
                              <span className="text-zinc-600">
                                NK-Vorauszahlung: {formatCurrency(monthly)} &rarr;{" "}
                                <span className="font-medium text-zinc-900">
                                  {formatCurrency(suggested)}
                                </span>
                                <span className="ml-1 text-zinc-400">/Monat</span>
                              </span>
                            </div>
                          </div>
                          <div className="flex flex-shrink-0 items-center gap-2">
                            {applied ? (
                              <span className="inline-flex rounded-full bg-green-100 px-3 py-1 text-xs font-medium text-green-700">
                                Anpassung angelegt
                              </span>
                            ) : (
                              <button
                                onClick={() =>
                                  setNkAdjustTarget({
                                    unitId: unit.id,
                                    unitName: unit.name,
                                    tenantName,
                                    currentMonthly: monthly,
                                    shortfall,
                                    suggested,
                                  })
                                }
                                className="rounded-lg bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-800"
                              >
                                Anpassung anlegen
                              </button>
                            )}
                          </div>
                        </div>
                      </div>
                    );
                  }
                )}
              </div>
            </section>
          );
        })()}

        {/* Section 4: Dokumente */}
        <section className="mt-8">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Dokumente
          </h2>
          <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
            <p className="mb-4 text-sm text-zinc-500">
              Belege und Rechnungen für diese Abrechnung (optional).
              Erlaubte Formate: PDF, JPEG, PNG, WebP (max. 10 MB).
            </p>
            <DocumentUpload
              billingPeriodId={id}
              category="invoice"
              label="Beleg hochladen"
            />
          </div>
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

        <RentChangeDialog
          open={nkAdjustTarget !== null}
          onOpenChange={(open) => !open && setNkAdjustTarget(null)}
          onSubmit={(value) => {
            if (nkAdjustTarget) return applyNkAdjustment(nkAdjustTarget, value);
          }}
          title="NK-Vorauszahlung anpassen"
          description={
            nkAdjustTarget
              ? `${nkAdjustTarget.tenantName} (${nkAdjustTarget.unitName}): aktuell ${formatCurrency(nkAdjustTarget.currentMonthly)}/Monat, Vorschlag ${formatCurrency(nkAdjustTarget.suggested)}/Monat nach Nachzahlung von ${formatCurrency(nkAdjustTarget.shortfall)}. Betrag bei Bedarf anpassen.`
              : ""
          }
          defaultAmount={nkAdjustTarget?.suggested ?? 0}
          defaultEffectiveDate={
            billingPeriod ? dayAfter(billingPeriod.endDate) : ""
          }
          defaultReason={
            nkAdjustTarget
              ? `NK-Anpassung nach Abrechnung (Nachzahlung ${nkAdjustTarget.shortfall.toFixed(2)} €)`
              : ""
          }
          amountLabel="Neue NK-Vorauszahlung (EUR / Monat)"
        />
      </main>
    </>
  );
}
