"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, TextInput } from "@zweiund40gmbh/ui";
import { apiJson } from "@/lib/api-client";
import { useAction } from "@/features/settings/use-action";
import type { AppUser } from "@/types";

interface Props {
  profile: AppUser;
  /** Called after the name was stored (updates session and profile). */
  onSaved: (name: string) => Promise<void>;
}

/** Personal data: editable display name, e-mail shown read-only. */
export function NameForm({ profile, onSaved }: Props) {
  const [name, setName] = useState(profile.name || "");
  const [saved, setSaved] = useState(false);
  const { busy, error, run } = useAction("Fehler beim Speichern");

  async function submit(e: FormEvent) {
    e.preventDefault();
    setSaved(false);
    const ok = await run(async () => {
      await apiJson("/api/profile", { fallback: "Fehler beim Speichern", method: "PUT", json: { name } });
      await onSaved(name);
    });
    setSaved(ok);
  }

  return (
    <form noValidate onSubmit={submit} className="panel grid gap-4" aria-label="Persönliche Daten">
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="Name">
          <TextInput
            autoComplete="name"
            value={name}
            onChange={(e) => {
              setName(e.target.value);
              setSaved(false);
            }}
          />
        </Field>
        <Field label="E-Mail">
          <TextInput type="email" value={profile.email || ""} disabled readOnly />
        </Field>
      </div>
      {error && <InlineNotice tone="bad">{error}</InlineNotice>}
      {saved && <InlineNotice tone="ok">Name gespeichert</InlineNotice>}
      <div>
        <Button type="submit" variant="primary" disabled={busy}>
          {busy ? "Speichern …" : "Speichern"}
        </Button>
      </div>
    </form>
  );
}
