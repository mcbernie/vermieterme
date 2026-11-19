"use client";

import { useState } from "react";
import { Button, EmptyState, InlineNotice, Tag, useConfirm } from "@zweiund40gmbh/ui";
import type { CostCategory } from "@/types";
import { CostCategoryModal, type CostCategoryValues } from "./cost-category-modal";
import { requestJson } from "./request";
import { LoadGate, SettingsSection } from "./section";
import { useAction } from "./use-action";
import { LOAD_FAILED, useLoad } from "./use-load";

/** Loads the cost categories. */
const loadCategories = (signal: AbortSignal) =>
  requestJson<CostCategory[]>("/api/cost-categories", LOAD_FAILED, { signal });

/** Sort position after the last category (1 for an empty list). */
export function nextSortOrder(categories: CostCategory[]): number {
  return categories.length > 0 ? Math.max(...categories.map((c) => c.sortOrder)) + 1 : 1;
}

/** Open dialog: create (no category) or edit. */
type Dialog = { category?: CostCategory } | null;

/** Cost categories with create, edit and delete. */
export function CostCategoriesSection() {
  const { data, loading, error, refreshError, retry, refresh } = useLoad(loadCategories);
  const [dialog, setDialog] = useState<Dialog>(null);
  const remove = useAction("Fehler beim Löschen");
  const confirm = useConfirm();
  const categories = data ?? [];

  /** Creates or updates a category, then reloads the list. */
  async function save(values: CostCategoryValues, id?: string) {
    await requestJson(id ? `/api/cost-categories/${id}` : "/api/cost-categories", "Fehler beim Speichern", {
      method: id ? "PUT" : "POST",
      json: values,
    });
    await refresh();
  }

  async function askDelete(category: CostCategory) {
    const ok = await confirm({
      title: "Kostenart löschen",
      message: "Möchten Sie diese Kostenart wirklich löschen?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (!ok) return;
    await remove.run(async () => {
      await requestJson(`/api/cost-categories/${category.id}`, "Fehler beim Löschen", { method: "DELETE" });
      await refresh();
    });
  }

  return (
    <SettingsSection
      title="Kostenarten"
      actions={
        <Button variant="primary" onClick={() => setDialog({})} disabled={loading || !!error}>
          Neue Kostenart
        </Button>
      }
    >
      <LoadGate loading={loading} error={error} refreshError={refreshError} onRetry={retry}>
        {remove.error && <InlineNotice tone="bad">{remove.error}</InlineNotice>}
        {categories.length === 0 ? (
          <EmptyState icon={null} title="Noch keine Kostenarten vorhanden. Legen Sie Ihre erste Kostenart an." />
        ) : (
          <div className="overflow-x-auto border border-line bg-layer">
            <table className="table-plain">
              <thead>
                <tr>
                  <th className="text-right">Reihenfolge</th>
                  <th>Name</th>
                  <th>Verteilerschlüssel</th>
                  <th className="text-right">Aktionen</th>
                </tr>
              </thead>
              <tbody>
                {categories.map((category) => (
                  <tr key={category.id}>
                    <td className="num text-right text-ink2">{category.sortOrder}</td>
                    <td>{category.name}</td>
                    <td>
                      <Tag>{category.distributionKey}</Tag>
                    </td>
                    <td className="text-right">
                      <div className="flex justify-end gap-2">
                        <Button size="sm" variant="ghost" onClick={() => setDialog({ category })}>
                          Bearbeiten
                        </Button>
                        <Button
                          size="sm"
                          variant="danger"
                          disabled={remove.busy}
                          onClick={() => askDelete(category)}
                        >
                          Löschen
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </LoadGate>

      {dialog && (
        <CostCategoryModal
          category={dialog.category}
          nextSortOrder={nextSortOrder(categories)}
          onSave={(values) => save(values, dialog.category?.id)}
          onClose={() => setDialog(null)}
        />
      )}
    </SettingsSection>
  );
}
