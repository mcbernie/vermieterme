"use client";

import { useState, type FormEvent } from "react";
import { Button, Field, InlineNotice, TextInput } from "@zweiund40gmbh/ui";
import { importFromBundesbank } from "./vpi";
import { useAction } from "@/components/form/use-action";

interface Props {
  /** Called after values were imported. */
  onImported: () => Promise<void>;
}

/** Loads the VPI values of a base year from the Deutsche Bundesbank and stores them. */
export function VpiFetchPanel({ onImported }: Props) {
  const [baseYear, setBaseYear] = useState("2020");
  const [attempted, setAttempted] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const { busy, error, run } = useAction("Fehler beim Abrufen der VPI-Daten");
  const year = parseInt(baseYear);
  const yearError = attempted && !Number.isInteger(year) ? "Bitte ein Basisjahr angeben." : undefined;

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    setStatus(null);
    if (!Number.isInteger(year)) return;
    await run(async () => {
      const count = await importFromBundesbank(year);
      setStatus(`${count} VPI-Werte erfolgreich importiert`);
      await onImported();
    });
  }

  return (
    <form noValidate onSubmit={submit} className="panel grid gap-4" aria-label="VPI automatisch abrufen">
      <p className="text-sm text-ink2">
        VPI-Werte automatisch von der Deutschen Bundesbank laden. Unterstützte Basisjahre: 2010, 2015, 2020.
      </p>
      <div className="flex flex-wrap items-start gap-3">
        <Field label="Basisjahr" error={yearError} className="w-40">
          <TextInput type="number" value={baseYear} onChange={(e) => setBaseYear(e.target.value)} />
        </Field>
        <Button type="submit" variant="secondary" disabled={busy} className="mt-6">
          {busy ? "Lade …" : "Von Bundesbank laden"}
        </Button>
      </div>
      {error && <InlineNotice tone="bad">{error}</InlineNotice>}
      {status && <InlineNotice tone="ok">{status}</InlineNotice>}
    </form>
  );
}
