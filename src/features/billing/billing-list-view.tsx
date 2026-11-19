"use client";

import { useMemo, useState } from "react";
import {
  Button,
  EmptyState,
  ErrorState,
  InlineNotice,
  LoadingState,
  PageHeader,
  useConfirm,
} from "@zweiund40gmbh/ui";
import { BillingCreateModal } from "./billing-create-modal";
import { groupByProperty, previousYearSuggestions } from "./billing-list-utils";
import { BillingPeriodTable } from "./billing-period-table";
import { BillingSuggestions } from "./billing-suggestions";
import { useBillingList } from "./use-billing-list";

/** Billing overview: previous-year hints, periods grouped by property, create dialog and delete. */
export function BillingListView() {
  const list = useBillingList();
  const confirm = useConfirm();
  const [creating, setCreating] = useState(false);

  const groups = useMemo(() => groupByProperty(list.periods), [list.periods]);
  const suggestions = useMemo(
    () => previousYearSuggestions(list.properties, list.periods, new Date().getFullYear()),
    [list.properties, list.periods],
  );

  async function remove(id: string) {
    const ok = await confirm({
      title: "Abrechnung löschen",
      message: "Möchten Sie diese Abrechnung wirklich löschen?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (ok) await list.remove(id);
  }

  return (
    <div className="grid gap-6">
      <PageHeader
        title="Abrechnungen"
        actions={
          <Button variant="primary" onClick={() => setCreating(true)}>
            Neue Abrechnung
          </Button>
        }
      />
      {list.actionError && <InlineNotice tone="bad">{list.actionError}</InlineNotice>}
      {list.loading ? (
        <LoadingState />
      ) : list.loadFailed ? (
        <ErrorState
          title="Die Abrechnungen konnten nicht geladen werden."
          error={null}
          formatError={() => "Bitte versuchen Sie es erneut."}
          onRetry={() => void list.retry()}
        />
      ) : (
        <>
          <BillingSuggestions suggestions={suggestions} creatingFor={list.creatingFor} onCreate={list.quickCreate} />
          {groups.length === 0 ? (
            <EmptyState
              title="Noch keine Abrechnungen vorhanden. Erstellen Sie Ihre erste Abrechnung."
              icon={null}
            />
          ) : (
            groups.map((group) => <BillingPeriodTable key={group.propertyId} group={group} onDelete={remove} />)
          )}
        </>
      )}
      {creating && (
        <BillingCreateModal
          properties={list.properties}
          periods={list.periods}
          onCreate={list.create}
          onClose={() => setCreating(false)}
        />
      )}
    </div>
  );
}
