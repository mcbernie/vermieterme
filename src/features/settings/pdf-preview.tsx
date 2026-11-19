"use client";

import type { CSSProperties, KeyboardEvent } from "react";
import { fontToCss, SECTION_META } from "@/lib/pdf-template";
import type { PdfTemplateConfig, SectionId } from "@/types/pdf-template";
import { BillingBody, type SectionBox } from "./pdf-preview-body";

/** Selected part of the template: a section, the page settings or nothing. */
export type Selection = SectionId | "page" | null;

const A4_WIDTH = 595;
const A4_HEIGHT = 842;
const PREVIEW_WIDTH = 520;
const SCALE = PREVIEW_WIDTH / A4_WIDTH;

/**
 * The preview imitates the printed page, so it keeps paper colours and the fonts chosen for the PDF
 * independent of the app theme. These are the only inline colours and fonts on purpose.
 */
const PAPER: CSSProperties = { background: "#ffffff", color: "#000000" };

interface Props {
  config: PdfTemplateConfig;
  selected: Selection;
  onSelect: (selection: Selection) => void;
}

/**
 * Scaled A4 preview of the billing with sample data. Clicking (or Enter/Space on) a section selects it,
 * clicking the page background selects the page settings.
 */
export function PdfPreview({ config, selected, onSelect }: Props) {
  const { page, sections } = config;

  const box: SectionBox = (id, children) => {
    const section = sections[id];
    if (!section.visible) return null;
    const isSelected = selected === id;
    const { style } = section;
    const select = () => onSelect(id);
    return (
      <div
        role="button"
        tabIndex={0}
        aria-pressed={isSelected}
        aria-label={SECTION_META[id].label}
        className={`relative cursor-pointer outline-offset-2 ${
          isSelected ? "outline-2 outline-accent" : "hover:outline-1 hover:outline-line"
        } focus-visible:outline-2 focus-visible:outline-accent`}
        style={{
          fontFamily: fontToCss(style.fontFamily),
          fontSize: style.fontSize,
          fontWeight: style.bold ? "bold" : "normal",
          fontStyle: style.italic ? "italic" : "normal",
          textAlign: style.textAlign,
          color: style.color,
          marginTop: style.marginTop,
          marginBottom: style.marginBottom,
        }}
        onClick={(e) => {
          e.stopPropagation();
          select();
        }}
        onKeyDown={(e: KeyboardEvent) => {
          if (e.key !== "Enter" && e.key !== " ") return;
          e.preventDefault();
          e.stopPropagation();
          select();
        }}
      >
        {isSelected && (
          <span className="absolute -top-4 left-1 bg-accentsoft px-1 text-[9px] font-medium text-accentstrong">
            {SECTION_META[id].label}
          </span>
        )}
        {children}
      </div>
    );
  };

  return (
    <div
      className={`mx-auto overflow-hidden border ${selected === "page" ? "border-accent" : "border-strong"}`}
      style={{ ...PAPER, width: PREVIEW_WIDTH, height: A4_HEIGHT * SCALE }}
      data-testid="pdf-preview"
    >
      <div
        className="origin-top-left"
        style={{
          width: A4_WIDTH,
          height: A4_HEIGHT,
          transform: `scale(${SCALE})`,
          fontFamily: fontToCss(page.fontFamily),
          fontSize: page.fontSize,
          padding: `${page.paddingTop}px ${page.paddingRight}px ${page.paddingBottom}px ${page.paddingLeft}px`,
        }}
        onClick={() => onSelect("page")}
      >
        <BillingBody config={config} box={box} />
      </div>
    </div>
  );
}
