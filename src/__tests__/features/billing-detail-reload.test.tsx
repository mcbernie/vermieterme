import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import type { CostCategory } from "@/types";
import { categories, detail, json } from "./billing-fixtures";
import { clickSave, mount, stub, typeMoney } from "./billing-detail-harness";

/** The fixture categories plus one without an entry in the period. */
const withHeating: CostCategory[] = [
  ...categories,
  { id: "c3", name: "Heizung", distributionKey: "MEA", sortOrder: 3 },
];

const hideButtons = () => screen.queryAllByRole("button", { name: "In dieser Abrechnung ausblenden" });
const showButtons = () => screen.queryAllByRole("button", { name: "In dieser Abrechnung einblenden" });

describe("BillingDetailView reload", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows a category without entry again when hiding it fails (#61)", async () => {
    stub(() => json({}, 500), 200, detail, withHeating);
    await mount();
    expect(hideButtons()).toHaveLength(3);
    await act(async () => fireEvent.click(hideButtons()[2]!));
    expect(screen.getByText("Die Kostenart konnte nicht umgeschaltet werden.")).toBeInTheDocument();
    expect(hideButtons()).toHaveLength(3);
    expect(showButtons()).toHaveLength(0);
  });

  it("shows amounts changed in another tab after a reload once its own change is saved (#63)", async () => {
    let water = 300;
    stub(undefined, 200, () => {
      const bp = detail();
      bp.costs[1]!.totalAmount = water;
      return bp;
    });
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1500");
    await clickSave();
    water = 450; // stored from another tab meanwhile
    await act(async () => fireEvent.click(screen.getByRole("button", { name: "Bestätigen" })));
    expect(screen.getByLabelText("Gesamtbetrag Wasser")).toHaveValue("450,00 €");
  });

  it("keeps an amount whose save failed across a reload (#63)", async () => {
    stub((url, init) => {
      const body = JSON.parse(init?.body as string);
      return url.endsWith("/costs") && "totalAmount" in body && !("reviewed" in body) ? json({}, 500) : undefined;
    });
    await mount();
    await typeMoney("Gesamtbetrag Grundsteuer", "1500");
    await clickSave();
    expect(screen.getByText("Fehler beim Speichern")).toBeInTheDocument();
    await act(async () => fireEvent.click(hideButtons()[1]!));
    expect(screen.getByLabelText("Gesamtbetrag Grundsteuer")).toHaveValue("1.500,00 €");
  });

});
