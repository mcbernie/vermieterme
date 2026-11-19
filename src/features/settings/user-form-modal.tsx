"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, Modal, PasswordInput, TextInput } from "@zweiund40gmbh/ui";
import { PASSWORD_MIN_LENGTH } from "@/lib/constants";
import { useAction } from "@/components/form/use-action";

/** Values of a new user. */
export interface NewUserValues {
  name: string;
  email: string;
  password: string;
}

/** Field errors of the new-user form; empty when it can be sent. */
export function newUserErrors(values: NewUserValues): Partial<Record<keyof NewUserValues, string>> {
  const errors: Partial<Record<keyof NewUserValues, string>> = {};
  const email = values.email.trim();
  if (!email) errors.email = "Bitte eine E-Mail-Adresse angeben.";
  else if (!/^[^@\s]+@[^@\s]+$/.test(email)) errors.email = "Das ist keine gültige E-Mail-Adresse.";
  if (values.password.length < PASSWORD_MIN_LENGTH) {
    errors.password = `Passwort muss mindestens ${PASSWORD_MIN_LENGTH} Zeichen lang sein`;
  }
  return errors;
}

interface Props {
  /** Creates the user; throws a `RequestError` with the API message on failure. */
  onCreate: (values: NewUserValues) => Promise<void>;
  onClose: () => void;
}

/** Dialog to create a user with e-mail and initial password. Mount it only while it should be open. */
export function UserFormModal({ onCreate, onClose }: Props) {
  const [form, setForm] = useState<NewUserValues>({ name: "", email: "", password: "" });
  const [attempted, setAttempted] = useState(false);
  const { busy, error, run } = useAction("Fehler beim Erstellen");
  const errors = attempted ? newUserErrors(form) : {};

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    if (Object.keys(newUserErrors(form)).length > 0) return;
    if (await run(() => onCreate(form))) onClose();
  }

  return (
    <Modal
      title="Neuen Benutzer anlegen"
      description="Der Benutzer kann sein Passwort nach dem ersten Login im Profil-Bereich ändern."
      onClose={onClose}
      preventClose={busy}
      footer={
        <>
          <Button type="button" variant="secondary" onClick={onClose} disabled={busy}>
            Abbrechen
          </Button>
          <Button type="submit" form="user-form" variant="primary" disabled={busy}>
            {busy ? "Erstellen …" : "Erstellen"}
          </Button>
        </>
      }
    >
      <form id="user-form" noValidate onSubmit={submit} className="grid gap-4">
        <Field label="Name">
          <TextInput
            value={form.name}
            placeholder="Optional"
            onChange={(e) => setForm({ ...form, name: e.target.value })}
          />
        </Field>
        <Field label="E-Mail" required error={errors.email}>
          <TextInput
            type="email"
            autoComplete="off"
            value={form.email}
            onChange={(e) => setForm({ ...form, email: e.target.value })}
          />
        </Field>
        <Field label="Passwort" required helper={`Mindestens ${PASSWORD_MIN_LENGTH} Zeichen`} error={errors.password}>
          <PasswordInput
            autoComplete="new-password"
            value={form.password}
            onChange={(e) => setForm({ ...form, password: e.target.value })}
          />
        </Field>
        {error && <InlineNotice tone="bad">{error}</InlineNotice>}
      </form>
    </Modal>
  );
}
