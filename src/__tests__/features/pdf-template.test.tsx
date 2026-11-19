import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getDefaultConfig } from "@/lib/pdf-template";
import { PdfTemplateView } from "@/features/settings/pdf-template-view";
import { SettingsFrame } from "@/features/settings/settings-layout";
import { SESSION_EXPIRED } from "@/features/settings/request";
import type { PdfTemplateConfig } from "@/types/pdf-template";
import { json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

const push = vi.fn();
vi.mock("next/navigation", () => ({ usePathname: () => "/settings/pdf-template", useRouter: () => ({ push }) }));

afterEach(() => {
  vi.unstubAllGlobals();
  push.mockClear();
});

const preview = () => within(screen.getByTestId("pdf-preview"));

describe("PdfTemplateView", () => {
  it("offers no saving when the template could not be loaded", async () => {
    mockFetch({ "GET /api/pdf-template": loginPage() });
    renderWithUi(<PdfTemplateView />);
    expect(await screen.findByText(SESSION_EXPIRED)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Speichern" })).not.toBeInTheDocument();
  });

  it("edits a section selected by keyboard and saves the configuration", async () => {
    const { calls } = mockFetch({
      "GET /api/pdf-template": json({ id: null, name: "Standard", config: getDefaultConfig() }),
      "PUT /api/pdf-template": json({ id: "t1", name: "Standard" }),
    });
    renderWithUi(<PdfTemplateView />);
    await screen.findByTestId("pdf-preview");
    const title = preview().getByRole("button", { name: "Titel" });
    fireEvent.keyDown(title, { key: "Enter" });
    expect(title).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("switch", { name: "Sichtbar" }));
    expect(preview().queryByRole("button", { name: "Titel" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText("Gespeichert")).toBeInTheDocument();
    const sent = calls.find((c) => c.method === "PUT")?.body as { config: PdfTemplateConfig };
    expect(sent.config.sections.title.visible).toBe(false);
    expect(sent.config.sections.header.visible).toBe(true);
  });

  it("shows a failed save and resets to the default", async () => {
    const stored = getDefaultConfig();
    stored.sections.title.visible = false;
    mockFetch({ "GET /api/pdf-template": json({ config: stored }), "PUT /api/pdf-template": loginPage() });
    renderWithUi(<PdfTemplateView />);
    await screen.findByTestId("pdf-preview");
    expect(preview().queryByRole("button", { name: "Titel" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText(SESSION_EXPIRED)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Zurücksetzen" }));
    expect(preview().getByRole("button", { name: "Titel" })).toBeInTheDocument();
  });

  it("does not report „Gespeichert“ for edits made while saving", async () => {
    let release: (r: Response) => void = () => {};
    const { calls } = mockFetch({
      "GET /api/pdf-template": json({ config: getDefaultConfig() }),
      "PUT /api/pdf-template": () => new Promise<Response>((r) => (release = r)),
    });
    renderWithUi(<PdfTemplateView />);
    await screen.findByTestId("pdf-preview");
    fireEvent.click(preview().getByRole("button", { name: "Titel" }));
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    fireEvent.click(screen.getByRole("switch", { name: "Fett" }));
    await act(async () => release(json({ id: "t1" })));
    expect(calls.filter((c) => c.method === "PUT")).toHaveLength(1);
    expect(screen.queryByText("Gespeichert")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    await act(async () => release(json({ id: "t1" })));
    expect(await screen.findByText("Gespeichert")).toBeInTheDocument();
  });

  it("lists hidden sections so they can be switched on again", async () => {
    const stored = getDefaultConfig();
    stored.sections.closing.visible = false;
    mockFetch({ "GET /api/pdf-template": json({ config: stored }) });
    renderWithUi(<PdfTemplateView />);
    await screen.findByTestId("pdf-preview");
    const aside = within(screen.getByRole("complementary", { name: "Eigenschaften" }));
    fireEvent.click(aside.getByRole("button", { name: "Schlusstext" }));
    fireEvent.click(screen.getByRole("switch", { name: "Sichtbar" }));
    expect(preview().getByRole("button", { name: "Schlusstext" })).toBeInTheDocument();
    expect(screen.queryByText("Ausgeblendete Bereiche")).not.toBeInTheDocument();
  });

  it("asks before switching the sub page with unsaved changes", async () => {
    mockFetch({ "GET /api/pdf-template": json({ config: getDefaultConfig() }) });
    render(
      <UiProvider locale="de-DE">
        <ConfirmProvider>
          <SettingsFrame>
            <PdfTemplateView />
          </SettingsFrame>
        </ConfirmProvider>
      </UiProvider>,
    );
    await screen.findByTestId("pdf-preview");
    fireEvent.click(screen.getByRole("link", { name: "VPI-Werte" }));
    await waitFor(() => expect(push).toHaveBeenCalledTimes(1));

    fireEvent.click(preview().getByRole("button", { name: "Titel" }));
    fireEvent.click(screen.getByRole("switch", { name: "Kursiv" }));
    fireEvent.click(screen.getByRole("link", { name: "VPI-Werte" }));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(push).toHaveBeenCalledTimes(1);
  });
  describe("compares the content, not the object (#38)", () => {
    /** Renders the editor inside the settings frame with the stored template and waits for the preview. */
    async function renderFrame(stored = getDefaultConfig()) {
      mockFetch({
        "GET /api/pdf-template": json({ config: stored }),
        "PUT /api/pdf-template": json({ id: "t1" }),
      });
      render(
        <UiProvider locale="de-DE">
          <ConfirmProvider>
            <SettingsFrame>
              <PdfTemplateView />
            </SettingsFrame>
          </ConfirmProvider>
        </UiProvider>,
      );
      await screen.findByTestId("pdf-preview");
    }

    /** Clicks the VPI tab and expects it to navigate at once, without the leave dialog. */
    async function expectLeaveWithoutDialog() {
      fireEvent.click(screen.getByRole("link", { name: "VPI-Werte" }));
      await waitFor(() => expect(push).toHaveBeenCalledWith("/settings/vpi"));
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    }

    it("does not count clicking the alignment that is already active", async () => {
      await renderFrame();
      fireEvent.click(preview().getByRole("button", { name: "Titel" }));
      fireEvent.click(screen.getByRole("radio", { name: "Mitte" }));
      await expectLeaveWithoutDialog();
    });

    it("does not count a switch toggled twice", async () => {
      await renderFrame();
      fireEvent.click(preview().getByRole("button", { name: "Titel" }));
      fireEvent.click(screen.getByRole("switch", { name: "Fett" }));
      fireEvent.click(screen.getByRole("switch", { name: "Fett" }));
      await expectLeaveWithoutDialog();
    });

    it("does not count a reset when the stored template already is the default", async () => {
      const stored = getDefaultConfig();
      const reordered = { sections: stored.sections, page: stored.page } as PdfTemplateConfig;
      await renderFrame(reordered);
      fireEvent.click(screen.getByRole("button", { name: "Zurücksetzen" }));
      await expectLeaveWithoutDialog();
    });

    it("still asks for a real change (counter-check)", async () => {
      await renderFrame();
      fireEvent.click(preview().getByRole("button", { name: "Titel" }));
      fireEvent.click(screen.getByRole("radio", { name: "Links" }));
      fireEvent.click(screen.getByRole("link", { name: "VPI-Werte" }));
      expect(await screen.findByRole("dialog")).toBeInTheDocument();
      expect(push).not.toHaveBeenCalled();
    });

    it("keeps „Gespeichert“ after a switch was toggled twice and drops it for a real change", async () => {
      await renderFrame();
      fireEvent.click(preview().getByRole("button", { name: "Titel" }));
      fireEvent.click(screen.getByRole("switch", { name: "Kursiv" }));
      fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
      expect(await screen.findByText("Gespeichert")).toBeInTheDocument();
      fireEvent.click(screen.getByRole("switch", { name: "Fett" }));
      fireEvent.click(screen.getByRole("switch", { name: "Fett" }));
      expect(screen.getByText("Gespeichert")).toBeInTheDocument();
      fireEvent.click(screen.getByRole("switch", { name: "Fett" }));
      expect(screen.queryByText("Gespeichert")).not.toBeInTheDocument();
    });
  });
});
