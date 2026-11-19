"use client";

import { useRef, useState, type ChangeEvent } from "react";
import { Button, InlineNotice, useConfirm } from "@zweiund40gmbh/ui";
import { downloadBackup, IMPORT_FAILED, importBackup } from "./backup";
import { SettingsSection } from "./section";
import { useAction } from "@/components/form/use-action";

interface Props {
  /** Called after a successful import so the page can reload its data. */
  onImported: () => void;
}

/** Export of all data as JSON and restore from such a file (after a confirmation). */
export function BackupSection({ onImported }: Props) {
  const fileInput = useRef<HTMLInputElement>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const action = useAction(IMPORT_FAILED);
  const confirm = useConfirm();

  async function exportData() {
    setStatus(null);
    if (await action.run(downloadBackup)) setStatus("Export erfolgreich");
  }

  async function onFile(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (fileInput.current) fileInput.current.value = "";
    if (!file) return;
    const ok = await confirm({
      title: "Daten importieren",
      message:
        "Alle vorhandenen Daten werden unwiderruflich durch die Daten aus der Backup-Datei ersetzt. Dieser Vorgang kann nicht rückgängig gemacht werden.",
      confirmLabel: "Importieren",
      danger: true,
    });
    if (!ok) return;
    setStatus(null);
    setImporting(true);
    const done = await action.run(() => importBackup(file));
    setImporting(false);
    if (!done) return;
    setStatus("Import erfolgreich — Daten wurden wiederhergestellt");
    onImported();
  }

  return (
    <SettingsSection
      title="Datensicherung"
      description="Exportieren Sie alle Daten als JSON-Datei oder stellen Sie einen vorherigen Datenstand wieder her."
    >
      <div className="panel grid gap-4">
        <div className="flex flex-wrap gap-3">
          <Button variant="secondary" onClick={exportData} disabled={action.busy}>
            Daten exportieren
          </Button>
          <Button variant="secondary" onClick={() => fileInput.current?.click()} disabled={action.busy}>
            {importing ? "Importiere …" : "Daten importieren"}
          </Button>
          <input
            ref={fileInput}
            type="file"
            accept=".json"
            onChange={onFile}
            className="hidden"
            aria-label="Backup-Datei auswählen"
            data-testid="backup-file"
          />
        </div>
        {status && <InlineNotice tone="ok">{status}</InlineNotice>}
        {action.error && <InlineNotice tone="bad">{action.error}</InlineNotice>}
      </div>
    </SettingsSection>
  );
}
