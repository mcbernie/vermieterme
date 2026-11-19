"use client";

import { useState } from "react";
import { Button, ErrorState, InlineNotice, LoadingState } from "@zweiund40gmbh/ui";
import { PdfHiddenSections } from "./pdf-hidden-sections";
import { PdfPageEditor } from "./pdf-page-editor";
import { PdfPreview, type Selection } from "./pdf-preview";
import { PdfSectionEditor } from "./pdf-section-editor";
import { useSettingsDirty } from "./settings-guard";
import { SettingsIntro } from "./settings-layout";
import { usePdfTemplate } from "./use-pdf-template";

/** Editor of the billing PDF template: A4 preview on the left, properties of the selection on the right. */
export function PdfTemplateView() {
  const t = usePdfTemplate();
  const [selected, setSelected] = useState<Selection>(null);
  const config = t.config;
  useSettingsDirty(t.dirty);

  const actions = config && (
    <>
      <Button
        variant="secondary"
        disabled={t.saving}
        onClick={() => {
          t.reset();
          setSelected(null);
        }}
      >
        Zurücksetzen
      </Button>
      <Button variant="primary" disabled={t.saving} onClick={t.saveConfig}>
        {t.saving ? "Speichern …" : "Speichern"}
      </Button>
    </>
  );

  return (
    <>
      <SettingsIntro
        description="Klicken Sie auf einen Bereich in der Vorschau, um ihn zu bearbeiten."
        actions={actions}
      />
      {t.loading ? (
        <LoadingState />
      ) : t.loadError || !config ? (
        <ErrorState error={t.loadError} onRetry={t.retry} />
      ) : (
        <div className="grid gap-4">
          {t.saveError && <InlineNotice tone="bad">{t.saveError}</InlineNotice>}
          {t.saved && <InlineNotice tone="ok">Gespeichert</InlineNotice>}
          <div className="flex flex-col gap-6 lg:flex-row">
            <div className="flex-1 overflow-auto">
              <PdfPreview config={config} selected={selected} onSelect={setSelected} />
            </div>
            <aside className="w-full shrink-0 lg:w-80" aria-label="Eigenschaften">
              <div className="grid gap-3 lg:sticky lg:top-4">
                {selected === null && (
                  <div className="panel grid gap-3 text-sm text-ink2">
                    <p>Klicken Sie auf einen Bereich in der Vorschau, um seine Eigenschaften zu bearbeiten.</p>
                    <p>Klicken Sie auf den Seitenhintergrund für die Seiten-Einstellungen.</p>
                  </div>
                )}
                {selected !== "page" && (
                  <Button variant="ghost" size="sm" onClick={() => setSelected("page")}>
                    Seiten-Einstellungen öffnen
                  </Button>
                )}
                <PdfHiddenSections config={config} onSelect={setSelected} />
                {selected === "page" && <PdfPageEditor page={config.page} onChange={t.setPage} />}
                {selected && selected !== "page" && (
                  <PdfSectionEditor
                    sectionId={selected}
                    config={config.sections[selected]}
                    pageFont={config.page.fontFamily}
                    onUpdate={(patch) => t.updateSection(selected, patch)}
                    onUpdateStyle={(patch) => t.updateStyle(selected, patch)}
                    onUpdateText={(key, value) => t.updateText(selected, key, value)}
                  />
                )}
              </div>
            </aside>
          </div>
        </div>
      )}
    </>
  );
}
