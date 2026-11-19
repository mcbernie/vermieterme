import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import { detail, json } from "./billing-fixtures";
import { amountSaves, clickSave, mount, stub, typeMoney, typeOnly, writesTo } from "./billing-detail-harness";

describe("BillingDetailView", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => {
    // Fake timers only; pending autosave timers must not reach a later test.
    vi.clearAllTimers();
    vi.useRealTimers();
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

  it("shows the loading state while retrying a failed load", async () => {
    const fetchMock = stub(undefined, 500);
    await mount();
    fetchMock.mockImplementation(() => new Promise<Response>(() => {}));
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Erneut versuchen" })));
    expect(screen.queryByText("Die Abrechnung konnte nicht geladen werden.")).not.toBeInTheDocument();
    expect(document.querySelector('[data-slot="loading-state"]')).toBeInTheDocument();
  });

  it("saves an edited amount in euros", async () => {
    const fetchMock = stub();
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1.250,50");
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Speichern" })));
    const body = writesTo(fetchMock, "/costs").map((c) => JSON.parse(c[1]?.body as string));
    expect(body).toContainEqual({ costCategoryId: "c1", totalAmount: 1250.5, unitAmount: null });
    expect(screen.getByText(/^Gespeichert um/)).toBeInTheDocument();
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
    // Server texts are shown for client errors only; 5xx answers carry an English generic text.
    const fetchMock = stub(() => json({ error: "Kaputt" }, 409));
    await mount();
    const gets = () => fetchMock.mock.calls.filter((c) => c[0] === "/api/billing-periods/bp" && (c[1]?.method ?? "GET") === "GET").length;
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

  it("withdraws an unreadable amount when its field disappears", async () => {
    vi.useFakeTimers();
    const fetchMock = stub();
    await mount();
    await typeMoney("Anteil Wohnung Wasser", "1.2.3");
    expect(screen.getByText("Bitte einen gültigen Betrag eingeben.")).toBeInTheDocument();
    await act(async () =>
      fireEvent.click(screen.getByRole("combobox", { name: "Verteilerschlüssel Wasser" })),
    );
    await act(async () => fireEvent.click(screen.getByRole("option", { name: "MEA" })));
    await clickSave();
    expect(amountSaves(fetchMock).length).toBeGreaterThan(0);
    expect(screen.queryByText(/Nicht gespeichert/)).not.toBeInTheDocument();
  });

  it("does not block saving with an unreadable amount of a hidden category", async () => {
    vi.useFakeTimers();
    let hidden = false;
    const fetchMock = stub(
      (_url, init) => {
        if (JSON.parse(init?.body as string).enabled === false) hidden = true;
        return undefined;
      },
      200,
      () => {
        const bp = detail();
        if (hidden) bp.costs[0]!.enabled = false;
        return bp;
      },
    );
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1.2.3");
    await act(async () =>
      fireEvent.click(screen.getAllByRole("button", { name: "In dieser Abrechnung ausblenden" })[0]!),
    );
    await clickSave();
    expect(amountSaves(fetchMock).length).toBeGreaterThan(0);
    expect(screen.queryByText(/Nicht gespeichert/)).not.toBeInTheDocument();
  });

  it("saves automatically 1.5 s after the last keystroke, without leaving the field", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: false });
    try {
      const fetchMock = stub();
      await mount();
      await typeOnly("Gesamtbetrag Grundsteuer", "1250,5");
      await act(async () => vi.advanceTimersByTime(1499));
      expect(amountSaves(fetchMock)).toHaveLength(0);
      await act(async () => vi.advanceTimersByTime(1));
      expect(amountSaves(fetchMock)).toContainEqual({ costCategoryId: "c1", totalAmount: 1250.5, unitAmount: null });
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps an unsaved amount across tabs and shows it in the summary at once", async () => {
    stub();
    await mount();
    const input = await typeOnly("Gesamtbetrag Grundsteuer", "1500");
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Zusammenfassung" })));
    expect(screen.getByRole("tabpanel")).toHaveTextContent("1.800,00 €");
    await act(async () => fireEvent.click(screen.getByRole("tab", { name: "Kosten" })));
    expect(input).toBeInTheDocument();
    expect(input).toHaveValue("1500");
  });

  it("marks unreadable text while typing and saves no readable intermediate value", async () => {
    vi.useFakeTimers();
    const fetchMock = stub();
    await mount();
    const input = await typeOnly("Gesamtbetrag Grundsteuer", "12");
    await act(async () => fireEvent.input(input, { target: { value: "12x3" } }));
    expect(screen.getByText("Bitte einen gültigen Betrag eingeben.")).toBeInTheDocument();
    await act(async () => vi.advanceTimersByTime(1500));
    expect(amountSaves(fetchMock)).toHaveLength(0);
    await act(async () => fireEvent.input(input, { target: { value: "13" } }));
    expect(screen.queryByText("Bitte einen gültigen Betrag eingeben.")).not.toBeInTheDocument();
    await act(async () => vi.advanceTimersByTime(1500));
    expect(amountSaves(fetchMock)).toContainEqual({ costCategoryId: "c1", totalAmount: 13, unitAmount: null });
    expect(screen.queryByText(/Nicht gespeichert/)).not.toBeInTheDocument();
  });

  it("keeps an unsaved amount when a toggle reloads the period (#19)", async () => {
    stub();
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1500");
    await act(async () => fireEvent.click(screen.getAllByRole("button", { name: "In dieser Abrechnung ausblenden" })[1]!));
    expect(screen.getByLabelText("Gesamtbetrag Grundsteuer")).toHaveValue("1.500,00\u00a0€");
  });

  it("keeps an unsaved amount when confirming another position reloads the period (#19)", async () => {
    stub();
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1500");
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Bestätigen" })));
    expect(screen.getByLabelText("Gesamtbetrag Grundsteuer")).toHaveValue("1.500,00\u00a0€");
  });

  it("treats text without a digit as unfinished: no error while typing, marked when the field is left (#33)", async () => {
    vi.useFakeTimers();
    const fetchMock = stub();
    await mount();
    const input = await typeOnly("Gesamtbetrag Grundsteuer", "-");
    expect(screen.queryByText("Bitte einen gültigen Betrag eingeben.")).not.toBeInTheDocument();
    await act(async () => vi.advanceTimersByTime(1500));
    expect(amountSaves(fetchMock)).toHaveLength(0);
    await act(async () => fireEvent.blur(input));
    expect(screen.getByText("Bitte einen gültigen Betrag eingeben.")).toBeInTheDocument();
  });

  it("saves a negative amount typed with a leading minus (#33)", async () => {
    vi.useFakeTimers();
    const fetchMock = stub();
    await mount();
    const input = await typeOnly("Gesamtbetrag Grundsteuer", "-");
    await act(async () => fireEvent.input(input, { target: { value: "-50" } }));
    expect(screen.queryByText("Bitte einen gültigen Betrag eingeben.")).not.toBeInTheDocument();
    await act(async () => vi.advanceTimersByTime(1500));
    expect(amountSaves(fetchMock)).toContainEqual({ costCategoryId: "c1", totalAmount: -50, unitAmount: null });
  });
});
