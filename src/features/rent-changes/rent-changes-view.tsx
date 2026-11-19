"use client";

import { useState } from "react";
import { Button, EmptyState, LoadingState, PageHeader, useConfirm } from "@zweiund40gmbh/ui";
import type { RentChangeWithUnit } from "@/types";
import { RentChangeFormModal } from "./rent-change-form-modal";
import { RentChangeGroupList } from "./rent-change-groups";
import { groupRentChanges } from "./rent-change-utils";
import { useRentChanges } from "./use-rent-changes";

/** Rent changes page: grouped tables plus a dialog for creating and editing. */
export function RentChangesView() {
  const confirm = useConfirm();
  const { rentChanges, properties, loading, create, update, remove } = useRentChanges();
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<RentChangeWithUnit | null>(null);

  const unitOptions = properties.flatMap((prop) =>
    (prop.units || []).map((unit) => ({ value: unit.id, label: `${prop.street}, ${prop.city} – ${unit.name}` }))
  );

  function openForm(change: RentChangeWithUnit | null) {
    setEditing(change);
    setFormOpen(true);
  }

  async function handleDelete(change: RentChangeWithUnit) {
    const ok = await confirm({
      title: "Anpassung löschen",
      message: "Möchten Sie diese Mietanpassung wirklich löschen?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (ok) await remove(change.id);
  }

  return (
    <div className="grid gap-6">
      <PageHeader
        title="Mietanpassungen"
        actions={
          <Button size="sm" variant="primary" onClick={() => openForm(null)}>
            Neue Anpassung
          </Button>
        }
      />
      {loading ? (
        <LoadingState />
      ) : rentChanges.length === 0 ? (
        <EmptyState title="Noch keine Mietanpassungen vorhanden. Legen Sie Ihre erste Anpassung an." />
      ) : (
        <RentChangeGroupList groups={groupRentChanges(rentChanges)} onEdit={openForm} onDelete={handleDelete} />
      )}
      <RentChangeFormModal
        open={formOpen}
        editing={editing}
        unitOptions={unitOptions}
        onClose={() => setFormOpen(false)}
        onSubmit={(payload) => (editing ? update(editing.id, payload) : create(payload))}
      />
    </div>
  );
}
