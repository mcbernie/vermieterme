"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, Modal, TextInput } from "@zweiund40gmbh/ui";
import type { Unit } from "@/types";
import { useSave } from "./use-save";
import type { UnitFormValues } from "./use-properties";

const EMPTY: UnitFormValues = { name: "", floor: "", shares: 0 };

interface Props {
  /** Unit to edit; undefined creates a new one. */
  unit?: Unit;
  onSave: (values: UnitFormValues) => Promise<boolean>;
  onClose: () => void;
}

/** Dialog to create or edit a unit. Mount it only while it should be open. */
export function UnitFormModal({ unit, onSave, onClose }: Props) {
  const [form, setForm] = useState<UnitFormValues>(
    unit ? { name: unit.name, floor: unit.floor, shares: unit.shares } : EMPTY,
  );
  const { saving, error, run } = useSave();

  function submit(e: FormEvent) {
    e.preventDefault();
    void run(() => onSave(form), onClose);
  }

  return (
    <Modal
      title={unit ? "Wohnung bearbeiten" : "Wohnung hinzufügen"}
      onClose={onClose}
      preventClose={saving}
      footer={
        <>
          <Button type="button" variant="secondary" onClick={onClose} disabled={saving}>
            Abbrechen
          </Button>
          <Button type="submit" form="unit-form" variant="primary" disabled={saving}>
            Speichern
          </Button>
        </>
      }
    >
      <form id="unit-form" onSubmit={submit} className="grid gap-4">
        <Field label="Bezeichnung" required>
          <TextInput
            required
            value={form.name}
            placeholder="z.B. Wohnung EG links"
            onChange={(e) => setForm({ ...form, name: e.target.value })}
          />
        </Field>
        <Field label="Stockwerk" required>
          <TextInput
            required
            value={form.floor}
            placeholder="z.B. EG"
            onChange={(e) => setForm({ ...form, floor: e.target.value })}
          />
        </Field>
        <Field label="Anteile (MEA)" required>
          <TextInput
            type="number"
            required
            value={form.shares}
            onChange={(e) => setForm({ ...form, shares: parseInt(e.target.value) || 0 })}
          />
        </Field>
        {error && <InlineNotice tone="bad">{error}</InlineNotice>}
      </form>
    </Modal>
  );
}
