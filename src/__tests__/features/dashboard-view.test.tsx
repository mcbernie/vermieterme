import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, screen } from "@testing-library/react";
import { DashboardView } from "@/features/dashboard/dashboard-view";
import { onSessionExpired } from "@/lib/session-expiry";
import { json, loginPage, mockFetch, renderWithUi, type Routes } from "./settings-helpers";

vi.mock("next/navigation", () => ({ useRouter: () => ({ push: vi.fn() }) }));

const LOAD_FAILED = "Einige Daten konnten nicht geladen werden. Bitte laden Sie die Seite neu.";

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
    expect(screen.queryByText(LOAD_FAILED)).toBeNull();
  });

  it("shows a load failure instead of silently empty figures", async () => {
    mockFetch(routes({ "GET /api/billing-periods": json({ error: "x" }, 500) }));
    await mount();
    expect(screen.getByText(LOAD_FAILED)).toBeTruthy();
  });

  it("reports the login page as answer as expired session", async () => {
    const expired = vi.fn();
    const off = onSessionExpired(expired);
    mockFetch(routes({ "GET /api/properties": loginPage() }));
    await mount();
    off();
    expect(expired).toHaveBeenCalled();
    expect(screen.getByText(LOAD_FAILED)).toBeTruthy();
  });
});
