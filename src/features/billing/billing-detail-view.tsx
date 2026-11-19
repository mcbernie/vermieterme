"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import {
  EmptyState,
  ErrorState,
  InlineNotice,
  LoadingState,
  Tabs,
  tabPanelProps,
  type AutosaveStatus,
} from "@zweiund40gmbh/ui";
import { DocumentUpload } from "@/components/document-upload";
import { costKey, nkRecommendations, prepaymentKey, reviewProgress, unitResults } from "./billing-calc";
import { BillingDetailHeader } from "./billing-detail-header";
import { CostTable } from "./cost-table";
import { NkRecommendations } from "./nk-recommendations";
import { PrepaymentTable } from "./prepayment-table";
import { ReviewProgress } from "./review-progress";
import { SummaryTable } from "./summary-table";
import { useBillingDetail } from "./use-billing-detail";
import { useBillingSave, useDirtyKeys, type SaveState } from "./use-billing-save";

type TabKey = "costs" | "prepayments" | "summary" | "documents";
const TABS_ID = "billing-tabs";
const TAB_ITEMS: [TabKey, string][] = [
  ["costs", "Kosten"],
  ["prepayments", "Vorauszahlungen"],
  ["summary", "Zusammenfassung"],
  ["documents", "Dokumente"],
];

/** Notice text for a failed or refused save. */
const SAVE_ERRORS: Partial<Record<NonNullable<SaveState>, string>> = {
  error: "Fehler beim Speichern",
  invalid: "Nicht gespeichert: Bitte korrigieren Sie die markierten Beträge.",
};

/**
 * State of the save indicator. A failed or refused save counts as unsaved changes; its reason is
 * shown as page notice.
 */
function indicator(saver: { saving: boolean; pending: boolean; state: SaveState }): AutosaveStatus {
  if (saver.saving) return "saving";
  if (saver.pending || saver.state === "error" || saver.state === "invalid") return "pending";
  return saver.state === "saved" ? "saved" : "idle";
}

/** Billing period editor: costs, prepayments, summary with recommendations and receipts in tabs. */
export function BillingDetailView({ id }: { id: string }) {
  // Unsaved amounts: the save marks and settles them, a reload keeps only these from the page.
  const dirty = useDirtyKeys();
  const detail = useBillingDetail(id, dirty);
  const values = useMemo(
    () => ({ costs: detail.costs, prepayments: detail.prepayments }),
    [detail.costs, detail.prepayments],
  );
  const saver = useBillingSave(id, values, dirty);
  const [tab, setTab] = useState<TabKey>("costs");
  const { period, categories } = detail;

  const results = useMemo(
    () => (period ? unitResults(period, categories, detail.costs, detail.prepayments) : []),
    [period, categories, detail.costs, detail.prepayments],
  );

  if (detail.status === "loading") return <LoadingState />;
  if (detail.status === "notFound") {
    return (
      <EmptyState
        title="Abrechnung nicht gefunden."
        icon={null}
        action={
          <Link href="/billing" className="text-sm text-accentstrong hover:underline">
            Zurück zur Übersicht
          </Link>
        }
      />
    );
  }
  if (detail.status === "failed" || !period) {
    return (
      <ErrorState
        title="Die Abrechnung konnte nicht geladen werden."
        error={null}
        formatError={() => "Bitte versuchen Sie es erneut."}
        onRetry={() => void detail.retry()}
      />
    );
  }

  const saveError = saver.state ? SAVE_ERRORS[saver.state] : undefined;
  const panel = (key: TabKey) => ({ ...tabPanelProps(TABS_ID, key), hidden: tab !== key, className: "grid gap-6" });

  return (
    <div className="grid gap-6">
      <BillingDetailHeader
        id={id}
        period={period}
        saving={saver.saving}
        saveStatus={indicator(saver)}
        lastSavedAt={saver.lastSavedAt}
        onSave={() => void saver.save()}
        onDate={detail.setDate}
      />
      {saveError && <InlineNotice tone="bad">{saveError}</InlineNotice>}
      {detail.actionError && <InlineNotice tone="bad">{detail.actionError}</InlineNotice>}
      <ReviewProgress {...reviewProgress(period)} />
      <Tabs id={TABS_ID} aria-label="Bereiche der Abrechnung" value={tab} onChange={setTab} items={TAB_ITEMS} />

      <div {...panel("costs")}>
        <CostTable
          period={period}
          categories={categories}
          costs={detail.costs}
          onAmount={(categoryId, field, euros) => {
            detail.patchCost(categoryId, { [field]: euros });
            saver.schedule(costKey(categoryId));
          }}
          onProblem={saver.reportProblem}
          onToggle={detail.toggleEnabled}
          onOverride={detail.setOverride}
          onConfirm={detail.confirmCost}
        />
      </div>
      <div {...panel("prepayments")}>
        <PrepaymentTable
          period={period}
          prepayments={detail.prepayments}
          onChange={(unitId, euros) => {
            detail.setPrepayment(unitId, euros);
            saver.schedule(prepaymentKey(unitId));
          }}
          onProblem={saver.reportProblem}
          onConfirm={detail.confirmPrepayment}
        />
      </div>
      <div {...panel("summary")}>
        <SummaryTable results={results} />
        <NkRecommendations recommendations={nkRecommendations(results)} periodEnd={period.endDate} />
      </div>
      <div {...panel("documents")}>
        <section className="panel grid gap-3">
          <p className="text-sm text-ink2">
            Belege und Rechnungen für diese Abrechnung (optional). Erlaubte Formate: PDF, JPEG, PNG, WebP (max. 10 MB).
          </p>
          <DocumentUpload billingPeriodId={id} category="invoice" label="Beleg hochladen" />
        </section>
      </div>
    </div>
  );
}
