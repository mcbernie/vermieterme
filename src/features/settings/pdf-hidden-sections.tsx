"use client";

import { Button } from "@zweiund40gmbh/ui";
import { SECTION_META } from "@/lib/pdf-template";
import type { PdfTemplateConfig, SectionId } from "@/types/pdf-template";

interface Props {
  config: PdfTemplateConfig;
  onSelect: (id: SectionId) => void;
}

/**
 * Sections switched off in the template. They are not drawn in the preview, so this list is the way to
 * select them again (and switch them back on). Renders nothing while every section is visible.
 */
export function PdfHiddenSections({ config, onSelect }: Props) {
  const hidden = (Object.keys(config.sections) as SectionId[]).filter((id) => !config.sections[id].visible);
  if (hidden.length === 0) return null;
  return (
    <div className="panel grid gap-2">
      <h3 className="label-caps">Ausgeblendete Bereiche</h3>
      <ul className="grid gap-1">
        {hidden.map((id) => (
          <li key={id}>
            <Button variant="ghost" size="sm" onClick={() => onSelect(id)}>
              {SECTION_META[id].label}
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
