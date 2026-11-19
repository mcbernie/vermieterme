"use client";

import { useState } from "react";
import { Button, EmptyState, InlineNotice, useConfirm } from "@zweiund40gmbh/ui";
import type { AppUser } from "@/types";
import { apiJson } from "@/lib/api-client";
import { LoadGate, SettingsSection } from "./section";
import { useAction } from "@/components/form/use-action";
import { LOAD_FAILED, useLoad } from "./use-load";
import { UserFormModal, type NewUserValues } from "./user-form-modal";

/** Loads the users that may sign in. */
const loadUsers = (signal: AbortSignal) => apiJson<AppUser[]>("/api/users", { fallback: LOAD_FAILED, signal });

/** Users with sign-in access: list, create and delete. */
export function UsersSection() {
  const { data, loading, error, refreshError, retry, refresh } = useLoad(loadUsers);
  const [creating, setCreating] = useState(false);
  const remove = useAction("Fehler beim Löschen");
  const confirm = useConfirm();
  const users = data ?? [];

  /** Creates a user, then reloads the list. */
  async function create(values: NewUserValues) {
    await apiJson("/api/users", { fallback: "Fehler beim Erstellen", method: "POST", json: values });
    await refresh();
  }

  async function askDelete(user: AppUser) {
    const ok = await confirm({
      title: "Benutzer löschen",
      message: "Möchten Sie diesen Benutzer wirklich löschen? Der Benutzer kann sich danach nicht mehr anmelden.",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (!ok) return;
    await remove.run(async () => {
      await apiJson(`/api/users/${user.id}`, { fallback: "Fehler beim Löschen", method: "DELETE" });
      await refresh();
    });
  }

  return (
    <SettingsSection
      title="Benutzer"
      actions={
        <Button variant="primary" onClick={() => setCreating(true)} disabled={loading || !!error}>
          Neuer Benutzer
        </Button>
      }
    >
      <LoadGate loading={loading} error={error} refreshError={refreshError} onRetry={retry}>
        {remove.error && <InlineNotice tone="bad">{remove.error}</InlineNotice>}
        {users.length === 0 ? (
          <EmptyState icon={null} title="Keine Benutzer vorhanden." />
        ) : (
          <div className="overflow-x-auto border border-line bg-layer">
            <table className="table-plain">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>E-Mail</th>
                  <th className="text-right">Aktionen</th>
                </tr>
              </thead>
              <tbody>
                {users.map((user) => (
                  <tr key={user.id}>
                    <td>{user.name || "—"}</td>
                    <td className="text-ink2">{user.email}</td>
                    <td className="text-right">
                      <Button size="sm" variant="danger" disabled={remove.busy} onClick={() => askDelete(user)}>
                        Löschen
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </LoadGate>

      {creating && <UserFormModal onCreate={create} onClose={() => setCreating(false)} />}
    </SettingsSection>
  );
}
