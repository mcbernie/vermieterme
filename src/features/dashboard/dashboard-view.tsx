"use client";

import { useCallback, useRef } from "react";
import { useRouter } from "next/navigation";
import { Button, InlineNotice, LoadingState, PageHeader } from "@zweiund40gmbh/ui";
import { RentChangeDialog } from "@/components/rent-change-dialog";
import { useLoad } from "@/components/form/use-load";
import { apiLoadAll } from "@/lib/api-client";
import { formatCurrency } from "@/lib/format";
import type { BillingPeriodWithProperty, LandlordInfo, PropertyWithCount, VpiSuggestion } from "@/types";
import { RecentBillings } from "./recent-billings";
import { StatRow } from "./stat-row";
import { useVpiAdjustment } from "./use-vpi-adjustment";
import { formatVpiPercent, vpiReason } from "./vpi-text";
import { VpiSuggestions } from "./vpi-suggestions";

/** Shown when one of the dashboard requests failed, instead of silently empty figures. */
const PARTLY_FAILED = "Einige Daten konnten nicht geladen werden. Bitte laden Sie die Seite neu.";

interface DashboardData {
  properties: PropertyWithCount[];
  billingPeriods: BillingPeriodWithProperty[];
  landlord: LandlordInfo;
  vpiSuggestions: VpiSuggestion[];
}

/** Loads all dashboard data; a failed endpoint only leaves its part empty. */
function loadDashboard(signal: AbortSignal) {
  return apiLoadAll<DashboardData>(
    {
      properties: "/api/properties",
      billingPeriods: "/api/billing-periods",
      landlord: "/api/settings",
      vpiSuggestions: "/api/vpi/suggestions",
    },
    signal,
  );
}

/**
 * Loader for `useLoad`. The first load shows what it got (with a notice when parts are missing). A reload
 * with missing parts throws instead, so `useLoad` keeps the figures shown so far and reports the failure.
 */
function useDashboardLoader() {
  const shown = useRef(false);
  return useCallback(async (signal: AbortSignal) => {
    const result = await loadDashboard(signal);
    if (shown.current && result.failed) throw new Error("Dashboard reload incomplete");
    if (!signal.aborted) shown.current = true;
    return result;
  }, []);
}

/** Dashboard: key figures, quick actions, VPI hints and the latest billings. */
export function DashboardView() {
  const router = useRouter();
  const { data: loaded, loading, refreshError, refresh } = useLoad(useDashboardLoader());
  const data = loaded?.data ?? {};
  const failed = loaded?.failed ?? false;
  const vpi = useVpiAdjustment(refresh);

  const properties = data.properties ?? [];
  const billingPeriods = data.billingPeriods ?? [];
  const suggestions = data.vpiSuggestions ?? [];
  const units = properties.reduce((sum, p) => sum + (p._count?.units || 0), 0);
  const target = vpi.target;

  return (
    <div className="grid gap-6">
      <PageHeader
        title="Dashboard"
        description={data.landlord ? `Willkommen, ${data.landlord.name}` : undefined}
        actions={
          <>
            <Button size="sm" onClick={() => router.push("/tenants")}>
              Mieter verwalten
            </Button>
            <Button size="sm" onClick={() => router.push("/properties")}>
              Objekt hinzufügen
            </Button>
            <Button size="sm" variant="primary" onClick={() => router.push("/billing")}>
              Neue Abrechnung erstellen
            </Button>
          </>
        }
      />
      {loading ? (
        <LoadingState />
      ) : (
        <>
          {failed && <InlineNotice tone="bad">{PARTLY_FAILED}</InlineNotice>}
          {refreshError && <InlineNotice tone="bad">{refreshError}</InlineNotice>}
          {vpi.error && <InlineNotice tone="bad">{vpi.error}</InlineNotice>}
          <StatRow properties={properties.length} units={units} billings={billingPeriods.length} />
          {suggestions.length > 0 && (
            <VpiSuggestions suggestions={suggestions} appliedIds={vpi.appliedIds} onApply={vpi.setTarget} />
          )}
          <RecentBillings billingPeriods={billingPeriods} />
        </>
      )}
      <RentChangeDialog
        open={target !== null}
        onOpenChange={(open) => !open && vpi.setTarget(null)}
        onSubmit={(value) => {
          if (target) return vpi.apply(target, value);
        }}
        title="Mietanpassung durchführen"
        description={
          target
            ? `Kaltmiete für ${target.tenantName}: aktuell ${formatCurrency(target.currentRent)}, Vorschlag ${formatCurrency(target.suggestedRent)} (VPI ${formatVpiPercent(target)}). Betrag bei Bedarf anpassen.`
            : ""
        }
        defaultAmount={target?.suggestedRent ?? 0}
        defaultEffectiveDate={new Date().toISOString().split("T")[0]}
        defaultReason={target ? vpiReason(target) : ""}
        amountLabel="Neue Kaltmiete (EUR)"
      />
    </div>
  );
}
