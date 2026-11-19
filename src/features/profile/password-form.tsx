"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, PasswordInput } from "@zweiund40gmbh/ui";
import { PASSWORD_MIN_LENGTH } from "@/lib/constants";
import { apiJson } from "@/lib/api-client";
import { useAction } from "@/components/form/use-action";
import { passwordErrors, type PasswordValues } from "./password";

const EMPTY: PasswordValues = { currentPassword: "", newPassword: "", confirmPassword: "" };
const FAILED = "Fehler beim Ändern des Passworts";

/**
 * Change-password form. Checked on submit, errors appear at the fields; on success all fields are cleared.
 * Passwords are only sent to the API and never logged or shown.
 */
export function PasswordForm() {
  const [form, setForm] = useState<PasswordValues>(EMPTY);
  const [attempted, setAttempted] = useState(false);
  const [changed, setChanged] = useState(false);
  const { busy, error, run } = useAction(FAILED);
  const errors = attempted ? passwordErrors(form) : {};

  /** Changes one field and hides an old success message. */
  function set(key: keyof PasswordValues, value: string) {
    setForm((prev) => ({ ...prev, [key]: value }));
    setChanged(false);
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    setChanged(false);
    if (Object.keys(passwordErrors(form)).length > 0) return;
    const ok = await run(async () => {
      await apiJson("/api/profile", {
        fallback: FAILED,
        method: "PUT",
        json: { currentPassword: form.currentPassword, newPassword: form.newPassword },
      });
    });
    if (!ok) return;
    setForm(EMPTY);
    setAttempted(false);
    setChanged(true);
  }

  return (
    <form noValidate onSubmit={submit} className="panel grid max-w-md gap-4" aria-label="Passwort ändern">
      <Field label="Aktuelles Passwort" required error={errors.currentPassword}>
        <PasswordInput
          autoComplete="current-password"
          value={form.currentPassword}
          onChange={(e) => set("currentPassword", e.target.value)}
        />
      </Field>
      <Field
        label="Neues Passwort"
        required
        helper={`Mindestens ${PASSWORD_MIN_LENGTH} Zeichen`}
        error={errors.newPassword}
      >
        <PasswordInput
          autoComplete="new-password"
          value={form.newPassword}
          onChange={(e) => set("newPassword", e.target.value)}
        />
      </Field>
      <Field label="Neues Passwort bestätigen" required error={errors.confirmPassword}>
        <PasswordInput
          autoComplete="new-password"
          value={form.confirmPassword}
          onChange={(e) => set("confirmPassword", e.target.value)}
        />
      </Field>
      {error && <InlineNotice tone="bad">{error}</InlineNotice>}
      {changed && <InlineNotice tone="ok">Passwort geändert</InlineNotice>}
      <div>
        <Button type="submit" variant="primary" disabled={busy}>
          {busy ? "Ändern …" : "Passwort ändern"}
        </Button>
      </div>
    </form>
  );
}
