"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, Modal, TextInput } from "@zweiund40gmbh/ui";
import type { Property } from "@/types";
import { useSave } from "./use-save";
import type { PropertyFormValues } from "./use-properties";

const EMPTY: PropertyFormValues = { street: "", zip: "", city: "", totalShares: 100 };

interface Props {
  /** Property to edit; undefined creates a new one. */
  property?: Property;
  onSave: (values: PropertyFormValues) => Promise<boolean>;
  onClose: () => void;
}

/** Dialog to create or edit a property. Mount it only while it should be open. */
export function PropertyFormModal({ property, onSave, onClose }: Props) {
  const [form, setForm] = useState<PropertyFormValues>(
    property
      ? { street: property.street, zip: property.zip, city: property.city, totalShares: property.totalShares }
      : EMPTY,
  );
  const { saving, error, run } = useSave();

  function submit(e: FormEvent) {
    e.preventDefault();
    void run(() => onSave(form), onClose);
  }

  return (
    <Modal
      title={property ? "Objekt bearbeiten" : "Neues Objekt anlegen"}
      onClose={onClose}
      preventClose={saving}
      footer={
        <>
          <Button type="button" variant="secondary" onClick={onClose} disabled={saving}>
            Abbrechen
          </Button>
          <Button type="submit" form="property-form" variant="primary" disabled={saving}>
            Speichern
          </Button>
        </>
      }
    >
      <form id="property-form" onSubmit={submit} className="grid gap-4 sm:grid-cols-2">
        <Field label="Straße" required className="sm:col-span-2">
          <TextInput required value={form.street} onChange={(e) => setForm({ ...form, street: e.target.value })} />
        </Field>
        <Field label="PLZ" required>
          <TextInput required value={form.zip} onChange={(e) => setForm({ ...form, zip: e.target.value })} />
        </Field>
        <Field label="Stadt" required>
          <TextInput required value={form.city} onChange={(e) => setForm({ ...form, city: e.target.value })} />
        </Field>
        <Field label="Gesamtanteile (MEA)" required>
          <TextInput
            type="number"
            required
            value={form.totalShares}
            onChange={(e) => setForm({ ...form, totalShares: parseInt(e.target.value) || 0 })}
          />
        </Field>
        {error && (
          <InlineNotice tone="bad" className="sm:col-span-2">
            {error}
          </InlineNotice>
        )}
      </form>
    </Modal>
  );
}
