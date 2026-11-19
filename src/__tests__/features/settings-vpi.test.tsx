import "@testing-library/jest-dom/vitest";
import { fireEvent, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SESSION_EXPIRED } from "@/lib/api-client";
import { groupByBaseYear, importFromBundesbank } from "@/features/settings/vpi";
import { VpiEntryForm, vpiFormErrors } from "@/features/settings/vpi-entry-form";
import { VpiFetchPanel } from "@/features/settings/vpi-fetch-panel";
import { json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

afterEach(() => vi.unstubAllGlobals());

const entry = (id: string, baseYear: number, year: number, month: number) => ({ id, baseYear, year, month, value: 100 });
const FETCHED = { entries: [{ year: 2024, month: 1, value: 117.4 }, { year: 2024, month: 2, value: 118 }] };

describe("VPI helpers", () => {
  it("groups by base year and sorts newest first", () => {
    const groups = groupByBaseYear([entry("a", 2015, 2023, 5), entry("b", 2020, 2023, 1), entry("c", 2020, 2024, 2), entry("d", 2020, 2024, 7)]);
    expect(groups.map((g) => g.baseYear)).toEqual([2020, 2015]);
    expect(groups[0]?.entries.map((e) => e.id)).toEqual(["d", "c", "b"]);
  });

  it("stores every fetched value and counts only the stored ones", async () => {
    let n = 0;
    const { calls } = mockFetch({
      "POST /api/vpi/fetch": json(FETCHED),
      "POST /api/vpi": () => (n++ === 0 ? json({}, 201) : json({ error: "x" }, 500)),
    });
    await expect(importFromBundesbank(2020)).resolves.toBe(1);
    expect(calls.filter((c) => c.url === "/api/vpi").map((c) => c.body)).toEqual([
      { baseYear: 2020, year: 2024, month: 1, value: 117.4 },
      { baseYear: 2020, year: 2024, month: 2, value: 118 },
    ]);
  });

  it("tells an expired session apart by type, not by its text", async () => {
    const { calls } = mockFetch({
      "POST /api/vpi/fetch": json(FETCHED),
      "POST /api/vpi": json({ error: SESSION_EXPIRED }, 400),
    });
    await expect(importFromBundesbank(2020)).resolves.toBe(0);
    expect(calls.filter((c) => c.url === "/api/vpi")).toHaveLength(2);
  });

  it("stops the import when the session expired", async () => {
    const { calls } = mockFetch({ "POST /api/vpi/fetch": json(FETCHED), "POST /api/vpi": loginPage() });
    await expect(importFromBundesbank(2020)).rejects.toThrow(SESSION_EXPIRED);
    expect(calls.filter((c) => c.url === "/api/vpi")).toHaveLength(1);
  });

  it("requires every field of a manual entry", () => {
    expect(Object.keys(vpiFormErrors({ baseYear: "", year: "x", month: "", value: "" }))).toEqual([
      "baseYear",
      "year",
      "month",
      "value",
    ]);
    expect(vpiFormErrors({ baseYear: "2020", year: "2024", month: "3", value: "117.4" })).toEqual({});
  });
});

describe("VpiEntryForm", () => {
  it("shows errors on submit without sending", async () => {
    const { calls } = mockFetch({});
    renderWithUi(<VpiEntryForm onSaved={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText("Bitte einen Monat wählen.")).toBeInTheDocument();
    expect(calls).toHaveLength(0);
  });
});

describe("VpiFetchPanel", () => {
  it("reports the number of imported values and reloads", async () => {
    mockFetch({ "POST /api/vpi/fetch": json(FETCHED), "POST /api/vpi": json({}, 201) });
    const onImported = vi.fn(async () => {});
    renderWithUi(<VpiFetchPanel onImported={onImported} />);
    fireEvent.click(screen.getByRole("button", { name: "Von Bundesbank laden" }));
    expect(await screen.findByText("2 VPI-Werte erfolgreich importiert")).toBeInTheDocument();
    expect(onImported).toHaveBeenCalledTimes(1);
  });

  it("shows the API error of the Bundesbank request", async () => {
    mockFetch({ "POST /api/vpi/fetch": json({ error: "Keine VPI-Daten gefunden. Bitte VPI-Werte manuell eintragen." }, 404) });
    renderWithUi(<VpiFetchPanel onImported={vi.fn(async () => {})} />);
    fireEvent.click(screen.getByRole("button", { name: "Von Bundesbank laden" }));
    expect(await screen.findByText(/Keine VPI-Daten gefunden/)).toBeInTheDocument();
  });
});
