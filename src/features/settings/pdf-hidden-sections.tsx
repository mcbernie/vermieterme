"use client";

import { Button } from "@zweiund40gmbh/ui";
import { SECTION_META } from "@/lib/pdf-template";
import type { PdfTemplateConfig, SectionId } from "@/types/pdf-template";

interface Props {
  config: PdfTemplateConfig;
  /** Current selection of the editor; its button is marked with `aria-pressed`. */
  selected?: string | null;
  onSelect: (id: SectionId) => void;
}

/** Whether `id` is a section the editor knows (stored templates may hold unknown or outdated ids). */
const isKnownSection = (id: string): id is SectionId => Object.hasOwn(SECTION_META, id);

/**
 * Sections switched off in the template. They are not drawn in the preview, so this list is the way to
 * select them again (and switch them back on). Renders nothing while every section is visible. Ids that are
 * not in `SECTION_META` are skipped.
 */
export function PdfHiddenSections({ config, selected = null, onSelect }: Props) {
  const hidden = Object.keys(config.sections)
    .filter(isKnownSection)
    .filter((id) => config.sections[id] && !config.sections[id].visible);
  if (hidden.length === 0) return null;
  return (
    <div className="panel grid gap-2">
      <h3 className="label-caps">Ausgeblendete Bereiche</h3>
      <ul className="grid gap-1">
        {hidden.map((id) => (
          <li key={id}>
            <Button variant="ghost" size="sm" aria-pressed={selected === id} onClick={() => onSelect(id)}>
              {SECTION_META[id].label}
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
