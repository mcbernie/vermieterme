"use client";

import { useState } from "react";
import { Button, EmptyState, LoadingState, PageHeader, useConfirm } from "@zweiund40gmbh/ui";
import type { PropertyWithCount, UnitWithTenants } from "@/types";
import { PropertyCard } from "./property-card";
import { PropertyFormModal } from "./property-form-modal";
import { UnitFormModal } from "./unit-form-modal";
import { useProperties } from "./use-properties";

/** Open dialog: a property form or a unit form (missing entity = create). */
type Dialog =
  | { kind: "property"; property?: PropertyWithCount }
  | { kind: "unit"; propertyId: string; unit?: UnitWithTenants }
  | null;

/** Objects and units overview with create, edit and delete dialogs. */
export function PropertiesView() {
  const data = useProperties();
  const confirm = useConfirm();
  const [dialog, setDialog] = useState<Dialog>(null);

  async function removeProperty(id: string) {
    const ok = await confirm({
      title: "Objekt löschen",
      message: "Alle zugehörigen Daten werden ebenfalls gelöscht. Möchten Sie fortfahren?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (ok) await data.deleteProperty(id);
  }

  async function removeUnit(unitId: string, propertyId: string) {
    const ok = await confirm({
      title: "Wohnung löschen",
      message: "Möchten Sie diese Wohnung wirklich löschen?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (ok) await data.deleteUnit(unitId, propertyId);
  }

  return (
    <>
      <PageHeader
        title="Objekte"
        actions={
          <Button variant="primary" onClick={() => setDialog({ kind: "property" })}>
            Neues Objekt
          </Button>
        }
      />

      {data.loading ? (
        <LoadingState />
      ) : data.properties.length === 0 ? (
        <EmptyState title="Noch keine Objekte vorhanden. Erstellen Sie Ihr erstes Objekt." icon={null} />
      ) : (
        <div className="grid gap-4">
          {data.properties.map((property) => (
            <PropertyCard
              key={property.id}
              property={property}
              expanded={data.expandedId === property.id}
              detail={data.detail}
              detailFailed={data.detailFailed}
              rentChanges={data.rentChanges}
              onToggle={() => data.toggleExpand(property.id)}
              onEdit={() => setDialog({ kind: "property", property })}
              onDelete={() => removeProperty(property.id)}
              onAddUnit={() => setDialog({ kind: "unit", propertyId: property.id })}
              onEditUnit={(unit) => setDialog({ kind: "unit", propertyId: property.id, unit })}
              onDeleteUnit={(unit) => removeUnit(unit.id, property.id)}
            />
          ))}
        </div>
      )}

      {dialog?.kind === "property" && (
        <PropertyFormModal
          property={dialog.property}
          onSave={(values) => data.saveProperty(values, dialog.property?.id)}
          onClose={() => setDialog(null)}
        />
      )}
      {dialog?.kind === "unit" && (
        <UnitFormModal
          unit={dialog.unit}
          onSave={(values) => data.saveUnit({ propertyId: dialog.propertyId, values, unitId: dialog.unit?.id })}
          onClose={() => setDialog(null)}
        />
      )}
    </>
  );
}
