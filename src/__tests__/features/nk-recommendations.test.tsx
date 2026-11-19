import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { nkRecommendations, seedValues, unitResults } from "@/features/billing/billing-calc";
import { NkRecommendations } from "@/features/billing/nk-recommendations";
import { categories, detail, json } from "./billing-fixtures";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

function mount() {
  const bp = detail();
  const seeded = seedValues(bp);
  const recs = nkRecommendations(unitResults(bp, categories, seeded.costs, seeded.prepayments));
  render(
    <UiProvider locale="de-DE">
      <NkRecommendations recommendations={recs} periodEnd={bp.endDate} />
    </UiProvider>,
  );
}

async function openAndSubmit(times = 1) {
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Anpassung anlegen" })));
  const form = (screen.getByLabelText(/Gültig ab/) as HTMLInputElement).closest("form") as HTMLFormElement;
  await act(async () => {
    for (let i = 0; i < times; i++) fireEvent.submit(form);
  });
}

describe("NkRecommendations", () => {
  it("recommends 125 € per month after a back payment of 300 €", () => {
    mount();
    expect(screen.getByText("125,00 €")).toBeInTheDocument();
    expect(screen.getByText("300,00 €")).toBeInTheDocument();
  });

  it("creates the prepayment change once and marks it as created", async () => {
    const fetchMock = vi.fn(async () => json({}));
    vi.stubGlobal("fetch", fetchMock);
    mount();
    await openAndSubmit(2);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(JSON.parse((fetchMock.mock.calls[0] as unknown as [string, RequestInit])[1].body as string)).toEqual({
      unitId: "u1",
      type: "prepayment",
      amount: 125,
      effectiveDate: "2026-01-01",
      reason: "NK-Anpassung nach Abrechnung (Nachzahlung 300.00 €)",
    });
    expect(screen.getByText("Anpassung angelegt")).toBeInTheDocument();
  });

  it("keeps the dialog open with a message when creating fails", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => json({}, 500)));
    mount();
    await openAndSubmit();
    expect(screen.getByText("Die Anpassung konnte nicht angelegt werden.")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.queryByText("Anpassung angelegt")).not.toBeInTheDocument();
  });
});
