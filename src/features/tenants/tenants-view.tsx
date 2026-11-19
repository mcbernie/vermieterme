"use client";

import { useCallback, useMemo, useState } from "react";
import { Button, DataTable, EmptyState, InlineNotice, PageHeader, useConfirm } from "@zweiund40gmbh/ui";
import type { TenantWithUnit } from "@/types";
import { RentAdjustModal } from "./rent-adjust-modal";
import { tenantColumns } from "./tenant-columns";
import { TenantDetailSheet } from "./tenant-detail-sheet";
import { TenantFormSheet } from "./tenant-form-sheet";
import { latestRentChange, tenantName } from "./tenant-helpers";
import type { TenantFormValues } from "./tenant-form-values";
import { useTenants, type RentAdjustment } from "./use-tenants";

/** Which form is open: a new tenant, an existing one, or none. */
type FormTarget = { tenant?: TenantWithUnit } | null;

/**
 * Tenants page: table of all tenants with detail panel (rent, app access, documents),
 * form panel to create and edit, rent adjustment dialog and confirmed delete.
 */
export function TenantsView() {
  const data = useTenants();
  const confirm = useConfirm();
  const [formTarget, setFormTarget] = useState<FormTarget>(null);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [adjustId, setAdjustId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const detail = data.tenants.find((t) => t.id === detailId) ?? null;
  const adjust = data.tenants.find((t) => t.id === adjustId) ?? null;

  const { deleteTenant, saveTenant, saveAdjustment } = data;

  /** Asks for confirmation and deletes the tenant; closes its detail panel on success. */
  const remove = useCallback(
    async (tenant: TenantWithUnit) => {
      const ok = await confirm({
        title: "Mieter löschen",
        message: `Möchten Sie ${tenantName(tenant)} wirklich löschen?`,
        confirmLabel: "Löschen",
        danger: true,
      });
      if (!ok) return;
      const error = await deleteTenant(tenant.id);
      setActionError(error);
      if (!error) setDetailId((current) => (current === tenant.id ? null : current));
    },
    [confirm, deleteTenant],
  );

  const columns = useMemo(
    () =>
      tenantColumns({
        rentChanges: data.rentChanges,
        onEdit: (tenant) => setFormTarget({ tenant }),
        onDelete: (tenant) => void remove(tenant),
      }),
    [data.rentChanges, remove],
  );

  /** Saves the open form; a success also clears an older action error. */
  async function save(values: TenantFormValues, id?: string) {
    const error = await saveTenant(values, id);
    if (!error) setActionError(null);
    return error;
  }

  /** Saves a rent change; a success also clears an older action error. */
  async function adjustRent(adjustment: RentAdjustment) {
    const error = await saveAdjustment(adjustment);
    if (!error) setActionError(null);
    return error;
  }

  /** Reloads the page data and clears an older action error. */
  function reload() {
    setActionError(null);
    void data.reload();
  }

  return (
    <div className="grid gap-4">
      <PageHeader
        title="Mieter"
        description="Mieter, Mietverträge, Miete und Vorauszahlungen, Dokumente und App-Zugang."
        actions={
          <Button variant="primary" onClick={() => setFormTarget({})}>
            Neuer Mieter
          </Button>
        }
      />
      {data.error && (
        <InlineNotice
          tone="bad"
          actions={
            <Button size="sm" onClick={reload}>
              Erneut laden
            </Button>
          }
        >
          {data.error}
        </InlineNotice>
      )}
      {actionError && <InlineNotice tone="bad">{actionError}</InlineNotice>}

      <DataTable
        label="Mieter"
        columns={columns}
        rows={data.tenants}
        getRowId={(t) => t.id}
        loading={data.loading}
        onRowClick={(t) => setDetailId(t.id)}
        defaultState={{ sort: { id: "property", desc: false } }}
        searchPlaceholder="Mieter, Objekt oder Wohnung suchen …"
        empty={
          data.error ? (
            // After a load error the list is unknown, not empty; the notice above explains it.
            <EmptyState title="Mieter konnten nicht geladen werden" icon={null} />
          ) : (
            <EmptyState
              title="Noch keine Mieter vorhanden"
              description="Legen Sie Ihren ersten Mieter an."
              action={
                <Button variant="primary" size="sm" onClick={() => setFormTarget({})}>
                  Neuer Mieter
                </Button>
              }
            />
          )
        }
      />

      {formTarget && (
        <TenantFormSheet
          tenant={formTarget.tenant}
          properties={data.properties}
          onSave={(values) => save(values, formTarget.tenant?.id)}
          onClose={() => setFormTarget(null)}
        />
      )}
      {detail && !formTarget && (
        <TenantDetailSheet
          tenant={detail}
          rentChanges={data.rentChanges}
          onEdit={() => setFormTarget({ tenant: detail })}
          onDelete={() => void remove(detail)}
          onAdjust={() => setAdjustId(detail.id)}
          onClose={() => setDetailId(null)}
        />
      )}
      {adjust && (
        <RentAdjustModal
          unitId={adjust.unitId}
          tenantName={tenantName(adjust)}
          defaultType={latestRentChange(data.rentChanges, adjust.unitId, "rent") ? "prepayment" : "rent"}
          onSave={adjustRent}
          onClose={() => setAdjustId(null)}
        />
      )}
    </div>
  );
}
