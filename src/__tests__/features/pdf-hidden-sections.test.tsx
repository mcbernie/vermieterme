import "@testing-library/jest-dom/vitest";
import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { getDefaultConfig } from "@/lib/pdf-template";
import { PdfHiddenSections } from "@/features/settings/pdf-hidden-sections";
import type { PdfTemplateConfig, SectionConfig } from "@/types/pdf-template";
import { renderWithUi } from "./settings-helpers";

/** Default template with the title and closing hidden, plus `extra` sections (e.g. unknown ids). */
function configWith(extra: Record<string, SectionConfig> = {}): PdfTemplateConfig {
  const config = getDefaultConfig();
  config.sections.title.visible = false;
  config.sections.closing.visible = false;
  return { ...config, sections: { ...config.sections, ...extra } as PdfTemplateConfig["sections"] };
}

describe("PdfHiddenSections", () => {
  it("marks the selected hidden section with aria-pressed", () => {
    const onSelect = vi.fn();
    renderWithUi(<PdfHiddenSections config={configWith()} selected="closing" onSelect={onSelect} />);
    expect(screen.getByRole("button", { name: "Schlusstext" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Titel" })).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(screen.getByRole("button", { name: "Titel" }));
    expect(onSelect).toHaveBeenCalledWith("title");
  });

  it("skips section ids it does not know instead of crashing", () => {
    const hiddenUnknown = { ...getDefaultConfig().sections.title, visible: false };
    renderWithUi(<PdfHiddenSections config={configWith({ legacyFooter: hiddenUnknown })} onSelect={vi.fn()} />);
    expect(screen.getAllByRole("button").map((b) => b.textContent)).toEqual(["Titel", "Schlusstext"]);
  });

  it("renders nothing when only unknown sections are hidden", () => {
    const config = getDefaultConfig();
    const hiddenUnknown = { ...config.sections.title, visible: false };
    const withUnknown = { ...config, sections: { ...config.sections, legacy: hiddenUnknown } } as PdfTemplateConfig;
    renderWithUi(<PdfHiddenSections config={withUnknown} onSelect={vi.fn()} />);
    expect(screen.queryByText("Ausgeblendete Bereiche")).not.toBeInTheDocument();
  });
});
