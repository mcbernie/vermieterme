import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsView } from "@/features/settings/settings-view";
import { json, mockFetch, renderWithUi } from "./settings-helpers";

vi.mock("next/navigation", () => ({ useRouter: () => ({ push: vi.fn() }) }));

afterEach(() => vi.unstubAllGlobals());

const ROUTES = {
  "GET /api/settings": json({ id: "l1", name: "Max", street: "Weg 1", zip: "1", city: "C", iban: null }),
  "GET /api/cost-categories": json([]),
  "GET /api/users": json([]),
  "POST /api/backup": json({ success: true }),
};

describe("SettingsView", () => {
  it("loads all sections and reloads them after an import", async () => {
    const { calls } = mockFetch(ROUTES);
    renderWithUi(<SettingsView />);
    expect(await screen.findByDisplayValue("Max")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Allgemein" })).toHaveAttribute("aria-selected", "true");
    const gets = () => calls.filter((c) => c.method === "GET").map((c) => c.url).sort();
    expect(gets()).toEqual(["/api/cost-categories", "/api/settings", "/api/users"]);

    const file = new File(['{"version":1}'], "b.json");
    fireEvent.change(screen.getByTestId("backup-file"), { target: { files: [file] } });
    fireEvent.click(await screen.findByRole("button", { name: "Importieren" }));
    expect(await screen.findByText(/Import erfolgreich/)).toBeInTheDocument();
    await waitFor(() => expect(gets()).toHaveLength(6));
    expect(await screen.findByDisplayValue("Max")).toBeInTheDocument();
  });
});
