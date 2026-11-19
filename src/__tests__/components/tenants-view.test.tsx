import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { TenantsView } from "@/features/tenants/tenants-view";
import { SESSION_EXPIRED } from "@/features/tenants/request";
import { defaultAnswers, stubApi } from "../features/tenant-api-stub";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function setup() {
  render(
    <UiProvider locale="de-DE">
      <ConfirmProvider>
        <TenantsView />
      </ConfirmProvider>
    </UiProvider>,
  );
}

/** Clicks „Löschen“ in the row and confirms in the dialog. */
async function deleteAndConfirm() {
  fireEvent.click(screen.getByRole("button", { name: "Löschen" }));
  const dialog = await screen.findByRole("dialog");
  expect(within(dialog).getByText(/Frau Erika Muster wirklich löschen/)).toBeInTheDocument();
  fireEvent.click(within(dialog).getByRole("button", { name: "Löschen" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
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

  it("deletes a tenant after confirmation and shows the reloaded list", async () => {
    const { fetchMock, answers } = stubApi();
    setup();
    await screen.findByText("Frau Erika Muster");
    answers["GET /api/tenants"] = { status: 200, body: [] };
    await deleteAndConfirm();
    expect(await screen.findByText("Noch keine Mieter vorhanden")).toBeInTheDocument();
    expect(screen.queryByText("Frau Erika Muster")).not.toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith("/api/tenants/t1", { method: "DELETE" });
  });

  it("shows a failed delete and keeps the tenant", async () => {
    const answers = defaultAnswers();
    answers["DELETE /api/tenants/t1"] = { status: 500, body: {} };
    stubApi(answers);
    setup();
    await screen.findByText("Frau Erika Muster");
    await deleteAndConfirm();
    expect(await screen.findByText("Die Anfrage ist fehlgeschlagen (Status 500).")).toBeInTheDocument();
    expect(screen.getByText("Frau Erika Muster")).toBeInTheDocument();
  });

  it("shows a load error with retry and no empty state", async () => {
    const answers = defaultAnswers();
    answers["GET /api/tenants"] = { status: 401, body: { error: "Nicht angemeldet" } };
    stubApi(answers);
    setup();
    expect(await screen.findByText("Nicht angemeldet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Erneut laden" })).toBeInTheDocument();
    expect(screen.queryByText("Noch keine Mieter vorhanden")).not.toBeInTheDocument();
  });

  it("ends loading with a session hint when the API answers with the login page", async () => {
    const answers = defaultAnswers();
    answers["GET /api/properties"] = new Response("<html>Anmelden</html>", {
      status: 200,
      headers: { "Content-Type": "text/html" },
    });
    stubApi(answers);
    setup();
    expect(await screen.findByText(SESSION_EXPIRED)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Erneut laden" })).toBeInTheDocument();
    // The tenants themselves loaded; the table is not stuck in its loading state.
    expect(screen.getByText("Frau Erika Muster")).toBeInTheDocument();
  });

  it("loads again on retry", async () => {
    const answers = defaultAnswers();
    answers["GET /api/tenants"] = { status: 500, body: {} };
    stubApi(answers);
    setup();
    const retry = await screen.findByRole("button", { name: "Erneut laden" });
    answers["GET /api/tenants"] = defaultAnswers()["GET /api/tenants"];
    fireEvent.click(retry);
    expect(await screen.findByText("Frau Erika Muster")).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("button", { name: "Erneut laden" })).not.toBeInTheDocument());
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
