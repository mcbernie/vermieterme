"use client";

import { Field, SegmentedControl, SwitchRow, Textarea, TextInput } from "@zweiund40gmbh/ui";
import { SECTION_META } from "@/lib/pdf-template";
import type { PdfFontFamily, SectionConfig, SectionId, SectionStyle } from "@/types/pdf-template";
import { FontSelect, NumberField } from "./pdf-fields";

const ALIGN_OPTIONS = [
  { value: "left", label: "Links" },
  { value: "center", label: "Mitte" },
  { value: "right", label: "Rechts" },
] as const;

interface Props {
  sectionId: SectionId;
  config: SectionConfig;
  pageFont: PdfFontFamily;
  onUpdate: (patch: Partial<SectionConfig>) => void;
  onUpdateStyle: (patch: Partial<SectionStyle>) => void;
  onUpdateText: (key: string, value: string) => void;
}

/** Properties of one template section: visibility, font, emphasis, alignment, colour, spacing and texts. */
export function PdfSectionEditor({ sectionId, config, pageFont, onUpdate, onUpdateStyle, onUpdateText }: Props) {
  const meta = SECTION_META[sectionId];
  const { style } = config;

  return (
    <div className="panel grid gap-4">
      <div>
        <h3 className="section-title">{meta.label}</h3>
        <p className="mt-1 text-xs text-ink2">{meta.description}</p>
      </div>

      <SwitchRow label="Sichtbar" checked={config.visible} onChange={(visible) => onUpdate({ visible })} />

      <FontSelect
        label="Schriftart"
        value={style.fontFamily}
        pageFont={pageFont}
        onChange={(fontFamily) => onUpdateStyle({ fontFamily })}
      />
      <NumberField
        label="Schriftgröße (pt)"
        value={style.fontSize}
        min={5}
        max={30}
        fallback={10}
        onChange={(fontSize) => onUpdateStyle({ fontSize })}
      />

      <SwitchRow label="Fett" checked={style.bold} onChange={(bold) => onUpdateStyle({ bold })} />
      <SwitchRow label="Kursiv" checked={style.italic} onChange={(italic) => onUpdateStyle({ italic })} />

      <Field label="Ausrichtung">
        <SegmentedControl
          size="sm"
          value={style.textAlign}
          options={ALIGN_OPTIONS}
          onChange={(textAlign) => onUpdateStyle({ textAlign })}
        />
      </Field>

      <Field label="Farbe">
        {(control) => (
          <div className="flex items-center gap-2">
            <input
              type="color"
              aria-label="Farbe wählen"
              value={style.color}
              onChange={(e) => onUpdateStyle({ color: e.target.value })}
              className="h-8 w-8 cursor-pointer border border-line bg-field"
            />
            <TextInput {...control} size="sm" value={style.color} onChange={(e) => onUpdateStyle({ color: e.target.value })} />
          </div>
        )}
      </Field>

      <div className="grid gap-2 border-t border-line pt-3">
        <p className="label-caps">Abstände (pt)</p>
        <div className="grid grid-cols-2 gap-2">
          <NumberField
            label="Oben"
            value={style.marginTop}
            min={0}
            max={80}
            fallback={0}
            onChange={(marginTop) => onUpdateStyle({ marginTop })}
          />
          <NumberField
            label="Unten"
            value={style.marginBottom}
            min={0}
            max={80}
            fallback={0}
            onChange={(marginBottom) => onUpdateStyle({ marginBottom })}
          />
        </div>
      </div>

      {meta.textFields && meta.textFields.length > 0 && (
        <div className="grid gap-2 border-t border-line pt-3">
          <p className="label-caps">Text-Inhalte</p>
          {meta.textFields.map((field) => (
            <Field key={field.key} label={field.label}>
              {field.multiline ? (
                <Textarea
                  rows={3}
                  value={config.texts[field.key] || ""}
                  onChange={(e) => onUpdateText(field.key, e.target.value)}
                />
              ) : (
                <TextInput
                  size="sm"
                  value={config.texts[field.key] || ""}
                  onChange={(e) => onUpdateText(field.key, e.target.value)}
                />
              )}
            </Field>
          ))}
        </div>
      )}
    </div>
  );
}
