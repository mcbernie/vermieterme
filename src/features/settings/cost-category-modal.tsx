"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, Modal, Select, TextInput } from "@zweiund40gmbh/ui";
import { DISTRIBUTION_KEYS } from "@/lib/constants";
import type { CostCategory } from "@/types";
import { useAction } from "@/components/form/use-action";

/** Form values of a cost category. */
export interface CostCategoryValues {
  name: string;
  distributionKey: string;
  sortOrder: number;
}

const KEY_OPTIONS = DISTRIBUTION_KEYS.map((key) => ({ value: key, label: key }));

interface Props {
  /** Category to edit; undefined creates a new one. */
  category?: CostCategory;
  /** Sort position proposed for a new category. */
  nextSortOrder: number;
  /** Saves the values; throws a `RequestError` on failure. */
  onSave: (values: CostCategoryValues) => Promise<void>;
  onClose: () => void;
}

/** Dialog to create or edit a cost category. Mount it only while it should be open. */
export function CostCategoryModal({ category, nextSortOrder, onSave, onClose }: Props) {
  const [form, setForm] = useState<CostCategoryValues>(
    category
      ? { name: category.name, distributionKey: category.distributionKey, sortOrder: category.sortOrder }
      : { name: "", distributionKey: "MEA", sortOrder: nextSortOrder },
  );
  const [attempted, setAttempted] = useState(false);
  const { busy, error, run } = useAction("Fehler beim Speichern");
  const nameError = attempted && !form.name.trim() ? "Bitte einen Namen angeben." : undefined;

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    if (!form.name.trim()) return;
    if (await run(() => onSave(form))) onClose();
  }

  return (
    <Modal
      title={category ? "Kostenart bearbeiten" : "Neue Kostenart anlegen"}
      onClose={onClose}
      preventClose={busy}
      footer={
        <>
          <Button type="button" variant="secondary" onClick={onClose} disabled={busy}>
            Abbrechen
          </Button>
          <Button type="submit" form="cost-category-form" variant="primary" disabled={busy}>
            Speichern
          </Button>
        </>
      }
    >
      <form id="cost-category-form" noValidate onSubmit={submit} className="grid gap-4 sm:grid-cols-3">
        <Field label="Name" required error={nameError} className="sm:col-span-3">
          <TextInput
            value={form.name}
            placeholder="z.B. Grundsteuer"
            onChange={(e) => setForm({ ...form, name: e.target.value })}
          />
        </Field>
        <Field label="Verteilerschlüssel" required className="sm:col-span-2">
          <Select
            value={form.distributionKey}
            options={KEY_OPTIONS}
            onChange={(distributionKey) => setForm({ ...form, distributionKey })}
          />
        </Field>
        <Field label="Reihenfolge" required>
          <TextInput
            type="number"
            value={form.sortOrder}
            onChange={(e) => setForm({ ...form, sortOrder: parseInt(e.target.value) || 0 })}
          />
        </Field>
        {error && (
          <InlineNotice tone="bad" className="sm:col-span-3">
            {error}
          </InlineNotice>
        )}
      </form>
    </Modal>
  );
}
