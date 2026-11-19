"use client";

import { useState, type ReactNode } from "react";
import { Button, KeyValue, Sheet } from "@zweiund40gmbh/ui";
import { DocumentUpload } from "@/components/document-upload";
import { TenantAccessToken } from "@/components/tenant-access-token";
import { formatCurrency, formatDate } from "@/lib/format";
import type { RentChange, TenantWithUnit } from "@/types";
import { RentHistory } from "./rent-history";
import { LEASE_TYPE_OPTIONS } from "./tenant-form-values";
import { latestRentChange, propertyLabel, rentHistory, secondTenantName, tenantName } from "./tenant-helpers";
import { TenantTags } from "./tenant-tags";

interface Props {
  tenant: TenantWithUnit;
  rentChanges: readonly RentChange[];
  onEdit: () => void;
  onDelete: () => void;
  /** Opens the dialog to change rent or prepayment. */
  onAdjust: () => void;
  onClose: () => void;
}

/** Titled block inside the detail panel. */
function Block({ title, actions, children }: { title: string; actions?: ReactNode; children: ReactNode }) {
  return (
    <section className="grid gap-3 border-t border-line pt-4">
      <div className="flex items-center justify-between gap-2">
        <h3 className="section-title">{title}</h3>
        {actions}
      </div>
      {children}
    </section>
  );
}

/** Lease type label, extended by the index reference for index leases. */
function leaseText(tenant: TenantWithUnit): string {
  const label = LEASE_TYPE_OPTIONS.find((o) => o.value === tenant.leaseType)?.label ?? "Standard";
  if (tenant.leaseType !== "index") return label;
  const parts = [
    tenant.indexBaseYear && `Basisjahr ${tenant.indexBaseYear}`,
    tenant.indexReferenceValue && `Referenzwert ${tenant.indexReferenceValue.toLocaleString("de-DE")}`,
    tenant.indexReferenceDate && `angepasst ${formatDate(tenant.indexReferenceDate)}`,
    `mind. ${tenant.indexMinMonths} Monate`,
  ].filter(Boolean);
  return `${label} (${parts.join(", ")})`;
}

/**
 * Side panel with everything about one tenant: master data, rent and prepayment with history,
 * access code for the tenant app and contract documents. Mount it only while it should be open.
 */
export function TenantDetailSheet({ tenant, rentChanges, onEdit, onDelete, onAdjust, onClose }: Props) {
  const [showAccess, setShowAccess] = useState(false);
  const rent = latestRentChange(rentChanges, tenant.unitId, "rent");
  const prepayment = latestRentChange(rentChanges, tenant.unitId, "prepayment");
  const second = secondTenantName(tenant);
  // The access panel names the tenant without salutation, as it did before the redesign.
  const accessName = `${tenant.firstName} ${tenant.lastName}`;

  return (
    <Sheet
      title={tenantName(tenant)}
      sub={`${tenant.unit?.name ?? ""} · ${propertyLabel(tenant)}`}
      wide
      onClose={onClose}
      footer={
        // Same shape as `SheetFooter`: buttons fill the footer width.
        <>
          <Button variant="danger" className="h-12 flex-1" onClick={onDelete}>
            Löschen
          </Button>
          <Button variant="secondary" className="h-12 flex-1" onClick={onClose}>
            Schließen
          </Button>
          <Button variant="primary" className="h-12 flex-1" onClick={onEdit}>
            Bearbeiten
          </Button>
        </>
      }
    >
      <div className="grid gap-6 p-6">
        <TenantTags tenant={tenant} />
        <KeyValue
          items={[
            second ? ["Zweiter Mieter", second] : null,
            ["Einzug", formatDate(tenant.moveInDate)],
            tenant.moveOutDate ? ["Auszug", formatDate(tenant.moveOutDate)] : null,
            ["Vertragsart", leaseText(tenant)],
            tenant.phone ? ["Telefon", tenant.phone] : null,
            tenant.email ? ["E-Mail", tenant.email] : null,
            tenant.bankName ? ["Bank", tenant.bankName] : null,
            tenant.iban ? ["IBAN", <span key="iban" className="num">{tenant.iban}</span>] : null,
            tenant.accountHolder ? ["Kontoinhaber", tenant.accountHolder] : null,
          ]}
        />

        <Block
          title="Miete und Vorauszahlung"
          actions={
            <Button size="sm" variant="secondary" onClick={onAdjust}>
              Anpassen
            </Button>
          }
        >
          <KeyValue
            items={[
              ["Kaltmiete", <span key="rent" className="num">{rent ? formatCurrency(rent.amount) : "–"}</span>],
              [
                "NK-Vorauszahlung",
                <span key="nk" className="num">{prepayment ? formatCurrency(prepayment.amount) : "–"}</span>,
              ],
            ]}
          />
          <RentHistory changes={rentHistory(rentChanges, tenant.unitId)} />
        </Block>

        <Block
          title="App-Zugang"
          actions={
            !showAccess && (
              <Button size="sm" variant="secondary" onClick={() => setShowAccess(true)}>
                Anzeigen
              </Button>
            )
          }
        >
          {showAccess ? (
            <TenantAccessToken tenantId={tenant.id} tenantName={accessName} onClose={() => setShowAccess(false)} />
          ) : (
            <p className="text-sm text-ink3">Zugangscode, mit dem sich der Mieter in der VermieterMe-App anmeldet.</p>
          )}
        </Block>

        <Block title="Mietvertrag & Dokumente">
          <DocumentUpload
            tenantId={tenant.id}
            category="contract"
            label="Dokument hochladen"
            accept=".pdf,.jpg,.jpeg,.png,.webp"
          />
        </Block>
      </div>
    </Sheet>
  );
}
