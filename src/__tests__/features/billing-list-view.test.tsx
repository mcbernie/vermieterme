import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { BillingListView } from "@/features/billing/billing-list-view";
import { json } from "./billing-fixtures";

const prop = { id: "p1", street: "Weg 1", zip: "28195", city: "Bremen", totalShares: 100 };
const period = {
  id: "bp1",
  propertyId: "p1",
  startDate: `${new Date().getFullYear() - 1}-01-01T00:00:00.000Z`,
  endDate: `${new Date().getFullYear() - 1}-12-31T00:00:00.000Z`,
  billingDate: null,
  sentDate: null,
  paidDate: null,
  copiedFromId: null,
  property: prop,
  _count: { costs: 0 },
};

function stub(deleteStatus = 200, failLoad = false) {
  const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
    if (init?.method === "DELETE") return json({}, deleteStatus);
    if (failLoad) return json({}, 500);
    return json(url === "/api/properties" ? [prop] : [period]);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

async function mount() {
  await act(async () => {
    render(
      <UiProvider locale="de-DE">
        <ConfirmProvider>
          <BillingListView />
        </ConfirmProvider>
      </UiProvider>,
    );
  });
}

const deletes = (fetchMock: ReturnType<typeof stub>) => fetchMock.mock.calls.filter((c) => c[1]?.method === "DELETE");

async function clickDelete() {
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Löschen" })));
}

describe("BillingListView", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("groups periods under their property with status", async () => {
    stub();
    await mount();
    expect(screen.getByRole("heading", { name: /Weg 1/ })).toBeInTheDocument();
    expect(screen.getByText("Offen")).toBeInTheDocument();
  });

  it("asks before deleting and does nothing when cancelled", async () => {
    const fetchMock = stub();
    await mount();
    await clickDelete();
    const dialog = screen.getByRole("dialog", { name: "Abrechnung löschen" });
    await act(async () => fireEvent.click(within(dialog).getByRole("button", { name: "Abbrechen" })));
    expect(deletes(fetchMock)).toHaveLength(0);
  });

  it("shows a notice when deleting fails", async () => {
    const fetchMock = stub(500);
    await mount();
    await clickDelete();
    await act(async () =>
      fireEvent.click(within(screen.getByRole("dialog", { name: "Abrechnung löschen" })).getByRole("button", { name: "Löschen" })),
    );
    expect(deletes(fetchMock)).toHaveLength(1);
    expect(screen.getByText("Die Abrechnung konnte nicht gelöscht werden.")).toBeInTheDocument();
  });

  it("shows a load error instead of the empty state", async () => {
    stub(200, true);
    await mount();
    expect(screen.getByText("Die Abrechnungen konnten nicht geladen werden.")).toBeInTheDocument();
    expect(screen.queryByText(/Noch keine Abrechnungen/)).not.toBeInTheDocument();
  });

  it("shows the loading state while retrying a failed load", async () => {
    const fetchMock = stub(200, true);
    await mount();
    fetchMock.mockImplementation(() => new Promise<Response>(() => {}));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Erneut versuchen" })));
    expect(screen.queryByText("Die Abrechnungen konnten nicht geladen werden.")).not.toBeInTheDocument();
    expect(document.querySelector('[data-slot="loading-state"]')).toBeInTheDocument();
  });
});
