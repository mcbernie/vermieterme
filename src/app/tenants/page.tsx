"use client";

import { useEffect, useState } from "react";
import { Nav } from "@/components/nav";
import { formatDate, formatCurrency } from "@/lib/format";
import { Combobox, type ComboboxOption } from "@/components/ui/combobox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import { OptionalDateInput } from "@/components/ui/optional-date-input";
import { SALUTATIONS, SALUTATIONS_SECONDARY } from "@/lib/constants";
import { validateIBAN, formatIBAN } from "@/lib/iban";
import { DocumentUpload } from "@/components/document-upload";
import type { TenantWithUnit, PropertyWithUnits, RentChangeWithUnit } from "@/types";

const leaseTypeOptions: ComboboxOption[] = [
  { value: "standard", label: "Standard" },
  { value: "index", label: "Indexmiete" },
  { value: "staffel", label: "Staffelmiete" },
];

export default function TenantsPage() {
  const [tenants, setTenants] = useState<TenantWithUnit[]>([]);
  const [properties, setProperties] = useState<PropertyWithUnits[]>([]);
  const [rentChanges, setRentChanges] = useState<RentChangeWithUnit[]>([]);
  const [loading, setLoading] = useState(true);
  const [showNewForm, setShowNewForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [expandedTenant, setExpandedTenant] = useState<string | null>(null);
  const [adjustTarget, setAdjustTarget] = useState<string | null>(null);
  const [adjustForm, setAdjustForm] = useState({
    type: "prepayment" as "rent" | "prepayment",
    amount: "",
    effectiveDate: new Date().toISOString().split("T")[0],
    reason: "",
  });

  const [form, setForm] = useState({
    unitId: "",
    salutation: "Herr",
    firstName: "",
    lastName: "",
    salutation2: "",
    firstName2: "",
    lastName2: "",
    phone: "",
    email: "",
    bankName: "",
    iban: "",
    accountHolder: "",
    moveInDate: "",
    moveOutDate: "",
    leaseType: "standard",
    indexBaseYear: "",
    indexReferenceValue: "",
    indexReferenceDate: "",
    indexMinMonths: "12",
  });

  useEffect(() => {
    fetchData();
  }, []);

  async function fetchData() {
    try {
      const [tenantsRes, propsRes, rentChangesRes] = await Promise.all([
        fetch("/api/tenants"),
        fetch("/api/properties"),
        fetch("/api/rent-changes"),
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

      if (rentChangesRes.ok) {
        setRentChanges(await rentChangesRes.json());
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
      phone: "",
      email: "",
      bankName: "",
      iban: "",
      accountHolder: "",
      moveInDate: "",
      moveOutDate: "",
      leaseType: "standard",
      indexBaseYear: "",
      indexReferenceValue: "",
      indexReferenceDate: "",
      indexMinMonths: "12",
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
        phone: form.phone || null,
        email: form.email || null,
        bankName: form.bankName || null,
        iban: form.iban || null,
        accountHolder: form.accountHolder || null,
        moveOutDate: form.moveOutDate || null,
        leaseType: form.leaseType,
        indexBaseYear: form.indexBaseYear ? parseInt(form.indexBaseYear) : null,
        indexReferenceValue: form.indexReferenceValue ? parseFloat(form.indexReferenceValue) : null,
        indexReferenceDate: form.indexReferenceDate || null,
        indexMinMonths: parseInt(form.indexMinMonths) || 12,
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
        phone: form.phone || null,
        email: form.email || null,
        bankName: form.bankName || null,
        iban: form.iban || null,
        accountHolder: form.accountHolder || null,
        moveOutDate: form.moveOutDate || null,
        leaseType: form.leaseType,
        indexBaseYear: form.indexBaseYear ? parseInt(form.indexBaseYear) : null,
        indexReferenceValue: form.indexReferenceValue ? parseFloat(form.indexReferenceValue) : null,
        indexReferenceDate: form.indexReferenceDate || null,
        indexMinMonths: parseInt(form.indexMinMonths) || 12,
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

  async function handleAdjustSubmit(unitId: string) {
    try {
      const res = await fetch("/api/rent-changes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          unitId,
          type: adjustForm.type,
          amount: parseFloat(adjustForm.amount),
          effectiveDate: adjustForm.effectiveDate,
          reason: adjustForm.reason || null,
        }),
      });
      if (res.ok) {
        setAdjustTarget(null);
        setAdjustForm({
          type: "prepayment",
          amount: "",
          effectiveDate: new Date().toISOString().split("T")[0],
          reason: "",
        });
        // Refresh rent changes
        const rcRes = await fetch("/api/rent-changes");
        if (rcRes.ok) setRentChanges(await rcRes.json());
      }
    } catch (error) {
      console.error("Failed to save adjustment:", error);
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
      phone: tenant.phone || "",
      email: tenant.email || "",
      bankName: tenant.bankName || "",
      iban: tenant.iban || "",
      accountHolder: tenant.accountHolder || "",
      moveInDate: tenant.moveInDate.split("T")[0],
      moveOutDate: tenant.moveOutDate ? tenant.moveOutDate.split("T")[0] : "",
      leaseType: tenant.leaseType || "standard",
      indexBaseYear: tenant.indexBaseYear ? String(tenant.indexBaseYear) : "",
      indexReferenceValue: tenant.indexReferenceValue ? String(tenant.indexReferenceValue) : "",
      indexReferenceDate: tenant.indexReferenceDate ? tenant.indexReferenceDate.split("T")[0] : "",
      indexMinMonths: String(tenant.indexMinMonths ?? 12),
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

  function getLatestRentChange(unitId: string, type: "rent" | "prepayment") {
    const matches = rentChanges
      .filter((rc) => rc.unitId === unitId && rc.type === type)
      .sort((a, b) => new Date(b.effectiveDate).getTime() - new Date(a.effectiveDate).getTime());
    return matches.length > 0 ? matches[0] : null;
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

        {/* Mietvertrag */}
        <div className="mt-5 border-t border-zinc-100 pt-4">
          <p className="mb-3 text-sm font-medium text-zinc-500">
            Mietvertrag
          </p>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Vertragsart
              </label>
              <Combobox
                options={leaseTypeOptions}
                value={form.leaseType}
                onChange={(val) => setForm({ ...form, leaseType: val })}
                placeholder="Vertragsart wählen..."
              />
            </div>
          </div>
          {form.leaseType === "index" && (
            <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  VPI Basisjahr
                </label>
                <input
                  type="number"
                  value={form.indexBaseYear}
                  onChange={(e) => setForm({ ...form, indexBaseYear: e.target.value })}
                  placeholder="z.B. 2020"
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  VPI Referenzwert
                </label>
                <input
                  type="number"
                  step="0.1"
                  value={form.indexReferenceValue}
                  onChange={(e) => setForm({ ...form, indexReferenceValue: e.target.value })}
                  placeholder="z.B. 105.4"
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Referenzdatum (letzte Anpassung)
                </label>
                <OptionalDateInput
                  value={form.indexReferenceDate}
                  onChange={(v) => setForm({ ...form, indexReferenceDate: v })}
                  placeholder="Noch nicht angepasst"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Min. Monate
                </label>
                <input
                  type="number"
                  value={form.indexMinMonths}
                  onChange={(e) => setForm({ ...form, indexMinMonths: e.target.value })}
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
            </div>
          )}
        </div>

        {/* Kontaktdaten */}
        <div className="mt-5 border-t border-zinc-100 pt-4">
          <p className="mb-3 text-sm font-medium text-zinc-500">
            Kontaktdaten
          </p>
          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Telefon / Handy
              </label>
              <input
                type="tel"
                value={form.phone}
                onChange={(e) => setForm({ ...form, phone: e.target.value })}
                placeholder="z.B. +49 170 1234567"
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                E-Mail
              </label>
              <input
                type="email"
                value={form.email}
                onChange={(e) => setForm({ ...form, email: e.target.value })}
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </div>
          </div>
        </div>

        {/* Bankverbindung */}
        <div className="mt-5 border-t border-zinc-100 pt-4">
          <p className="mb-3 text-sm font-medium text-zinc-500">
            Bankverbindung (für Gutschriften)
          </p>
          <div className="grid gap-4 sm:grid-cols-3">
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Bankname
              </label>
              <input
                type="text"
                value={form.bankName}
                onChange={(e) => setForm({ ...form, bankName: e.target.value })}
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                IBAN
              </label>
              <input
                type="text"
                value={form.iban}
                onChange={(e) => setForm({ ...form, iban: e.target.value })}
                onBlur={() => {
                  if (form.iban) {
                    setForm({ ...form, iban: formatIBAN(form.iban) });
                  }
                }}
                className={`w-full rounded-lg border px-3 py-2 text-sm focus:outline-none focus:ring-1 ${
                  form.iban && !validateIBAN(form.iban)
                    ? "border-red-300 focus:border-red-500 focus:ring-red-500"
                    : "border-zinc-300 focus:border-zinc-500 focus:ring-zinc-500"
                }`}
              />
              {form.iban && !validateIBAN(form.iban) && (
                <p className="mt-1 text-xs text-red-600">Ungültige IBAN</p>
              )}
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-zinc-700">
                Kontoinhaber
              </label>
              <input
                type="text"
                value={form.accountHolder}
                onChange={(e) => setForm({ ...form, accountHolder: e.target.value })}
                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </div>
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
                                {tenant.leaseType === "index" && (
                                  <span className="inline-flex rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                                    Indexmiete
                                  </span>
                                )}
                                {tenant.leaseType === "staffel" && (
                                  <span className="inline-flex rounded-full bg-purple-100 px-2 py-0.5 text-xs font-medium text-purple-700">
                                    Staffelmiete
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
                              <div className="mt-2 flex flex-wrap gap-4 text-sm text-zinc-500">
                                <span>
                                  Einzug: {formatDate(tenant.moveInDate)}
                                </span>
                                {tenant.moveOutDate && (
                                  <span>
                                    Auszug: {formatDate(tenant.moveOutDate)}
                                  </span>
                                )}
                                {tenant.phone && (
                                  <span>Tel: {tenant.phone}</span>
                                )}
                                {tenant.email && (
                                  <span>{tenant.email}</span>
                                )}
                              </div>
                              {(() => {
                                const latestRent = getLatestRentChange(tenant.unitId, "rent");
                                const latestPrepayment = getLatestRentChange(tenant.unitId, "prepayment");
                                return (
                                  <div className="mt-2 flex items-center gap-4 text-sm text-zinc-500">
                                    <span>
                                      Kaltmiete: {latestRent ? formatCurrency(latestRent.amount) : "–"}
                                    </span>
                                    <span>
                                      NK-Vorauszahlung: {latestPrepayment ? formatCurrency(latestPrepayment.amount) : "–"}
                                    </span>
                                    <button
                                      onClick={() => {
                                        setAdjustTarget(adjustTarget === tenant.id ? null : tenant.id);
                                        setAdjustForm({
                                          type: latestRent ? "prepayment" : "rent",
                                          amount: "",
                                          effectiveDate: new Date().toISOString().split("T")[0],
                                          reason: "",
                                        });
                                      }}
                                      className="rounded border border-zinc-300 px-2 py-0.5 text-xs font-medium text-zinc-600 hover:bg-zinc-50"
                                    >
                                      Anpassen
                                    </button>
                                  </div>
                                );
                              })()}
                            </div>
                            <div className="flex gap-1">
                              <button
                                onClick={() =>
                                  setExpandedTenant(
                                    expandedTenant === tenant.id ? null : tenant.id
                                  )
                                }
                                className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                              >
                                {expandedTenant === tenant.id ? "Weniger" : "Dokumente"}
                              </button>
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

                          {/* NK-Anpassung */}
                          {adjustTarget === tenant.id && (
                            <div className="mt-4 rounded-lg border border-zinc-200 bg-zinc-50 p-4">
                              <p className="mb-3 text-sm font-medium text-zinc-700">
                                Miete / Vorauszahlung anpassen
                              </p>
                              <div className="grid gap-3 sm:grid-cols-4">
                                <div>
                                  <label className="mb-1 block text-xs font-medium text-zinc-500">
                                    Art
                                  </label>
                                  <Combobox
                                    options={[
                                      { value: "prepayment", label: "NK-Vorauszahlung" },
                                      { value: "rent", label: "Kaltmiete" },
                                    ]}
                                    value={adjustForm.type}
                                    onChange={(val) =>
                                      setAdjustForm({ ...adjustForm, type: val as "rent" | "prepayment" })
                                    }
                                  />
                                </div>
                                <div>
                                  <label className="mb-1 block text-xs font-medium text-zinc-500">
                                    Neuer Betrag
                                  </label>
                                  <input
                                    type="number"
                                    step="0.01"
                                    value={adjustForm.amount}
                                    onChange={(e) =>
                                      setAdjustForm({ ...adjustForm, amount: e.target.value })
                                    }
                                    placeholder="0,00"
                                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                                  />
                                </div>
                                <div>
                                  <label className="mb-1 block text-xs font-medium text-zinc-500">
                                    Gültig ab
                                  </label>
                                  <input
                                    type="date"
                                    value={adjustForm.effectiveDate}
                                    onChange={(e) =>
                                      setAdjustForm({ ...adjustForm, effectiveDate: e.target.value })
                                    }
                                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                                  />
                                </div>
                                <div>
                                  <label className="mb-1 block text-xs font-medium text-zinc-500">
                                    Grund (optional)
                                  </label>
                                  <input
                                    type="text"
                                    value={adjustForm.reason}
                                    onChange={(e) =>
                                      setAdjustForm({ ...adjustForm, reason: e.target.value })
                                    }
                                    placeholder="z.B. Anpassung NK"
                                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                                  />
                                </div>
                              </div>
                              <div className="mt-3 flex gap-2">
                                <button
                                  onClick={() => handleAdjustSubmit(tenant.unitId)}
                                  disabled={!adjustForm.amount}
                                  className="rounded-lg bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-800 disabled:opacity-50"
                                >
                                  Speichern
                                </button>
                                <button
                                  onClick={() => setAdjustTarget(null)}
                                  className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                                >
                                  Abbrechen
                                </button>
                              </div>
                            </div>
                          )}

                          {/* Mietvertrag-Upload */}
                          {expandedTenant === tenant.id && (
                            <div className="mt-4 border-t border-zinc-100 pt-4">
                              <p className="mb-2 text-sm font-medium text-zinc-700">
                                Mietvertrag & Dokumente
                              </p>
                              <DocumentUpload
                                tenantId={tenant.id}
                                category="contract"
                                label="Dokument hochladen"
                                accept=".pdf,.jpg,.jpeg,.png,.webp"
                              />
                            </div>
                          )}
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
