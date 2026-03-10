"use client";

import { useEffect, useState } from "react";
import { Nav } from "@/components/nav";
import { formatDate, formatCurrency } from "@/lib/format";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import type { RentChangeWithUnit, PropertyWithUnits } from "@/types";

const TYPE_OPTIONS: ComboboxOption[] = [
  { value: "rent", label: "Kaltmiete" },
  { value: "prepayment", label: "Vorauszahlung NK" },
];

export default function RentChangesPage() {
  const [rentChanges, setRentChanges] = useState<RentChangeWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [loading, setLoading] = useState(true);
  const [showNewForm, setShowNewForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const [form, setForm] = useState({
    unitId: "",
    type: "rent" as "rent" | "prepayment",
    amount: "",
    effectiveDate: "",
    reason: "",
  });

  useEffect(() => {
    fetchData();
  }, []);

  async function fetchData() {
    try {
      const [changesRes, propsRes] = await Promise.all([
        fetch("/api/rent-changes"),
        fetch("/api/properties"),
      ]);

      if (changesRes.ok) {
        setRentChanges(await changesRes.json());
      }

      if (propsRes.ok) {
        const props = await propsRes.json();
        const detailPromises = props.map((p: PropertyWithUnits) =>
          fetch(`/api/properties/${p.id}`).then((r) => r.json())
        );
        const details = await Promise.all(detailPromises);
        setProperties(details);
      }
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      setLoading(false);
    }
  }

  async function fetchRentChanges() {
    try {
      const res = await fetch("/api/rent-changes");
      if (res.ok) {
        setRentChanges(await res.json());
      }
    } catch (error) {
      console.error("Failed to fetch rent changes:", error);
    }
  }

  function resetForm() {
    setForm({
      unitId: "",
      type: "rent",
      amount: "",
      effectiveDate: "",
      reason: "",
    });
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    try {
      const payload = {
        unitId: form.unitId,
        type: form.type,
        amount: parseFloat(form.amount),
        effectiveDate: form.effectiveDate,
        reason: form.reason || null,
      };
      const res = await fetch("/api/rent-changes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (res.ok) {
        resetForm();
        setShowNewForm(false);
        await fetchRentChanges();
      }
    } catch (error) {
      console.error("Failed to create rent change:", error);
    }
  }

  async function handleUpdate(id: string) {
    try {
      const payload = {
        unitId: form.unitId,
        type: form.type,
        amount: parseFloat(form.amount),
        effectiveDate: form.effectiveDate,
        reason: form.reason || null,
      };
      const res = await fetch(`/api/rent-changes/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (res.ok) {
        setEditingId(null);
        resetForm();
        await fetchRentChanges();
      }
    } catch (error) {
      console.error("Failed to update rent change:", error);
    }
  }

  async function handleDelete(id: string) {
    setDeleteTarget(null);
    try {
      const res = await fetch(`/api/rent-changes/${id}`, { method: "DELETE" });
      if (res.ok) {
        await fetchRentChanges();
      }
    } catch (error) {
      console.error("Failed to delete rent change:", error);
    }
  }

  function startEdit(change: RentChangeWithUnit) {
    setEditingId(change.id);
    setShowNewForm(false);
    setForm({
      unitId: change.unitId,
      type: change.type,
      amount: String(change.amount),
      effectiveDate: change.effectiveDate.split("T")[0],
      reason: change.reason || "",
    });
  }

  // Unit options for Combobox
  const unitOptions: ComboboxOption[] = properties.flatMap((prop) =>
    (prop.units || []).map((unit) => ({
      value: unit.id,
      label: `${prop.street}, ${prop.city} – ${unit.name}`,
      group: `${prop.street}, ${prop.city}`,
    }))
  );

  // Group rent changes by property, then by unit, sorted by effectiveDate desc
  const grouped = rentChanges.reduce(
    (acc, change) => {
      const propKey = change.unit?.property
        ? `${change.unit.property.street}, ${change.unit.property.city}`
        : "Unbekanntes Objekt";
      const unitKey = change.unit?.name || "Unbekannte Wohnung";

      if (!acc[propKey]) {
        acc[propKey] = {};
      }
      if (!acc[propKey][unitKey]) {
        acc[propKey][unitKey] = [];
      }
      acc[propKey][unitKey].push(change);
      return acc;
    },
    {} as Record<string, Record<string, RentChangeWithUnit[]>>
  );

  // Sort each unit's changes by effectiveDate desc
  for (const propKey of Object.keys(grouped)) {
    for (const unitKey of Object.keys(grouped[propKey])) {
      grouped[propKey][unitKey].sort(
        (a, b) =>
          new Date(b.effectiveDate).getTime() -
          new Date(a.effectiveDate).getTime()
      );
    }
  }

  function renderForm(
    onSubmit: (e: React.FormEvent) => void,
    onCancel: () => void,
    submitLabel: string
  ) {
    return (
      <form
        onSubmit={onSubmit}
        className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
      >
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {/* Wohnung */}
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Wohnung
            </label>
            <Combobox
              options={unitOptions}
              value={form.unitId}
              onChange={(val) => setForm({ ...form, unitId: val })}
              placeholder="Wohnung wählen..."
              searchPlaceholder="Wohnung suchen..."
              required
            />
          </div>

          {/* Typ */}
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Typ
            </label>
            <Combobox
              options={TYPE_OPTIONS}
              value={form.type}
              onChange={(val) =>
                setForm({ ...form, type: val as "rent" | "prepayment" })
              }
              placeholder="Typ wählen..."
              required
            />
          </div>

          {/* Betrag */}
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Betrag (EUR)
            </label>
            <input
              type="number"
              required
              step="0.01"
              min="0"
              value={form.amount}
              onChange={(e) => setForm({ ...form, amount: e.target.value })}
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </div>

          {/* Gültig ab */}
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Gültig ab
            </label>
            <input
              type="date"
              required
              value={form.effectiveDate}
              onChange={(e) =>
                setForm({ ...form, effectiveDate: e.target.value })
              }
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </div>

          {/* Grund */}
          <div className="sm:col-span-2">
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Grund (optional)
            </label>
            <input
              type="text"
              value={form.reason}
              onChange={(e) => setForm({ ...form, reason: e.target.value })}
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </div>
        </div>

        <div className="mt-4 flex gap-2">
          <button
            type="submit"
            className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
          >
            {submitLabel}
          </button>
          <button
            type="button"
            onClick={onCancel}
            className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
          >
            Abbrechen
          </button>
        </div>
      </form>
    );
  }

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-2xl font-bold text-zinc-900">Mietanpassungen</h1>
          <button
            onClick={() => {
              setShowNewForm(!showNewForm);
              setEditingId(null);
              resetForm();
            }}
            className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
          >
            Neue Anpassung
          </button>
        </div>

        {showNewForm && (
          <div className="mb-6">
            <h3 className="mb-4 text-lg font-semibold text-zinc-900">
              Neue Anpassung anlegen
            </h3>
            {renderForm(handleCreate, () => setShowNewForm(false), "Speichern")}
          </div>
        )}

        {loading ? (
          <Loading />
        ) : rentChanges.length === 0 ? (
          <EmptyState message="Noch keine Mietanpassungen vorhanden. Legen Sie Ihre erste Anpassung an." />
        ) : (
          <div className="space-y-8">
            {Object.entries(grouped).map(([propertyName, units]) => (
              <div key={propertyName}>
                <h2 className="mb-3 text-lg font-semibold text-zinc-900">
                  {propertyName}
                </h2>
                <div className="space-y-4">
                  {Object.entries(units).map(([unitName, changes]) => (
                    <div key={unitName}>
                      <h3 className="mb-2 text-sm font-semibold text-zinc-700">
                        {unitName}
                      </h3>
                      <div className="space-y-3">
                        {changes.map((change) =>
                          editingId === change.id ? (
                            <div key={change.id}>
                              <h3 className="mb-3 text-sm font-semibold text-zinc-700">
                                Anpassung bearbeiten
                              </h3>
                              {renderForm(
                                (e) => {
                                  e.preventDefault();
                                  handleUpdate(change.id);
                                },
                                () => {
                                  setEditingId(null);
                                  resetForm();
                                },
                                "Aktualisieren"
                              )}
                            </div>
                          ) : (
                            <div
                              key={change.id}
                              className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
                            >
                              <div className="flex items-start justify-between">
                                <div className="flex items-center gap-3">
                                  <span
                                    className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium ${
                                      change.type === "rent"
                                        ? "bg-blue-100 text-blue-700"
                                        : "bg-orange-100 text-orange-700"
                                    }`}
                                  >
                                    {change.type === "rent"
                                      ? "Kaltmiete"
                                      : "Vorauszahlung NK"}
                                  </span>
                                  <span className="font-medium text-zinc-900">
                                    {formatCurrency(change.amount)}
                                  </span>
                                  <span className="text-sm text-zinc-500">
                                    ab {formatDate(change.effectiveDate)}
                                  </span>
                                  {change.reason && (
                                    <span className="text-sm text-zinc-500">
                                      – {change.reason}
                                    </span>
                                  )}
                                </div>
                                <div className="flex gap-1">
                                  <button
                                    onClick={() => startEdit(change)}
                                    className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                                  >
                                    Bearbeiten
                                  </button>
                                  <button
                                    onClick={() => setDeleteTarget(change.id)}
                                    className="rounded-lg border border-red-200 px-3 py-1.5 text-xs font-medium text-red-600 hover:bg-red-50"
                                  >
                                    Löschen
                                  </button>
                                </div>
                              </div>
                            </div>
                          )
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
        <ConfirmDialog
          open={deleteTarget !== null}
          onOpenChange={(open) => !open && setDeleteTarget(null)}
          onConfirm={() => {
            if (deleteTarget) handleDelete(deleteTarget);
          }}
          title="Anpassung löschen"
          description="Möchten Sie diese Mietanpassung wirklich löschen?"
        />
      </main>
    </>
  );
}
