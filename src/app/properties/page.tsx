"use client";

import { useEffect, useState } from "react";
import { Nav } from "@/components/nav";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { EmptyState } from "@/components/ui/empty-state";
import { formatCurrency } from "@/lib/format";
import type { PropertyWithCount, PropertyWithUnits, UnitWithTenants, Tenant, RentChange } from "@/types";

export default function PropertiesPage() {
  const [properties, setProperties] = useState<PropertyWithCount[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [propertyDetail, setPropertyDetail] = useState<PropertyWithUnits | null>(null);
  const [loading, setLoading] = useState(true);
  const [showNewPropertyForm, setShowNewPropertyForm] = useState(false);
  const [editingPropertyId, setEditingPropertyId] = useState<string | null>(null);
  const [showNewUnitForm, setShowNewUnitForm] = useState<string | null>(null);
  const [editingUnitId, setEditingUnitId] = useState<string | null>(null);
  const [deletePropertyId, setDeletePropertyId] = useState<string | null>(null);
  const [deleteUnitTarget, setDeleteUnitTarget] = useState<{ unitId: string; propertyId: string } | null>(null);
  const [rentChanges, setRentChanges] = useState<RentChange[]>([]);

  // Property form state
  const [propertyForm, setPropertyForm] = useState({
    street: "",
    zip: "",
    city: "",
    totalShares: 100,
  });

  // Unit form state
  const [unitForm, setUnitForm] = useState({
    name: "",
    floor: "",
    shares: 0,
  });

  useEffect(() => {
    fetchProperties();
    fetchRentChanges();
  }, []);

  async function fetchProperties() {
    try {
      const res = await fetch("/api/properties");
      if (res.ok) {
        setProperties(await res.json());
      }
    } catch (error) {
      console.error("Failed to fetch properties:", error);
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

  function getLatestRent(unitId: string): { rent: number | null; prepayment: number | null } {
    const unitChanges = rentChanges.filter((rc) => rc.unitId === unitId);
    const latestRent = unitChanges
      .filter((rc) => rc.type === "rent")
      .sort((a, b) => new Date(b.effectiveDate).getTime() - new Date(a.effectiveDate).getTime())[0];
    const latestPrepayment = unitChanges
      .filter((rc) => rc.type === "prepayment")
      .sort((a, b) => new Date(b.effectiveDate).getTime() - new Date(a.effectiveDate).getTime())[0];
    return {
      rent: latestRent?.amount ?? null,
      prepayment: latestPrepayment?.amount ?? null,
    };
  }

  async function fetchPropertyDetail(id: string) {
    try {
      const res = await fetch(`/api/properties/${id}`);
      if (res.ok) {
        setPropertyDetail(await res.json());
      }
    } catch (error) {
      console.error("Failed to fetch property detail:", error);
    }
  }

  async function toggleExpand(id: string) {
    if (expandedId === id) {
      setExpandedId(null);
      setPropertyDetail(null);
    } else {
      setExpandedId(id);
      await fetchPropertyDetail(id);
    }
  }

  async function handleCreateProperty(e: React.FormEvent) {
    e.preventDefault();
    try {
      const res = await fetch("/api/properties", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(propertyForm),
      });
      if (res.ok) {
        setPropertyForm({ street: "", zip: "", city: "", totalShares: 100 });
        setShowNewPropertyForm(false);
        await fetchProperties();
      }
    } catch (error) {
      console.error("Failed to create property:", error);
    }
  }

  async function handleUpdateProperty(id: string) {
    try {
      const res = await fetch(`/api/properties/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(propertyForm),
      });
      if (res.ok) {
        setEditingPropertyId(null);
        setPropertyForm({ street: "", zip: "", city: "", totalShares: 100 });
        await fetchProperties();
        if (expandedId === id) {
          await fetchPropertyDetail(id);
        }
      }
    } catch (error) {
      console.error("Failed to update property:", error);
    }
  }

  async function handleDeleteProperty(id: string) {
    try {
      const res = await fetch(`/api/properties/${id}`, { method: "DELETE" });
      if (res.ok) {
        if (expandedId === id) {
          setExpandedId(null);
          setPropertyDetail(null);
        }
        await fetchProperties();
      }
    } catch (error) {
      console.error("Failed to delete property:", error);
    }
  }

  function startEditProperty(property: PropertyWithCount) {
    setEditingPropertyId(property.id);
    setPropertyForm({
      street: property.street,
      zip: property.zip,
      city: property.city,
      totalShares: property.totalShares,
    });
  }

  async function handleCreateUnit(propertyId: string, e: React.FormEvent) {
    e.preventDefault();
    try {
      const res = await fetch("/api/units", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...unitForm, propertyId }),
      });
      if (res.ok) {
        setUnitForm({ name: "", floor: "", shares: 0 });
        setShowNewUnitForm(null);
        await fetchPropertyDetail(propertyId);
        await fetchProperties();
      }
    } catch (error) {
      console.error("Failed to create unit:", error);
    }
  }

  async function handleUpdateUnit(unitId: string, propertyId: string) {
    try {
      const res = await fetch(`/api/units/${unitId}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...unitForm, propertyId }),
      });
      if (res.ok) {
        setEditingUnitId(null);
        setUnitForm({ name: "", floor: "", shares: 0 });
        await fetchPropertyDetail(propertyId);
      }
    } catch (error) {
      console.error("Failed to update unit:", error);
    }
  }

  async function handleDeleteUnit(unitId: string, propertyId: string) {
    try {
      const res = await fetch(`/api/units/${unitId}`, { method: "DELETE" });
      if (res.ok) {
        await fetchPropertyDetail(propertyId);
        await fetchProperties();
      }
    } catch (error) {
      console.error("Failed to delete unit:", error);
    }
  }

  function startEditUnit(unit: UnitWithTenants) {
    setEditingUnitId(unit.id);
    setUnitForm({
      name: unit.name,
      floor: unit.floor,
      shares: unit.shares,
    });
  }

  function getCurrentTenant(tenants: Tenant[]): Tenant | undefined {
    return tenants.find((t) => !t.moveOutDate);
  }

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-2xl font-bold text-zinc-900">Objekte</h1>
          <button
            onClick={() => {
              setShowNewPropertyForm(!showNewPropertyForm);
              setEditingPropertyId(null);
              setPropertyForm({ street: "", zip: "", city: "", totalShares: 100 });
            }}
            className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
          >
            Neues Objekt
          </button>
        </div>

        {/* New Property Form */}
        {showNewPropertyForm && (
          <form
            onSubmit={handleCreateProperty}
            className="mb-6 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
          >
            <h3 className="mb-4 text-lg font-semibold text-zinc-900">
              Neues Objekt anlegen
            </h3>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Straße
                </label>
                <input
                  type="text"
                  required
                  value={propertyForm.street}
                  onChange={(e) =>
                    setPropertyForm({ ...propertyForm, street: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  PLZ
                </label>
                <input
                  type="text"
                  required
                  value={propertyForm.zip}
                  onChange={(e) =>
                    setPropertyForm({ ...propertyForm, zip: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Stadt
                </label>
                <input
                  type="text"
                  required
                  value={propertyForm.city}
                  onChange={(e) =>
                    setPropertyForm({ ...propertyForm, city: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Gesamtanteile (MEA)
                </label>
                <input
                  type="number"
                  required
                  value={propertyForm.totalShares}
                  onChange={(e) =>
                    setPropertyForm({
                      ...propertyForm,
                      totalShares: parseInt(e.target.value) || 0,
                    })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
            </div>
            <div className="mt-4 flex gap-2">
              <button
                type="submit"
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
              >
                Speichern
              </button>
              <button
                type="button"
                onClick={() => setShowNewPropertyForm(false)}
                className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
              >
                Abbrechen
              </button>
            </div>
          </form>
        )}

        {loading ? (
          <Loading />
        ) : properties.length === 0 ? (
          <EmptyState message="Noch keine Objekte vorhanden. Erstellen Sie Ihr erstes Objekt." />
        ) : (
          <div className="space-y-4">
            {properties.map((property) => (
              <div
                key={property.id}
                className="rounded-xl border border-zinc-200 bg-white shadow-sm"
              >
                {/* Property Header */}
                {editingPropertyId === property.id ? (
                  <div className="p-6">
                    <h3 className="mb-4 text-lg font-semibold text-zinc-900">
                      Objekt bearbeiten
                    </h3>
                    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                      <div>
                        <label className="mb-1 block text-sm font-medium text-zinc-700">
                          Straße
                        </label>
                        <input
                          type="text"
                          required
                          value={propertyForm.street}
                          onChange={(e) =>
                            setPropertyForm({
                              ...propertyForm,
                              street: e.target.value,
                            })
                          }
                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                        />
                      </div>
                      <div>
                        <label className="mb-1 block text-sm font-medium text-zinc-700">
                          PLZ
                        </label>
                        <input
                          type="text"
                          required
                          value={propertyForm.zip}
                          onChange={(e) =>
                            setPropertyForm({
                              ...propertyForm,
                              zip: e.target.value,
                            })
                          }
                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                        />
                      </div>
                      <div>
                        <label className="mb-1 block text-sm font-medium text-zinc-700">
                          Stadt
                        </label>
                        <input
                          type="text"
                          required
                          value={propertyForm.city}
                          onChange={(e) =>
                            setPropertyForm({
                              ...propertyForm,
                              city: e.target.value,
                            })
                          }
                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                        />
                      </div>
                      <div>
                        <label className="mb-1 block text-sm font-medium text-zinc-700">
                          Gesamtanteile (MEA)
                        </label>
                        <input
                          type="number"
                          required
                          value={propertyForm.totalShares}
                          onChange={(e) =>
                            setPropertyForm({
                              ...propertyForm,
                              totalShares: parseInt(e.target.value) || 0,
                            })
                          }
                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                        />
                      </div>
                    </div>
                    <div className="mt-4 flex gap-2">
                      <button
                        onClick={() => handleUpdateProperty(property.id)}
                        className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
                      >
                        Speichern
                      </button>
                      <button
                        onClick={() => {
                          setEditingPropertyId(null);
                          setPropertyForm({
                            street: "",
                            zip: "",
                            city: "",
                            totalShares: 100,
                          });
                        }}
                        className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
                      >
                        Abbrechen
                      </button>
                    </div>
                  </div>
                ) : (
                  <div
                    className="flex cursor-pointer items-center justify-between p-6"
                    onClick={() => toggleExpand(property.id)}
                  >
                    <div>
                      <h3 className="text-lg font-semibold text-zinc-900">
                        {property.street}
                      </h3>
                      <p className="text-sm text-zinc-500">
                        {property.zip} {property.city}
                      </p>
                    </div>
                    <div className="flex items-center gap-4">
                      <div className="text-right">
                        <p className="text-sm text-zinc-500">
                          {property._count?.units || 0} Wohnungen
                        </p>
                        <p className="text-xs text-zinc-400">
                          {property.totalShares} Anteile
                        </p>
                      </div>
                      <div className="flex gap-1">
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            startEditProperty(property);
                          }}
                          className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                        >
                          Bearbeiten
                        </button>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setDeletePropertyId(property.id);
                          }}
                          className="rounded-lg border border-red-200 px-3 py-1.5 text-xs font-medium text-red-600 hover:bg-red-50"
                        >
                          Löschen
                        </button>
                      </div>
                      <svg
                        className={`h-5 w-5 text-zinc-400 transition-transform ${
                          expandedId === property.id ? "rotate-180" : ""
                        }`}
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M19 9l-7 7-7-7"
                        />
                      </svg>
                    </div>
                  </div>
                )}

                {/* Expanded Units */}
                {expandedId === property.id && propertyDetail && (
                  <div className="border-t border-zinc-200 px-6 pb-6 pt-4">
                    <div className="mb-4 flex items-center justify-between">
                      <h4 className="text-sm font-semibold text-zinc-700">
                        Wohnungen
                      </h4>
                      <button
                        onClick={() => {
                          setShowNewUnitForm(
                            showNewUnitForm === property.id ? null : property.id
                          );
                          setEditingUnitId(null);
                          setUnitForm({ name: "", floor: "", shares: 0 });
                        }}
                        className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                      >
                        Wohnung hinzufügen
                      </button>
                    </div>

                    {/* New Unit Form */}
                    {showNewUnitForm === property.id && (
                      <form
                        onSubmit={(e) => handleCreateUnit(property.id, e)}
                        className="mb-4 rounded-lg border border-zinc-200 bg-zinc-50 p-4"
                      >
                        <div className="grid gap-3 sm:grid-cols-3">
                          <div>
                            <label className="mb-1 block text-xs font-medium text-zinc-700">
                              Bezeichnung
                            </label>
                            <input
                              type="text"
                              required
                              value={unitForm.name}
                              onChange={(e) =>
                                setUnitForm({
                                  ...unitForm,
                                  name: e.target.value,
                                })
                              }
                              placeholder="z.B. Wohnung EG links"
                              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                            />
                          </div>
                          <div>
                            <label className="mb-1 block text-xs font-medium text-zinc-700">
                              Stockwerk
                            </label>
                            <input
                              type="text"
                              required
                              value={unitForm.floor}
                              onChange={(e) =>
                                setUnitForm({
                                  ...unitForm,
                                  floor: e.target.value,
                                })
                              }
                              placeholder="z.B. EG"
                              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                            />
                          </div>
                          <div>
                            <label className="mb-1 block text-xs font-medium text-zinc-700">
                              Anteile (MEA)
                            </label>
                            <input
                              type="number"
                              required
                              value={unitForm.shares}
                              onChange={(e) =>
                                setUnitForm({
                                  ...unitForm,
                                  shares: parseInt(e.target.value) || 0,
                                })
                              }
                              className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                            />
                          </div>
                        </div>
                        <div className="mt-3 flex gap-2">
                          <button
                            type="submit"
                            className="rounded-lg bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-800"
                          >
                            Speichern
                          </button>
                          <button
                            type="button"
                            onClick={() => setShowNewUnitForm(null)}
                            className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                          >
                            Abbrechen
                          </button>
                        </div>
                      </form>
                    )}

                    {propertyDetail.units.length === 0 ? (
                      <p className="text-sm text-zinc-500">
                        Noch keine Wohnungen angelegt.
                      </p>
                    ) : (
                      <div className="overflow-x-auto">
                        <table className="w-full">
                          <thead>
                            <tr className="border-b border-zinc-200">
                              <th className="pb-2 text-left text-xs font-medium uppercase text-zinc-500">
                                Bezeichnung
                              </th>
                              <th className="pb-2 text-left text-xs font-medium uppercase text-zinc-500">
                                Stockwerk
                              </th>
                              <th className="pb-2 text-left text-xs font-medium uppercase text-zinc-500">
                                Anteile
                              </th>
                              <th className="pb-2 text-left text-xs font-medium uppercase text-zinc-500">
                                Aktueller Mieter
                              </th>
                              <th className="pb-2 text-left text-xs font-medium uppercase text-zinc-500">
                                Miete
                              </th>
                              <th className="pb-2 text-right text-xs font-medium uppercase text-zinc-500">
                                Aktionen
                              </th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-zinc-200">
                            {propertyDetail.units.map((unit) => {
                              const currentTenant = getCurrentTenant(
                                unit.tenants
                              );
                              return editingUnitId === unit.id ? (
                                <tr key={unit.id}>
                                  <td className="py-3" colSpan={6}>
                                    <div className="grid gap-3 sm:grid-cols-3">
                                      <div>
                                        <input
                                          type="text"
                                          required
                                          value={unitForm.name}
                                          onChange={(e) =>
                                            setUnitForm({
                                              ...unitForm,
                                              name: e.target.value,
                                            })
                                          }
                                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                                        />
                                      </div>
                                      <div>
                                        <input
                                          type="text"
                                          required
                                          value={unitForm.floor}
                                          onChange={(e) =>
                                            setUnitForm({
                                              ...unitForm,
                                              floor: e.target.value,
                                            })
                                          }
                                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                                        />
                                      </div>
                                      <div className="flex gap-2">
                                        <input
                                          type="number"
                                          required
                                          value={unitForm.shares}
                                          onChange={(e) =>
                                            setUnitForm({
                                              ...unitForm,
                                              shares:
                                                parseInt(e.target.value) || 0,
                                            })
                                          }
                                          className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                                        />
                                        <button
                                          onClick={() =>
                                            handleUpdateUnit(
                                              unit.id,
                                              property.id
                                            )
                                          }
                                          className="rounded-lg bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-800"
                                        >
                                          Speichern
                                        </button>
                                        <button
                                          onClick={() => {
                                            setEditingUnitId(null);
                                            setUnitForm({
                                              name: "",
                                              floor: "",
                                              shares: 0,
                                            });
                                          }}
                                          className="rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                                        >
                                          Abbrechen
                                        </button>
                                      </div>
                                    </div>
                                  </td>
                                </tr>
                              ) : (
                                <tr key={unit.id}>
                                  <td className="py-3 text-sm text-zinc-900">
                                    {unit.name}
                                  </td>
                                  <td className="py-3 text-sm text-zinc-600">
                                    {unit.floor}
                                  </td>
                                  <td className="py-3 text-sm text-zinc-600">
                                    {unit.shares}
                                  </td>
                                  <td className="py-3 text-sm text-zinc-600">
                                    {currentTenant
                                      ? `${currentTenant.firstName} ${currentTenant.lastName}`
                                      : "Kein Mieter"}
                                  </td>
                                  <td className="py-3 text-sm text-zinc-600">
                                    {(() => {
                                      const { rent, prepayment } = getLatestRent(unit.id);
                                      if (!rent && !prepayment) return <span className="text-zinc-400">—</span>;
                                      return (
                                        <div className="space-y-0.5">
                                          {rent != null && (
                                            <div>{formatCurrency(rent)}</div>
                                          )}
                                          {prepayment != null && (
                                            <div className="text-xs text-zinc-400">
                                              + {formatCurrency(prepayment)} NK
                                            </div>
                                          )}
                                        </div>
                                      );
                                    })()}
                                  </td>
                                  <td className="py-3 text-right">
                                    <div className="flex justify-end gap-1">
                                      <button
                                        onClick={() => startEditUnit(unit)}
                                        className="rounded-lg border border-zinc-300 px-2.5 py-1 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                                      >
                                        Bearbeiten
                                      </button>
                                      <button
                                        onClick={() =>
                                          setDeleteUnitTarget({ unitId: unit.id, propertyId: property.id })
                                        }
                                        className="rounded-lg border border-red-200 px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-50"
                                      >
                                        Löschen
                                      </button>
                                    </div>
                                  </td>
                                </tr>
                              );
                            })}
                          </tbody>
                        </table>
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
        <ConfirmDialog
          open={deletePropertyId !== null}
          onOpenChange={(open) => !open && setDeletePropertyId(null)}
          onConfirm={() => {
            if (deletePropertyId) handleDeleteProperty(deletePropertyId);
          }}
          title="Objekt löschen"
          description="Alle zugehörigen Daten werden ebenfalls gelöscht. Möchten Sie fortfahren?"
        />
        <ConfirmDialog
          open={deleteUnitTarget !== null}
          onOpenChange={(open) => !open && setDeleteUnitTarget(null)}
          onConfirm={() => {
            if (deleteUnitTarget) handleDeleteUnit(deleteUnitTarget.unitId, deleteUnitTarget.propertyId);
          }}
          title="Wohnung löschen"
          description="Möchten Sie diese Wohnung wirklich löschen?"
        />
      </main>
    </>
  );
}
