import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { BillingDetailView } from "@/features/billing/billing-detail-view";
import { categories, detail, json } from "./billing-fixtures";

type Handler = (url: string, init?: RequestInit) => Response | undefined;

/** Routes fetch: GETs answer with the fixture, `writes` may answer writes (default: ok). */
function stub(writes: Handler = () => undefined, periodStatus = 200) {
  const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    if (method !== "GET") return writes(url, init) ?? json({});
    if (url === "/api/billing-periods/bp") return json(periodStatus === 200 ? detail() : { error: "x" }, periodStatus);
    if (url === "/api/cost-categories") return json(categories);
    return json([]);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

async function mount() {
  await act(async () => {
    render(
      <UiProvider locale="de-DE">
        <ConfirmProvider>
          <BillingDetailView id="bp" />
        </ConfirmProvider>
      </UiProvider>,
    );
  });
}

const writesTo = (fetchMock: ReturnType<typeof stub>, part: string) =>
  fetchMock.mock.calls.filter((c) => c[1]?.method && c[1].method !== "GET" && c[0].includes(part));

/** Types into a money field and leaves it. */
async function typeMoney(label: string, text: string) {
  const input = screen.getByLabelText(label);
  await act(async () => fireEvent.focus(input));
  await act(async () => fireEvent.change(input, { target: { value: text } }));
  await act(async () => fireEvent.blur(input));
}

describe("BillingDetailView", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("says when the billing period does not exist", async () => {
    stub(undefined, 404);
    await mount();
    expect(screen.getByText("Abrechnung nicht gefunden.")).toBeInTheDocument();
  });

  it("shows a load error with retry instead of an empty page", async () => {
    stub(undefined, 500);
    await mount();
    expect(screen.getByText("Die Abrechnung konnte nicht geladen werden.")).toBeInTheDocument();
  });

  it("saves an edited amount in euros", async () => {
    const fetchMock = stub();
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1.250,50");
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Speichern" })));
    const body = writesTo(fetchMock, "/costs").map((c) => JSON.parse(c[1]?.body as string));
    expect(body).toContainEqual({ costCategoryId: "c1", totalAmount: 1250.5, unitAmount: null });
    expect(screen.getByText("Gespeichert")).toBeInTheDocument();
  });

  it("marks an unreadable amount at the field and does not save", async () => {
    const fetchMock = stub();
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1.2.3");
    expect(screen.getByText("Bitte einen gültigen Betrag eingeben.")).toBeInTheDocument();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Speichern" })));
    expect(writesTo(fetchMock, "/costs")).toHaveLength(0);
    expect(screen.getByText(/Bitte korrigieren Sie die markierten Beträge/)).toBeInTheDocument();
  });

  it("shows a notice when saving fails", async () => {
    stub((url) => (url.endsWith("/prepayments") ? json({}, 500) : undefined));
    await mount();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Speichern" })));
    expect(screen.getByText("Fehler beim Speichern")).toBeInTheDocument();
  });

  it("shows a notice when hiding a cost category fails and reloads the stored state", async () => {
    const fetchMock = stub(() => json({ error: "Kaputt" }, 500));
    await mount();
    const gets = () => fetchMock.mock.calls.filter((c) => c[0] === "/api/billing-periods/bp" && !c[1]?.method).length;
    const before = gets();
    await act(async () => fireEvent.click(screen.getAllByRole("button", { name: "In dieser Abrechnung ausblenden" })[0]!));
    expect(screen.getByText("Kaputt")).toBeInTheDocument();
    expect(gets()).toBe(before + 1);
  });

  it("shows a notice when confirming a position fails", async () => {
    stub(() => json({}, 500));
    await mount();
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Bestätigen" })));
    expect(screen.getByText("Die Position konnte nicht bestätigt werden.")).toBeInTheDocument();
  });

  it("does not store an unreadable sent date and shows the error at the field", async () => {
    const fetchMock = stub();
    await mount();
    const input = screen.getByLabelText("Versendet am");
    await act(async () => {
      fireEvent.change(input, { target: { value: "32.13.2025" } });
      fireEvent.blur(input);
    });
    expect(screen.getByText("Bitte ein gültiges Datum eingeben.")).toBeInTheDocument();
    expect(writesTo(fetchMock, "/api/billing-periods/bp")).toHaveLength(0);
  });

  it("stores a readable sent date", async () => {
    const fetchMock = stub();
    await mount();
    const input = screen.getByLabelText("Versendet am");
    await act(async () => {
      fireEvent.change(input, { target: { value: "15.02.2026" } });
      fireEvent.blur(input);
    });
    const puts = writesTo(fetchMock, "/api/billing-periods/bp").map((c) => JSON.parse(c[1]?.body as string));
    expect(puts).toEqual([{ sentDate: "2026-02-15" }]);
  });
});
