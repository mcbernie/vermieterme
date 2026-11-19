import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, screen } from "@testing-library/react";
import { DashboardView } from "@/features/dashboard/dashboard-view";
import { onSessionExpired } from "@/lib/session-expiry";
import { json, loginPage, mockFetch, renderWithUi, type Routes } from "./settings-helpers";

vi.mock("next/navigation", () => ({ useRouter: () => ({ push: vi.fn() }) }));

const PARTLY_FAILED = "Einige Daten konnten nicht geladen werden. Bitte laden Sie die Seite neu.";

/** Routes of a working dashboard; single entries are replaced per test. */
function routes(overrides: Routes = {}): Routes {
  return {
    "GET /api/properties": json([]),
    "GET /api/billing-periods": json([]),
    "GET /api/settings": json({ name: "Vermieter Test" }),
    "GET /api/vpi/suggestions": json([]),
    ...overrides,
  };
}

/** Renders the dashboard and waits for the loads. */
async function mount() {
  await act(async () => {
    renderWithUi(<DashboardView />);
  });
}

describe("DashboardView", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows the greeting and no load failure", async () => {
    mockFetch(routes());
    await mount();
    expect(screen.getByText("Willkommen, Vermieter Test")).toBeTruthy();
    expect(screen.queryByText(PARTLY_FAILED)).toBeNull();
  });

  it("shows a load failure instead of silently empty figures", async () => {
    mockFetch(routes({ "GET /api/billing-periods": json({ error: "x" }, 500) }));
    await mount();
    expect(screen.getByText(PARTLY_FAILED)).toBeTruthy();
  });

  it("reports the login page as answer as expired session", async () => {
    const expired = vi.fn();
    const off = onSessionExpired(expired);
    mockFetch(routes({ "GET /api/properties": loginPage() }));
    await mount();
    off();
    expect(expired).toHaveBeenCalled();
    expect(screen.getByText(PARTLY_FAILED)).toBeTruthy();
  });

  it("keeps the figures shown so far when the reload after an adjustment fails", async () => {
    const suggestion = {
      tenantId: "t1", unitId: "u1", tenantName: "Max Muster", unitName: "EG", propertyAddress: "Weg 1",
      leaseType: "index", currentRent: 500, referenceVpi: 100, currentVpi: 110, percentageChange: 10,
      suggestedRent: 550, monthsSinceLastAdjustment: 14, minMonths: 12, eligible: true, baseYear: 2020,
    };
    let reload = false;
    const flaky = (data: unknown) => () => (reload ? json({ error: "x" }, 500) : json(data));
    mockFetch(
      routes({
        "GET /api/properties": flaky([{ id: "p1", _count: { units: 3 } }]),
        "GET /api/vpi/suggestions": flaky([suggestion]),
        "POST /api/rent-changes": json({}, 201),
        "PUT /api/tenants/t1": () => {
          reload = true;
          return json({});
        },
      }),
    );
    await mount();
    expect(screen.getByText("Max Muster")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Anpassung durchführen" }));
    await act(async () => {
      fireEvent.click(await screen.findByRole("button", { name: "Anpassung anlegen" }));
    });
    expect(await screen.findByText(/konnte nicht aktualisiert werden/)).toBeTruthy();
    expect(screen.getByText("Max Muster")).toBeTruthy();
    expect(screen.queryByText(PARTLY_FAILED)).toBeNull();
  });
});
