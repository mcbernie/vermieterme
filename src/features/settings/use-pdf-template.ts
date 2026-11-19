"use client";

import { useCallback, useState } from "react";
import { getDefaultConfig } from "@/lib/pdf-template";
import type { PageConfig, PdfTemplateConfig, SectionConfig, SectionId, SectionStyle } from "@/types/pdf-template";
import { requestJson } from "./request";
import { useAction } from "./use-action";
import { LOAD_FAILED, useLoad } from "./use-load";

/** Loads the stored template configuration (the API answers with the default when none is stored). */
const loadConfig = async (signal: AbortSignal) =>
  (await requestJson<{ config: PdfTemplateConfig }>("/api/pdf-template", LOAD_FAILED, { signal })).config;

/** Returns a copy of `config` with section `id` changed by `change`. */
function withSection(
  config: PdfTemplateConfig,
  id: SectionId,
  change: (section: SectionConfig) => SectionConfig,
): PdfTemplateConfig {
  return { ...config, sections: { ...config.sections, [id]: change(config.sections[id]) } };
}

/**
 * State of the template editor: loads the stored configuration, keeps the edited copy and saves it.
 * Editing starts only after loading succeeded, so a failed load can never overwrite the stored template.
 */
export function usePdfTemplate() {
  const loaded = useLoad(loadConfig);
  const [edited, setEdited] = useState<PdfTemplateConfig | null>(null);
  const [saved, setSaved] = useState(false);
  const save = useAction("Fehler beim Speichern");
  const stored = loaded.data;
  const config = edited ?? stored;

  /** Applies a change to the edited configuration and hides an old success message. */
  const edit = useCallback(
    (change: (config: PdfTemplateConfig) => PdfTemplateConfig) => {
      setEdited((current) => {
        const base = current ?? stored;
        return base ? change(base) : current;
      });
      setSaved(false);
    },
    [stored],
  );

  /** Saves the edited configuration; ignored while a save runs. */
  async function saveConfig() {
    if (!config) return;
    setSaved(false);
    const ok = await save.run(async () => {
      await requestJson("/api/pdf-template", "Fehler beim Speichern", { method: "PUT", json: { config } });
    });
    setSaved(ok);
  }

  return {
    config,
    loading: loaded.loading,
    loadError: loaded.error,
    retry: loaded.retry,
    saving: save.busy,
    saveError: save.error,
    saved,
    saveConfig,
    reset: () => edit(() => getDefaultConfig()),
    setPage: (page: PageConfig) => edit((c) => ({ ...c, page })),
    updateSection: (id: SectionId, patch: Partial<SectionConfig>) =>
      edit((c) => withSection(c, id, (s) => ({ ...s, ...patch }))),
    updateStyle: (id: SectionId, patch: Partial<SectionStyle>) =>
      edit((c) => withSection(c, id, (s) => ({ ...s, style: { ...s.style, ...patch } }))),
    updateText: (id: SectionId, key: string, value: string) =>
      edit((c) => withSection(c, id, (s) => ({ ...s, texts: { ...s.texts, [key]: value } }))),
  };
}
