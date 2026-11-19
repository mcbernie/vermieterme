"use client";

import { Field, Select, TextInput } from "@zweiund40gmbh/ui";
import { FONT_OPTIONS } from "@/lib/pdf-template";
import type { PdfFontFamily } from "@/types/pdf-template";

interface NumberProps {
  label: string;
  value: number;
  min: number;
  max: number;
  /** Value used when the text is empty or unreadable (as before the redesign). */
  fallback: number;
  onChange: (value: number) => void;
}

/** Small number field of the template editor; changes apply immediately to the preview. */
export function NumberField({ label, value, min, max, fallback, onChange }: NumberProps) {
  return (
    <Field label={label}>
      <TextInput
        type="number"
        size="sm"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(parseInt(e.target.value) || fallback)}
      />
    </Field>
  );
}

interface FontProps {
  label: string;
  value: PdfFontFamily;
  onChange: (value: PdfFontFamily) => void;
  /** Page font; when given it is offered first as „Standard (…)“. */
  pageFont?: PdfFontFamily;
}

/** Font selection of the template editor. */
export function FontSelect({ label, value, onChange, pageFont }: FontProps) {
  const options = pageFont
    ? [
        { value: pageFont, label: `Standard (${pageFont})` },
        ...FONT_OPTIONS.filter((f) => f.value !== pageFont).map((f) => ({ value: f.value, label: f.label })),
      ]
    : FONT_OPTIONS.map((f) => ({ value: f.value, label: f.label }));
  return (
    <Field label={label}>
      <Select size="sm" value={value} options={options} onChange={(font) => onChange(font as PdfFontFamily)} />
    </Field>
  );
}
