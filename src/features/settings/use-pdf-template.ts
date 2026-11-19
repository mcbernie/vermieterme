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
 * „Saved“ and „dirty“ compare object identity with the last stored configuration: every edit creates a
 * new object, so an edit made while a save runs is neither reported as saved nor lost from `dirty`.
 */
export function usePdfTemplate() {
  const loaded = useLoad(loadConfig);
  const [edited, setEdited] = useState<PdfTemplateConfig | null>(null);
  const [lastSaved, setLastSaved] = useState<PdfTemplateConfig | null>(null);
  const save = useAction("Fehler beim Speichern");
  const stored = loaded.data;
  const config = edited ?? stored;
  const baseline = lastSaved ?? stored;

  /** Applies a change to the edited configuration. */
  const edit = useCallback(
    (change: (config: PdfTemplateConfig) => PdfTemplateConfig) => {
      setEdited((current) => {
        const base = current ?? stored;
        return base ? change(base) : current;
      });
    },
    [stored],
  );

  /** Saves the configuration as it is now; ignored while a save runs. */
  async function saveConfig() {
    if (!config) return;
    const sent = config;
    const ok = await save.run(async () => {
      await requestJson("/api/pdf-template", "Fehler beim Speichern", { method: "PUT", json: { config: sent } });
    });
    if (ok) setLastSaved(sent);
  }

  return {
    config,
    loading: loaded.loading,
    loadError: loaded.error,
    retry: loaded.retry,
    saving: save.busy,
    saveError: save.error,
    /** The shown configuration is exactly the one saved last. */
    saved: lastSaved !== null && config === lastSaved,
    /** The shown configuration differs from the stored one. */
    dirty: config !== null && config !== baseline,
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
