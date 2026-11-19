import "@testing-library/jest-dom/vitest";
import { act, fireEvent, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getDefaultConfig } from "@/lib/pdf-template";
import { PdfTemplateView } from "@/features/settings/pdf-template-view";
import { SESSION_EXPIRED } from "@/features/settings/request";
import type { PdfTemplateConfig } from "@/types/pdf-template";
import { json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

const push = vi.fn();
vi.mock("next/navigation", () => ({ useRouter: () => ({ push }) }));

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

  it("navigates between the settings tabs", async () => {
    mockFetch({ "GET /api/pdf-template": json({ config: getDefaultConfig() }) });
    renderWithUi(<PdfTemplateView />);
    fireEvent.click(await screen.findByRole("tab", { name: "VPI-Werte" }));
    expect(push).toHaveBeenCalledWith("/settings/vpi");
    expect(screen.getByRole("tab", { name: "PDF-Vorlage" })).toHaveAttribute("aria-selected", "true");
  });
});
