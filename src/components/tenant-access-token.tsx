"use client";

import { useEffect, useRef, useState } from "react";
import { Button, CopyField, InlineNotice, LoadingState, useConfirm } from "@zweiund40gmbh/ui";
import { apiResult } from "@/lib/api-client";

interface TenantAccessTokenProps {
  tenantId: string;
  tenantName: string;
  onClose: () => void;
}

/** Answer of `GET`/`POST /api/tenants/:id/access-token`. */
interface TokenResponse {
  token: string | null;
}

/** Accepts an answer with a code or `null`. */
function isTokenResponse(data: unknown): data is TokenResponse {
  if (typeof data !== "object" || data === null || !("token" in data)) return false;
  return data.token === null || typeof data.token === "string";
}

/** Error text together with the tenant it belongs to (so a switch never shows a foreign error). */
interface TenantError {
  tenantId: string;
  text: string;
}

/**
 * Invitation code of a tenant for the tenant app: shows the current code with copy button, creates one
 * when there is none, and revokes it after confirmation. Errors of every request are shown inline.
 */
export function TenantAccessToken({ tenantId, tenantName, onClose }: TenantAccessTokenProps) {
  const confirm = useConfirm();
  const [token, setToken] = useState<string | null>(null);
  // Tenant whose code has been loaded; while it differs from `tenantId`, the code is loading.
  const [loadedFor, setLoadedFor] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<TenantError | null>(null);
  const loading = loadedFor !== tenantId;
  const error = failure?.tenantId === tenantId ? failure.text : null;
  // Tenant shown right now; answers of `generate`/`revoke` for another tenant are dropped.
  const shownTenant = useRef(tenantId);

  /** Stores the error of a request for this tenant, or clears it. */
  function report(id: string, text: string | null) {
    setFailure(text ? { tenantId: id, text } : null);
  }

  useEffect(() => {
    shownTenant.current = tenantId;
    let active = true;
    void apiResult(`/api/tenants/${tenantId}/access-token`, { accept: isTokenResponse }).then((res) => {
      if (!active) return;
      setToken(res.ok ? res.data.token : null);
      setFailure(res.ok ? null : { tenantId, text: res.error });
      setLoadedFor(tenantId);
    });
    return () => {
      active = false;
    };
  }, [tenantId]);

  /** Creates a new invitation code. */
  async function generate() {
    const id = tenantId;
    setBusy(true);
    const res = await apiResult(`/api/tenants/${id}/access-token`, { method: "POST", accept: isTokenResponse });
    setBusy(false);
    if (shownTenant.current !== id) return;
    if (res.ok) setToken(res.data.token);
    report(id, res.ok ? null : res.error);
  }

  /** Asks for confirmation and revokes the code. */
  async function revoke() {
    const ok = await confirm({
      title: "Zugang widerrufen",
      message: `Möchten Sie den App-Zugang für ${tenantName} wirklich widerrufen? Der Mieter kann sich danach nicht mehr in der App anmelden.`,
      confirmLabel: "Widerrufen",
      danger: true,
    });
    if (!ok) return;
    const id = tenantId;
    setBusy(true);
    const res = await apiResult(`/api/tenants/${id}/access-token`, { method: "DELETE" });
    setBusy(false);
    if (shownTenant.current !== id) return;
    if (res.ok) setToken(null);
    report(id, res.ok ? null : res.error);
  }

  return (
    <div className="panel grid gap-3">
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm font-semibold text-ink">App-Zugang für {tenantName}</p>
        <Button size="sm" variant="ghost" onClick={onClose}>
          Schließen
        </Button>
      </div>

      {error && <InlineNotice tone="bad">{error}</InlineNotice>}

      {loading ? (
        <LoadingState label="Laden …" />
      ) : token ? (
        <>
          <CopyField label="Einladungscode" value={token} size="md" />
          <p className="text-xs text-ink3">
            Der Mieter gibt diesen Code in der VermieterMe-App ein, um Zugang zu seinen Daten zu erhalten.
          </p>
          <div>
            <Button size="sm" variant="danger" onClick={() => void revoke()} disabled={busy}>
              Zugang widerrufen
            </Button>
          </div>
        </>
      ) : (
        <>
          <p className="text-xs text-ink3">
            Noch kein App-Zugang erstellt. Generieren Sie einen Einladungscode, damit der Mieter die App nutzen kann.
          </p>
          <div>
            <Button variant="primary" onClick={() => void generate()} disabled={busy}>
              {busy ? "Wird generiert …" : "Einladungscode generieren"}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
