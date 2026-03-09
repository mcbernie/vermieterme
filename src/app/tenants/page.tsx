"use client";

import { useEffect, useState } from "react";
import { Nav } from "@/components/nav";
import { formatDate } from "@/lib/format";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import { OptionalDateInput } from "@/components/ui/optional-date-input";
import { SALUTATIONS, SALUTATIONS_SECONDARY } from "@/lib/constants";
import type { TenantWithUnit, PropertyWithUnits } from "@/types";

export default function TenantsPage() {
  const [tenants, setTenants] = useState<TenantWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [loading, setLoading] = useState(true);
  const [showNewForm, setShowNewForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const [form, setForm] = useState({
    unitId: "",
    salutation: "Herr",
    firstName: "",
    lastName: "",
    salutation2: "",
    firstName2: "",
    lastName2: "",
    moveInDate: "",
    moveOutDate: "",
  });

  useEffect(() => {
    fetchData();
  }, []);

  async function fetchData() {
    try {
      const [tenantsRes, propsRes] = await Promise.all([
        fetch("/api/tenants"),
        fetch("/api/properties"),
      ]);

      if (tenantsRes.ok) {
        setTenants(await tenantsRes.json());
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

  async function fetchTenants() {
    try {
      const res = await fetch("/api/tenants");
      if (res.ok) {
        setTenants(await res.json());
      }
    } catch (error) {
      console.error("Failed to fetch tenants:", error);
    }
  }

  function resetForm() {
    setForm({
      unitId: "",
      salutation: "Herr",
      firstName: "",
      lastName: "",
      salutation2: "",
      firstName2: "",
      lastName2: "",
      moveInDate: "",
      moveOutDate: "",
    });
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    try {
      const payload = {
        ...form,
        salutation2: form.salutation2 || null,
        firstName2: form.firstName2 || null,
        lastName2: form.lastName2 || null,
        moveOutDate: form.moveOutDate || null,
      };
      const res = await fetch("/api/tenants", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (res.ok) {
        resetForm();
        setShowNewForm(false);
        await fetchTenants();
      }
    } catch (error) {
      console.error("Failed to create tenant:", error);
    }
  }

  async function handleUpdate(id: string) {
    try {
      const payload = {
        ...form,
        salutation2: form.salutation2 || null,
        firstName2: form.firstName2 || null,
        lastName2: form.lastName2 || null,
        moveOutDate: form.moveOutDate || null,
      };
      const res = await fetch(`/api/tenants/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (res.ok) {
        setEditingId(null);
        resetForm();
        await fetchTenants();
      }
    } catch (error) {
      console.error("Failed to update tenant:", error);
    }
  }

  async function handleDelete(id: string) {
    setDeleteTarget(null);
    try {
      const res = await fetch(`/api/tenants/${id}`, { method: "DELETE" });
      if (res.ok) {
        await fetchTenants();
      }
    } catch (error) {
      console.error("Failed to delete tenant:", error);
    }
  }

  function startEdit(tenant: TenantWithUnit) {
    setEditingId(tenant.id);
    setShowNewForm(false);
    setForm({
      unitId: tenant.unitId,
      salutation: tenant.salutation,
      firstName: tenant.firstName,
      lastName: tenant.lastName,
      salutation2: tenant.salutation2 || "",
      firstName2: tenant.firstName2 || "",
      lastName2: tenant.lastName2 || "",
      moveInDate: tenant.moveInDate.split("T")[0],
      moveOutDate: tenant.moveOutDate ? tenant.moveOutDate.split("T")[0] : "",
    });
  }

  // Unit options for Combobox
  const unitOptions: ComboboxOption[] = properties.flatMap((prop) =>
    (prop.units || []).map((unit) => ({
      value: unit.id,
      label: `${unit.name} (${unit.floor})`,
      group: `${prop.street}, ${prop.city}`,
    }))
  );

  // Group tenants by property
  const tenantsByProperty = tenants.reduce(
    (groups, tenant) => {
      const propKey = tenant.unit?.property
        ? `${tenant.unit.property.street}, ${tenant.unit.property.city}`
        : "Unbekanntes Objekt";
      if (!groups[propKey]) {
        groups[propKey] = [];
      }
      groups[propKey].push(tenant);
      return groups;
    },
    {} as Record<string, TenantWithUnit[]>
  );

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
        {/* Wohnung */}
        <div className="mb-5">
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
            className="max-w-sm"
          />
        </div>

        {/* Mieter-Daten */}
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Anrede
            </label>
            <Combobox
              options={SALUTATIONS}
              value={form.salutation}
              onChange={(val) => setForm({ ...form, salutation: val })}
              placeholder="Anrede wählen..."
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Vorname
            </label>
            <input
              type="text"
              required
              value={form.firstName}
              onChange={(e) => setForm({ ...form, firstName: e.target.value })}
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Nachname
            </label>
            <input
              type="text"
              required
              value={form.lastName}
              onChange={(e) => setForm({ ...form, lastName: e.target.value })}
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </div>
        </div>

        <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Einzugsdatum
            </label>
            <input
              type="date"
              required
              value={form.moveInDate}
              onChange={(e) => setForm({ ...form, moveInDate: e.target.value })}
              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-zinc-700">
              Auszugsdatum
            </label>
            <OptionalDateInput
              value={form.moveOutDate}
              onChange={(v) => setForm({ ...form, moveOutDate: v })}
              placeholder="Noch nicht ausgezogen"
            />
          </div>
        </div>

        {/* Zweiter Mieter */}
        <div className="mt-5 border-t border-zinc-100 pt-4">
          <p className="mb-3 text-sm font-medium text-zinc-500">
            Zweiter Mieter (optional)
          </p>
          <div className="grid gap-4 sm:grid-cols-3">
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Anrede
              </label>
              <Combobox
                options={SALUTATIONS_SECONDARY}
                value={form.salutation2}
                onChange={(val) => setForm({ ...form, salutation2: val })}
                placeholder="---"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Vorname
              </label>
              <input
                type="text"
                value={form.firstName2}
                onChange={(e) =>
                  setForm({ ...form, firstName2: e.target.value })
                }
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Nachname
              </label>
              <input
                type="text"
                value={form.lastName2}
                onChange={(e) =>
                  setForm({ ...form, lastName2: e.target.value })
                }
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </div>
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
          <h1 className="text-2xl font-bold text-zinc-900">Mieter</h1>
          <button
            onClick={() => {
              setShowNewForm(!showNewForm);
              setEditingId(null);
              resetForm();
            }}
            className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
          >
            Neuer Mieter
          </button>
        </div>

        {showNewForm && (
          <div className="mb-6">
            <h3 className="mb-4 text-lg font-semibold text-zinc-900">
              Neuen Mieter anlegen
            </h3>
            {renderForm(handleCreate, () => setShowNewForm(false), "Speichern")}
          </div>
        )}

        {loading ? (
          <Loading />
        ) : tenants.length === 0 ? (
          <EmptyState message="Noch keine Mieter vorhanden. Legen Sie Ihren ersten Mieter an." />
        ) : (
          <div className="space-y-8">
            {Object.entries(tenantsByProperty).map(
              ([propertyName, groupedTenants]) => (
                <div key={propertyName}>
                  <h2 className="mb-3 text-lg font-semibold text-zinc-900">
                    {propertyName}
                  </h2>
                  <div className="space-y-3">
                    {groupedTenants.map((tenant) =>
                      editingId === tenant.id ? (
                        <div key={tenant.id}>
                          <h3 className="mb-3 text-sm font-semibold text-zinc-700">
                            Mieter bearbeiten
                          </h3>
                          {renderForm(
                            (e) => {
                              e.preventDefault();
                              handleUpdate(tenant.id);
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
                          key={tenant.id}
                          className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
                        >
                          <div className="flex items-start justify-between">
                            <div>
                              <div className="flex items-center gap-2">
                                <h3 className="font-medium text-zinc-900">
                                  {tenant.salutation} {tenant.firstName}{" "}
                                  {tenant.lastName}
                                </h3>
                                {!tenant.moveOutDate && (
                                  <span className="inline-flex rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700">
                                    Aktiv
                                  </span>
                                )}
                                {tenant.moveOutDate && (
                                  <span className="inline-flex rounded-full bg-zinc-100 px-2 py-0.5 text-xs font-medium text-zinc-600">
                                    Ausgezogen
                                  </span>
                                )}
                              </div>
                              {tenant.firstName2 && tenant.lastName2 && (
                                <p className="mt-0.5 text-sm text-zinc-600">
                                  {tenant.salutation2} {tenant.firstName2}{" "}
                                  {tenant.lastName2}
                                </p>
                              )}
                              <p className="mt-1 text-sm text-zinc-500">
                                {tenant.unit?.name} – {tenant.unit?.property?.street}
                              </p>
                              <div className="mt-2 flex gap-4 text-sm text-zinc-500">
                                <span>
                                  Einzug: {formatDate(tenant.moveInDate)}
                                </span>
                                {tenant.moveOutDate && (
                                  <span>
                                    Auszug: {formatDate(tenant.moveOutDate)}
                                  </span>
                                )}
                              </div>
                            </div>
                            <div className="flex gap-1">
                              <button
                                onClick={() => startEdit(tenant)}
                                className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                              >
                                Bearbeiten
                              </button>
                              <button
                                onClick={() => setDeleteTarget(tenant.id)}
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
              )
            )}
          </div>
        )}
        <ConfirmDialog
          open={deleteTarget !== null}
          onOpenChange={(open) => !open && setDeleteTarget(null)}
          onConfirm={() => {
            if (deleteTarget) handleDelete(deleteTarget);
          }}
          title="Mieter löschen"
          description="Möchten Sie diesen Mieter wirklich löschen?"
        />
      </main>
    </>
  );
}
