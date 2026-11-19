"use client";

import { useRouter } from "next/navigation";
import { Button, LoadingState, PageHeader } from "@zweiund40gmbh/ui";
import { RentChangeDialog } from "@/components/rent-change-dialog";
import { useMultiFetch } from "@/hooks/use-fetch";
import { formatCurrency } from "@/lib/format";
import type { BillingPeriodWithProperty, LandlordInfo, PropertyWithCount, VpiSuggestion } from "@/types";
import { RecentBillings } from "./recent-billings";
import { StatRow } from "./stat-row";
import { useVpiAdjustment } from "./use-vpi-adjustment";
import { formatVpiPercent, vpiReason } from "./vpi-text";
import { VpiSuggestions } from "./vpi-suggestions";

interface DashboardData {
  properties: PropertyWithCount[];
  billingPeriods: BillingPeriodWithProperty[];
  landlord: LandlordInfo;
  vpiSuggestions: VpiSuggestion[];
}

/** Dashboard: key figures, quick actions, VPI hints and the latest billings. */
export function DashboardView() {
  const router = useRouter();
  const { data, loading, refetch } = useMultiFetch<DashboardData>({
    properties: "/api/properties",
    billingPeriods: "/api/billing-periods",
    landlord: "/api/settings",
    vpiSuggestions: "/api/vpi/suggestions",
  });
  const vpi = useVpiAdjustment(refetch);

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
