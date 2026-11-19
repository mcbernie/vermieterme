"use client";

import { Button, EmptyState, InlineNotice, Tag, useConfirm } from "@zweiund40gmbh/ui";
import type { VpiEntry } from "@/types";
import { apiJson } from "@/lib/api-client";
import { LoadGate, SettingsSection } from "./section";
import { SettingsIntro } from "./settings-layout";
import { useAction } from "@/components/form/use-action";
import { useLoad } from "@/components/form/use-load";
import { groupByBaseYear, loadVpiEntries, MONTH_NAMES } from "./vpi";
import { VpiEntryForm } from "./vpi-entry-form";
import { VpiFetchPanel } from "./vpi-fetch-panel";

interface TableProps {
  entries: VpiEntry[];
  busy: boolean;
  onDelete: (entry: VpiEntry) => void;
}

/** VPI values grouped by base year, each group as a small table. */
function VpiTables({ entries, busy, onDelete }: TableProps) {
  if (entries.length === 0) {
    return (
      <EmptyState
        icon={null}
        title="Noch keine VPI-Einträge vorhanden. Fügen Sie Ihren ersten Eintrag über das Formular oben hinzu."
      />
    );
  }
  return (
    <div className="grid gap-6">
      {groupByBaseYear(entries).map((group) => (
        <div key={group.baseYear} className="grid gap-2">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-ink">Basisjahr: {group.baseYear}</h3>
            <Tag>
              {group.entries.length} {group.entries.length === 1 ? "Eintrag" : "Einträge"}
            </Tag>
          </div>
          <div className="overflow-x-auto border border-line bg-layer">
            <table className="table-plain">
              <thead>
                <tr>
                  <th>Monat/Jahr</th>
                  <th className="text-right">VPI-Wert</th>
                  <th className="text-right">Aktionen</th>
                </tr>
              </thead>
              <tbody>
                {group.entries.map((entry) => (
                  <tr key={entry.id}>
                    <td>
                      {MONTH_NAMES[entry.month - 1]} {entry.year}
                    </td>
                    <td className="num text-right">{entry.value}</td>
                    <td className="text-right">
                      <Button size="sm" variant="danger" disabled={busy} onClick={() => onDelete(entry)}>
                        Löschen
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ))}
    </div>
  );
}

/** VPI management: automatic import, manual entry and the list of stored values. */
export function VpiView() {
  const { data, loading, error, refreshError, retry, refresh } = useLoad(loadVpiEntries);
  const remove = useAction("Fehler beim Löschen");
  const confirm = useConfirm();

  async function askDelete(entry: VpiEntry) {
    const ok = await confirm({
      title: "VPI-Eintrag löschen",
      message: "Möchten Sie diesen VPI-Eintrag wirklich löschen?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (!ok) return;
    await remove.run(async () => {
      await apiJson(`/api/vpi/${entry.id}`, { fallback: "Fehler beim Löschen", method: "DELETE" });
      await refresh();
    });
  }

  return (
    <>
      <SettingsIntro description="Der Verbraucherpreisindex (VPI) wird vom Statistischen Bundesamt veröffentlicht und dient als Grundlage für Indexmietanpassungen. Tragen Sie hier die VPI-Werte ein, die Sie für Ihre Mietverträge benötigen." />
      <SettingsSection title="VPI automatisch abrufen">
        <VpiFetchPanel onImported={refresh} />
      </SettingsSection>
      <SettingsSection title="VPI-Eintrag hinzufügen">
        <VpiEntryForm onSaved={refresh} />
      </SettingsSection>
      <SettingsSection title="Vorhandene VPI-Einträge">
        <LoadGate loading={loading} error={error} refreshError={refreshError} onRetry={retry}>
          {remove.error && <InlineNotice tone="bad">{remove.error}</InlineNotice>}
          <VpiTables entries={data ?? []} busy={remove.busy} onDelete={askDelete} />
        </LoadGate>
      </SettingsSection>
    </>
  );
}
