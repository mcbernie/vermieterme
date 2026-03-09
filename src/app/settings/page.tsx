"use client";

import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { Combobox } from "@/components/ui/combobox";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Loading } from "@/components/ui/loading";
import { DISTRIBUTION_KEY_OPTIONS } from "@/lib/constants";
import type { LandlordInfo, CostCategory, AppUser } from "@/types";

export default function SettingsPage() {
  const [landlord, setLandlord] = useState<LandlordInfo>({
    name: "",
    street: "",
    zip: "",
    city: "",
    phone: "",
    email: "",
    bankName: "",
    iban: "",
    accountHolder: "",
  });
  const [costCategories, setCostCategories] = useState<CostCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [savingLandlord, setSavingLandlord] = useState(false);
  const [landlordStatus, setLandlordStatus] = useState<string | null>(null);

  const [showNewCategoryForm, setShowNewCategoryForm] = useState(false);
  const [editingCategoryId, setEditingCategoryId] = useState<string | null>(
    null
  );
  const [categoryForm, setCategoryForm] = useState({
    name: "",
    distributionKey: "MEA",
    sortOrder: 0,
  });
  const [deleteCategoryId, setDeleteCategoryId] = useState<string | null>(null);

  const [users, setUsers] = useState<AppUser[]>([]);
  const [showNewUserForm, setShowNewUserForm] = useState(false);
  const [userForm, setUserForm] = useState({ name: "", email: "", password: "" });
  const [userError, setUserError] = useState<string | null>(null);
  const [savingUser, setSavingUser] = useState(false);
  const [deleteUserId, setDeleteUserId] = useState<string | null>(null);

  const [importing, setImporting] = useState(false);
  const [showImportConfirm, setShowImportConfirm] = useState(false);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [backupStatus, setBackupStatus] = useState<string | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    fetchData();
  }, []);

  async function fetchData() {
    try {
      const [landlordRes, categoriesRes, usersRes] = await Promise.all([
        fetch("/api/settings"),
        fetch("/api/cost-categories"),
        fetch("/api/users"),
      ]);

      if (landlordRes.ok) {
        const data = await landlordRes.json();
        if (data && !data.error) {
          setLandlord({
            id: data.id,
            name: data.name || "",
            street: data.street || "",
            zip: data.zip || "",
            city: data.city || "",
            phone: data.phone || "",
            email: data.email || "",
            bankName: data.bankName || "",
            iban: data.iban || "",
            accountHolder: data.accountHolder || "",
          });
        }
      }

      if (categoriesRes.ok) {
        setCostCategories(await categoriesRes.json());
      }

      if (usersRes.ok) {
        setUsers(await usersRes.json());
      }
    } catch (error) {
      console.error("Failed to fetch settings:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSaveLandlord(e: React.FormEvent) {
    e.preventDefault();
    setSavingLandlord(true);
    setLandlordStatus(null);

    try {
      const res = await fetch("/api/settings", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(landlord),
      });

      if (res.ok) {
        const data = await res.json();
        setLandlord((prev) => ({ ...prev, id: data.id }));
        setLandlordStatus("Gespeichert");
        setTimeout(() => setLandlordStatus(null), 2000);
      } else {
        setLandlordStatus("Fehler beim Speichern");
      }
    } catch (error) {
      console.error("Failed to save landlord:", error);
      setLandlordStatus("Fehler beim Speichern");
    } finally {
      setSavingLandlord(false);
    }
  }

  async function fetchCategories() {
    try {
      const res = await fetch("/api/cost-categories");
      if (res.ok) {
        setCostCategories(await res.json());
      }
    } catch (error) {
      console.error("Failed to fetch categories:", error);
    }
  }

  async function handleCreateCategory(e: React.FormEvent) {
    e.preventDefault();
    try {
      const res = await fetch("/api/cost-categories", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(categoryForm),
      });
      if (res.ok) {
        setCategoryForm({ name: "", distributionKey: "MEA", sortOrder: 0 });
        setShowNewCategoryForm(false);
        await fetchCategories();
      }
    } catch (error) {
      console.error("Failed to create category:", error);
    }
  }

  async function handleUpdateCategory(id: string) {
    try {
      const res = await fetch(`/api/cost-categories/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(categoryForm),
      });
      if (res.ok) {
        setEditingCategoryId(null);
        setCategoryForm({ name: "", distributionKey: "MEA", sortOrder: 0 });
        await fetchCategories();
      }
    } catch (error) {
      console.error("Failed to update category:", error);
    }
  }

  async function handleDeleteCategory(id: string) {
    try {
      const res = await fetch(`/api/cost-categories/${id}`, {
        method: "DELETE",
      });
      if (res.ok) {
        await fetchCategories();
      }
    } catch (error) {
      console.error("Failed to delete category:", error);
    }
  }

  async function fetchUsers() {
    try {
      const res = await fetch("/api/users");
      if (res.ok) setUsers(await res.json());
    } catch (error) {
      console.error("Failed to fetch users:", error);
    }
  }

  async function handleCreateUser(e: React.FormEvent) {
    e.preventDefault();
    setSavingUser(true);
    setUserError(null);

    try {
      const res = await fetch("/api/users", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(userForm),
      });

      if (res.ok) {
        setUserForm({ name: "", email: "", password: "" });
        setShowNewUserForm(false);
        await fetchUsers();
      } else {
        const data = await res.json();
        setUserError(data.error || "Fehler beim Erstellen");
      }
    } catch (error) {
      console.error("Failed to create user:", error);
      setUserError("Fehler beim Erstellen");
    } finally {
      setSavingUser(false);
    }
  }

  async function handleDeleteUser(id: string) {
    try {
      const res = await fetch(`/api/users/${id}`, { method: "DELETE" });
      if (res.ok) {
        await fetchUsers();
      } else {
        const data = await res.json();
        alert(data.error || "Fehler beim Löschen");
      }
    } catch (error) {
      console.error("Failed to delete user:", error);
    }
  }

  async function handleExport() {
    setBackupError(null);
    try {
      const res = await fetch("/api/backup");
      if (!res.ok) throw new Error("Export fehlgeschlagen");
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download =
        res.headers
          .get("Content-Disposition")
          ?.match(/filename="(.+)"/)?.[1] ||
        `vermieterme-backup-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setBackupStatus("Export erfolgreich");
      setTimeout(() => setBackupStatus(null), 3000);
    } catch (error) {
      console.error("Export failed:", error);
      setBackupError("Export fehlgeschlagen");
    }
  }

  function handleFileSelect(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (file) {
      setImportFile(file);
      setShowImportConfirm(true);
    }
    if (fileInputRef.current) fileInputRef.current.value = "";
  }

  async function handleImport() {
    if (!importFile) return;
    setImporting(true);
    setBackupError(null);
    setBackupStatus(null);

    try {
      const text = await importFile.text();
      const data = JSON.parse(text);
      const res = await fetch("/api/backup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data),
      });

      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error || "Import fehlgeschlagen");
      }

      setBackupStatus("Import erfolgreich — Daten wurden wiederhergestellt");
      setTimeout(() => setBackupStatus(null), 5000);
      await fetchData();
    } catch (error) {
      console.error("Import failed:", error);
      setBackupError(
        error instanceof Error ? error.message : "Import fehlgeschlagen"
      );
    } finally {
      setImporting(false);
      setImportFile(null);
    }
  }

  function startEditCategory(cat: CostCategory) {
    setEditingCategoryId(cat.id);
    setShowNewCategoryForm(false);
    setCategoryForm({
      name: cat.name,
      distributionKey: cat.distributionKey,
      sortOrder: cat.sortOrder,
    });
  }

  if (loading) {
    return (
      <>
        <Nav />
        <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <Loading />
        </main>
      </>
    );
  }

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        <h1 className="mb-8 text-2xl font-bold text-zinc-900">
          Einstellungen
        </h1>

        {/* Landlord Info */}
        <section className="mb-10">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Vermieter-Informationen
          </h2>
          <form
            onSubmit={handleSaveLandlord}
            className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
          >
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Name
                </label>
                <input
                  type="text"
                  required
                  value={landlord.name}
                  onChange={(e) =>
                    setLandlord({ ...landlord, name: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Straße
                </label>
                <input
                  type="text"
                  required
                  value={landlord.street}
                  onChange={(e) =>
                    setLandlord({ ...landlord, street: e.target.value })
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
                  value={landlord.zip}
                  onChange={(e) =>
                    setLandlord({ ...landlord, zip: e.target.value })
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
                  value={landlord.city}
                  onChange={(e) =>
                    setLandlord({ ...landlord, city: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  Telefon
                </label>
                <input
                  type="text"
                  value={landlord.phone}
                  onChange={(e) =>
                    setLandlord({ ...landlord, phone: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm font-medium text-zinc-700">
                  E-Mail
                </label>
                <input
                  type="email"
                  value={landlord.email}
                  onChange={(e) =>
                    setLandlord({ ...landlord, email: e.target.value })
                  }
                  className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                />
              </div>
            </div>

            <div className="mt-6 border-t border-zinc-100 pt-4">
              <p className="mb-3 text-sm font-medium text-zinc-500">
                Bankverbindung
              </p>
              <div className="grid gap-4 sm:grid-cols-3">
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Bankname
                  </label>
                  <input
                    type="text"
                    value={landlord.bankName}
                    onChange={(e) =>
                      setLandlord({ ...landlord, bankName: e.target.value })
                    }
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    IBAN
                  </label>
                  <input
                    type="text"
                    value={landlord.iban}
                    onChange={(e) =>
                      setLandlord({ ...landlord, iban: e.target.value })
                    }
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Kontoinhaber
                  </label>
                  <input
                    type="text"
                    value={landlord.accountHolder}
                    onChange={(e) =>
                      setLandlord({
                        ...landlord,
                        accountHolder: e.target.value,
                      })
                    }
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
              </div>
            </div>

            <div className="mt-4 flex items-center gap-3">
              <button
                type="submit"
                disabled={savingLandlord}
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
              >
                {savingLandlord ? "Speichern..." : "Speichern"}
              </button>
              {landlordStatus && (
                <span
                  className={`text-sm ${
                    landlordStatus === "Gespeichert"
                      ? "text-green-600"
                      : "text-red-600"
                  }`}
                >
                  {landlordStatus}
                </span>
              )}
            </div>
          </form>
        </section>

        {/* PDF Template */}
        <section className="mb-10">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            PDF-Vorlage
          </h2>
          <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
            <p className="mb-3 text-sm text-zinc-600">
              Passen Sie das Layout und die Schriftarten der
              Betriebskostenabrechnung an.
            </p>
            <Link
              href="/settings/pdf-template"
              className="inline-flex rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
            >
              PDF-Vorlage bearbeiten
            </Link>
          </div>
        </section>

        {/* Cost Categories */}
        <section>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-zinc-900">
              Kostenarten
            </h2>
            <button
              onClick={() => {
                setShowNewCategoryForm(!showNewCategoryForm);
                setEditingCategoryId(null);
                setCategoryForm({
                  name: "",
                  distributionKey: "MEA",
                  sortOrder:
                    costCategories.length > 0
                      ? Math.max(...costCategories.map((c) => c.sortOrder)) + 1
                      : 1,
                });
              }}
              className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
            >
              Neue Kostenart
            </button>
          </div>

          {/* New Category Form */}
          {showNewCategoryForm && (
            <form
              onSubmit={handleCreateCategory}
              className="mb-4 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
            >
              <h3 className="mb-4 text-sm font-semibold text-zinc-700">
                Neue Kostenart anlegen
              </h3>
              <div className="grid gap-4 sm:grid-cols-3">
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Name
                  </label>
                  <input
                    type="text"
                    required
                    value={categoryForm.name}
                    onChange={(e) =>
                      setCategoryForm({
                        ...categoryForm,
                        name: e.target.value,
                      })
                    }
                    placeholder="z.B. Grundsteuer"
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Verteilerschlüssel
                  </label>
                  <Combobox
                    options={DISTRIBUTION_KEY_OPTIONS}
                    value={categoryForm.distributionKey}
                    onChange={(val) =>
                      setCategoryForm({
                        ...categoryForm,
                        distributionKey: val,
                      })
                    }
                    placeholder="Schlüssel wählen..."
                    searchPlaceholder="Suchen..."
                    required
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Reihenfolge
                  </label>
                  <input
                    type="number"
                    required
                    value={categoryForm.sortOrder}
                    onChange={(e) =>
                      setCategoryForm({
                        ...categoryForm,
                        sortOrder: parseInt(e.target.value) || 0,
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
                  onClick={() => setShowNewCategoryForm(false)}
                  className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
                >
                  Abbrechen
                </button>
              </div>
            </form>
          )}

          {costCategories.length === 0 ? (
            <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
              <p className="text-sm text-zinc-500">
                Noch keine Kostenarten vorhanden. Legen Sie Ihre erste
                Kostenart an.
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white shadow-sm">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-zinc-200 bg-zinc-50">
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Reihenfolge
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Name
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Verteilerschlüssel
                    </th>
                    <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Aktionen
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-200">
                  {costCategories.map((cat) =>
                    editingCategoryId === cat.id ? (
                      <tr key={cat.id}>
                        <td className="px-4 py-3" colSpan={4}>
                          <div className="grid gap-3 sm:grid-cols-3">
                            <div>
                              <input
                                type="text"
                                required
                                value={categoryForm.name}
                                onChange={(e) =>
                                  setCategoryForm({
                                    ...categoryForm,
                                    name: e.target.value,
                                  })
                                }
                                className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                              />
                            </div>
                            <div>
                              <Combobox
                                options={DISTRIBUTION_KEY_OPTIONS}
                                value={categoryForm.distributionKey}
                                onChange={(val) =>
                                  setCategoryForm({
                                    ...categoryForm,
                                    distributionKey: val,
                                  })
                                }
                                placeholder="Schlüssel wählen..."
                                searchPlaceholder="Suchen..."
                              />
                            </div>
                            <div className="flex gap-2">
                              <input
                                type="number"
                                value={categoryForm.sortOrder}
                                onChange={(e) =>
                                  setCategoryForm({
                                    ...categoryForm,
                                    sortOrder: parseInt(e.target.value) || 0,
                                  })
                                }
                                className="w-20 rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                              />
                              <button
                                onClick={() => handleUpdateCategory(cat.id)}
                                className="rounded-lg bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-800"
                              >
                                Speichern
                              </button>
                              <button
                                onClick={() => {
                                  setEditingCategoryId(null);
                                  setCategoryForm({
                                    name: "",
                                    distributionKey: "MEA",
                                    sortOrder: 0,
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
                      <tr key={cat.id}>
                        <td className="px-4 py-3 text-sm text-zinc-500">
                          {cat.sortOrder}
                        </td>
                        <td className="px-4 py-3 text-sm text-zinc-900">
                          {cat.name}
                        </td>
                        <td className="px-4 py-3">
                          <span className="inline-flex rounded-full bg-zinc-100 px-2.5 py-0.5 text-xs font-medium text-zinc-700">
                            {cat.distributionKey}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-right">
                          <div className="flex justify-end gap-1">
                            <button
                              onClick={() => startEditCategory(cat)}
                              className="rounded-lg border border-zinc-300 px-2.5 py-1 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                            >
                              Bearbeiten
                            </button>
                            <button
                              onClick={() => setDeleteCategoryId(cat.id)}
                              className="rounded-lg border border-red-200 px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-50"
                            >
                              Löschen
                            </button>
                          </div>
                        </td>
                      </tr>
                    )
                  )}
                </tbody>
              </table>
            </div>
          )}
        </section>

        {/* Users */}
        <section className="mt-10">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-zinc-900">Benutzer</h2>
            <button
              onClick={() => {
                setShowNewUserForm(!showNewUserForm);
                setUserForm({ name: "", email: "", password: "" });
                setUserError(null);
              }}
              className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
            >
              Neuer Benutzer
            </button>
          </div>

          {showNewUserForm && (
            <form
              onSubmit={handleCreateUser}
              className="mb-4 rounded-xl border border-zinc-200 bg-white p-6 shadow-sm"
            >
              <h3 className="mb-4 text-sm font-semibold text-zinc-700">
                Neuen Benutzer anlegen
              </h3>
              <div className="grid gap-4 sm:grid-cols-3">
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Name
                  </label>
                  <input
                    type="text"
                    value={userForm.name}
                    onChange={(e) =>
                      setUserForm({ ...userForm, name: e.target.value })
                    }
                    placeholder="Optional"
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    E-Mail
                  </label>
                  <input
                    type="email"
                    required
                    value={userForm.email}
                    onChange={(e) =>
                      setUserForm({ ...userForm, email: e.target.value })
                    }
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm font-medium text-zinc-700">
                    Passwort
                  </label>
                  <input
                    type="password"
                    required
                    minLength={8}
                    value={userForm.password}
                    onChange={(e) =>
                      setUserForm({ ...userForm, password: e.target.value })
                    }
                    placeholder="Min. 8 Zeichen"
                    className="w-full rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
                  />
                </div>
              </div>
              {userError && (
                <p className="mt-3 text-sm text-red-600">{userError}</p>
              )}
              <div className="mt-4 flex gap-2">
                <button
                  type="submit"
                  disabled={savingUser}
                  className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
                >
                  {savingUser ? "Erstellen..." : "Erstellen"}
                </button>
                <button
                  type="button"
                  onClick={() => setShowNewUserForm(false)}
                  className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
                >
                  Abbrechen
                </button>
              </div>
              <p className="mt-3 text-xs text-zinc-400">
                Der Benutzer kann sein Passwort nach dem ersten Login im
                Profil-Bereich ändern.
              </p>
            </form>
          )}

          {users.length === 0 ? (
            <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
              <p className="text-sm text-zinc-500">Keine Benutzer vorhanden.</p>
            </div>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white shadow-sm">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-zinc-200 bg-zinc-50">
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      Name
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium uppercase text-zinc-500">
                      E-Mail
                    </th>
                    <th className="px-4 py-3 text-right text-xs font-medium uppercase text-zinc-500">
                      Aktionen
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-zinc-200">
                  {users.map((user) => (
                    <tr key={user.id}>
                      <td className="px-4 py-3 text-sm text-zinc-900">
                        {user.name || "—"}
                      </td>
                      <td className="px-4 py-3 text-sm text-zinc-600">
                        {user.email}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <button
                          onClick={() => setDeleteUserId(user.id)}
                          className="rounded-lg border border-red-200 px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-50"
                        >
                          Löschen
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>

        {/* Backup */}
        <section className="mt-10">
          <h2 className="mb-4 text-lg font-semibold text-zinc-900">
            Datensicherung
          </h2>
          <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
            <p className="mb-4 text-sm text-zinc-600">
              Exportieren Sie alle Daten als JSON-Datei oder stellen Sie einen
              vorherigen Datenstand wieder her.
            </p>
            <div className="flex flex-wrap gap-3">
              <button
                onClick={handleExport}
                className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
              >
                Daten exportieren
              </button>
              <button
                onClick={() => fileInputRef.current?.click()}
                disabled={importing}
                className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50 disabled:opacity-50"
              >
                {importing ? "Importiere..." : "Daten importieren"}
              </button>
              <input
                ref={fileInputRef}
                type="file"
                accept=".json"
                onChange={handleFileSelect}
                className="hidden"
              />
            </div>
            {backupStatus && (
              <p className="mt-3 text-sm text-green-600">{backupStatus}</p>
            )}
            {backupError && (
              <p className="mt-3 text-sm text-red-600">{backupError}</p>
            )}
          </div>
        </section>

        <ConfirmDialog
          open={showImportConfirm}
          onOpenChange={(open) => {
            if (!open) {
              setShowImportConfirm(false);
              setImportFile(null);
            }
          }}
          onConfirm={handleImport}
          title="Daten importieren"
          description="Alle vorhandenen Daten werden unwiderruflich durch die Daten aus der Backup-Datei ersetzt. Dieser Vorgang kann nicht rückgängig gemacht werden."
          confirmLabel="Importieren"
          variant="danger"
        />

        <ConfirmDialog
          open={deleteCategoryId !== null}
          onOpenChange={(open) => !open && setDeleteCategoryId(null)}
          onConfirm={() => {
            if (deleteCategoryId) handleDeleteCategory(deleteCategoryId);
          }}
          title="Kostenart löschen"
          description="Möchten Sie diese Kostenart wirklich löschen?"
        />

        <ConfirmDialog
          open={deleteUserId !== null}
          onOpenChange={(open) => !open && setDeleteUserId(null)}
          onConfirm={() => {
            if (deleteUserId) handleDeleteUser(deleteUserId);
          }}
          title="Benutzer löschen"
          description="Möchten Sie diesen Benutzer wirklich löschen? Der Benutzer kann sich danach nicht mehr anmelden."
        />
      </main>
    </>
  );
}
