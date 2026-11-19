import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { TenantsView } from "@/features/tenants/tenants-view";
import { PROPERTY, RENT_CHANGES, TENANT } from "../features/tenant-fixtures";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

/** Stubs the API of the page; `override` replaces single answers by `METHOD url`. */
function stubApi(override: Record<string, { status: number; body: unknown }> = {}) {
  const answers: Record<string, { status: number; body: unknown }> = {
    "GET /api/tenants": { status: 200, body: [TENANT] },
    "GET /api/properties": { status: 200, body: [{ ...PROPERTY, units: undefined }] },
    "GET /api/properties/p1": { status: 200, body: PROPERTY },
    "GET /api/rent-changes": { status: 200, body: RENT_CHANGES },
    "DELETE /api/tenants/t1": { status: 200, body: { success: true } },
    ...override,
  };
  const fetchMock = vi.fn((url: string, init?: RequestInit) => {
    const a = answers[`${init?.method ?? "GET"} ${url}`] ?? { status: 404, body: { error: "Unbekannt" } };
    return Promise.resolve(new Response(JSON.stringify(a.body), { status: a.status }));
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function setup() {
  render(
    <UiProvider locale="de-DE">
      <ConfirmProvider>
        <TenantsView />
      </ConfirmProvider>
    </UiProvider>,
  );
}

describe("TenantsView", () => {
  it("lists the tenants with latest rent and prepayment", async () => {
    stubApi();
    setup();
    expect(await screen.findByText("Frau Erika Muster")).toBeInTheDocument();
    expect(screen.getByText("650,00 €")).toBeInTheDocument();
    expect(screen.getByText("180,00 €")).toBeInTheDocument();
    expect(screen.getByText("Indexmiete")).toBeInTheDocument();
  });

  it("deletes a tenant only after confirmation", async () => {
    const fetchMock = stubApi();
    setup();
    await screen.findByText("Frau Erika Muster");
    fireEvent.click(screen.getByRole("button", { name: "Löschen" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/Frau Erika Muster wirklich löschen/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Löschen" }));
    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledWith("/api/tenants/t1", { method: "DELETE" }));
    // The list is reloaded after the delete.
    await vi.waitFor(() => expect(fetchMock.mock.calls.filter(([url]) => url === "/api/tenants")).toHaveLength(2));
    await screen.findByText("Frau Erika Muster");
  });

  it("shows a failed delete", async () => {
    stubApi({ "DELETE /api/tenants/t1": { status: 500, body: {} } });
    setup();
    await screen.findByText("Frau Erika Muster");
    fireEvent.click(screen.getByRole("button", { name: "Löschen" }));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Löschen" }));
    expect(await screen.findByText("Die Anfrage ist fehlgeschlagen (Status 500).")).toBeInTheDocument();
  });

  it("shows a load error with retry", async () => {
    stubApi({ "GET /api/tenants": { status: 401, body: { error: "Nicht angemeldet" } } });
    setup();
    expect(await screen.findByText("Nicht angemeldet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Erneut laden" })).toBeInTheDocument();
  });

  it("opens the detail panel on row click", async () => {
    stubApi();
    setup();
    fireEvent.click(await screen.findByText("Frau Erika Muster"));
    const panel = await screen.findByRole("dialog");
    expect(within(panel).getByText("Miete und Vorauszahlung")).toBeInTheDocument();
    expect(within(panel).getByText("Mietvertrag & Dokumente")).toBeInTheDocument();
  });
});
