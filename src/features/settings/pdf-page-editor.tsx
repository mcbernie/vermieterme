"use client";

import type { PageConfig } from "@/types/pdf-template";
import { FontSelect, NumberField } from "./pdf-fields";

const MARGINS = [
  ["paddingTop", "Oben"],
  ["paddingBottom", "Unten"],
  ["paddingLeft", "Links"],
  ["paddingRight", "Rechts"],
] as const;

interface Props {
  page: PageConfig;
  onChange: (page: PageConfig) => void;
}

/** Page-wide settings of the template: default font, font size and page margins. */
export function PdfPageEditor({ page, onChange }: Props) {
  return (
    <div className="panel grid gap-4">
      <div>
        <h3 className="section-title">Seiten-Einstellungen</h3>
        <p className="mt-1 text-xs text-ink2">Globale Einstellungen für die gesamte Seite</p>
      </div>
      <FontSelect
        label="Standard-Schriftart"
        value={page.fontFamily}
        onChange={(fontFamily) => onChange({ ...page, fontFamily })}
      />
      <NumberField
        label="Standard-Schriftgröße (pt)"
        value={page.fontSize}
        min={6}
        max={24}
        fallback={10}
        onChange={(fontSize) => onChange({ ...page, fontSize })}
      />
      <div className="grid gap-2 border-t border-line pt-3">
        <p className="label-caps">Seitenränder (pt)</p>
        <div className="grid grid-cols-2 gap-2">
          {MARGINS.map(([key, label]) => (
            <NumberField
              key={key}
              label={label}
              value={page[key]}
              min={10}
              max={100}
              fallback={40}
              onChange={(value) => onChange({ ...page, [key]: value })}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
