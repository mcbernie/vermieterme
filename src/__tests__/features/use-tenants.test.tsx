import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { formToPayload, tenantToForm } from "@/features/tenants/tenant-form-values";
import { useTenants } from "@/features/tenants/use-tenants";
import { defaultAnswers, sentBodies, stubApi } from "./tenant-api-stub";
import { TENANT } from "./tenant-fixtures";

afterEach(() => vi.unstubAllGlobals());

/** Renders the hook and waits for the initial load. */
async function loaded() {
  const hook = renderHook(() => useTenants());
  await waitFor(() => expect(hook.result.current.loading).toBe(false));
  return hook;
}

describe("useTenants", () => {
  it("loads tenants, properties with units and rent changes", async () => {
    stubApi();
    const { result } = await loaded();
    expect(result.current.error).toBeNull();
    expect(result.current.tenants.map((t) => t.id)).toEqual(["t1"]);
    expect(result.current.properties[0]?.units).toHaveLength(2);
    expect(result.current.rentChanges).toHaveLength(4);
  });

  it("creates a tenant with POST and the complete body, then reloads the list", async () => {
    const { fetchMock, answers } = stubApi();
    const { result } = await loaded();
    const form = { ...tenantToForm(TENANT), iban: "", email: "", bankName: "", accountHolder: "" };
    answers["GET /api/tenants"] = { status: 200, body: [TENANT, { ...TENANT, id: "t2" }] };
    let error: string | null = "unset";
    await act(async () => {
      error = await result.current.saveTenant(form);
    });
    expect(error).toBeNull();
    const [body] = sentBodies(fetchMock, "POST", "/api/tenants");
    expect(body).toEqual({
      unitId: "u1",
      salutation: "Frau",
      firstName: "Erika",
      lastName: "Muster",
      salutation2: null,
      firstName2: null,
      lastName2: null,
      phone: null,
      email: null,
      bankName: null,
      iban: null,
      accountHolder: null,
      moveInDate: "2022-04-01",
      moveOutDate: null,
      leaseType: "index",
      indexBaseYear: 2020,
      indexReferenceValue: 105.4,
      indexReferenceDate: "2023-01-01",
      indexMinMonths: 12,
    });
    expect(result.current.tenants.map((t) => t.id)).toEqual(["t1", "t2"]);
  });

  it("updates a tenant with PUT", async () => {
    const { fetchMock } = stubApi();
    const { result } = await loaded();
    const form = { ...tenantToForm(TENANT), leaseType: "staffel" as const, moveOutDate: "2026-06-30" };
    await act(async () => {
      await result.current.saveTenant(form, "t1");
    });
    expect(sentBodies(fetchMock, "PUT", "/api/tenants/t1")).toEqual([formToPayload(form)]);
    expect(sentBodies(fetchMock, "PUT", "/api/tenants/t1")[0]).toMatchObject({
      leaseType: "staffel",
      moveOutDate: "2026-06-30",
    });
  });

  it("returns the server error of a failed save", async () => {
    const answers = defaultAnswers();
    answers["POST /api/tenants"] = { status: 400, body: { error: "Wohnung fehlt" } };
    stubApi(answers);
    const { result } = await loaded();
    let error: string | null = null;
    await act(async () => {
      error = await result.current.saveTenant(tenantToForm(TENANT));
    });
    expect(error).toBe("Wohnung fehlt");
  });

  it("records a rent change in euros and reloads the rent changes", async () => {
    const { fetchMock } = stubApi();
    const { result } = await loaded();
    const adjustment = { unitId: "u1", type: "rent" as const, amount: 650.5, effectiveDate: "2026-05-01", reason: null };
    await act(async () => {
      await result.current.saveAdjustment(adjustment);
    });
    expect(sentBodies(fetchMock, "POST", "/api/rent-changes")).toEqual([adjustment]);
    expect(fetchMock.mock.calls.filter(([url, init]) => url === "/api/rent-changes" && !init?.method)).toHaveLength(0);
    expect(fetchMock.mock.calls.filter(([url, init]) => url === "/api/rent-changes" && init?.method === "GET")).toHaveLength(2);
  });

  it("sets the page error when reloading after a successful change fails", async () => {
    const { answers } = stubApi();
    const { result } = await loaded();
    answers["GET /api/tenants"] = { status: 500, body: {} };
    let error: string | null = "unset";
    await act(async () => {
      error = await result.current.deleteTenant("t1");
    });
    expect(error).toBeNull();
    expect(result.current.error).toBe("Die Anfrage ist fehlgeschlagen (Status 500).");
  });

  it("keeps a newer list when a slow initial answer arrives late", async () => {
    const { fetchMock, answers } = stubApi();
    const plain = fetchMock.getMockImplementation();
    let releaseInitial: () => void = () => undefined;
    let first = true;
    // The first GET of the tenants waits until released; everything else answers right away.
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/tenants" && !init?.method?.match(/POST|PUT|DELETE/) && first) {
        first = false;
        const late = Response.json([TENANT]);
        return new Promise<Response>((resolve) => (releaseInitial = () => resolve(late)));
      }
      return plain!(url, init);
    });
    const { result } = renderHook(() => useTenants());
    answers["GET /api/tenants"] = { status: 200, body: [] };
    await act(async () => {
      await result.current.deleteTenant("t1");
    });
    expect(result.current.tenants).toEqual([]);
    await act(async () => releaseInitial());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.tenants).toEqual([]);
  });

  it("clears the page error once the failed list reloads successfully", async () => {
    const { answers } = stubApi();
    const { result } = await loaded();
    answers["GET /api/tenants"] = { status: 500, body: {} };
    await act(async () => {
      await result.current.deleteTenant("t1");
    });
    expect(result.current.error).not.toBeNull();
    answers["GET /api/tenants"] = { status: 200, body: [] };
    await act(async () => {
      await result.current.deleteTenant("t1");
    });
    expect(result.current.error).toBeNull();
  });

  it("drops the error of an outdated initial load when a newer reload of that list succeeded", async () => {
    const { fetchMock, answers } = stubApi();
    const plain = fetchMock.getMockImplementation();
    let releaseInitial: () => void = () => undefined;
    let first = true;
    // The first GET of the tenants fails late; the reload after the delete succeeds before it.
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/tenants" && !init?.method?.match(/POST|PUT|DELETE/) && first) {
        first = false;
        const late = Response.json({}, { status: 500 });
        return new Promise<Response>((resolve) => (releaseInitial = () => resolve(late)));
      }
      return plain!(url, init);
    });
    const { result } = renderHook(() => useTenants());
    answers["GET /api/tenants"] = { status: 200, body: [] };
    await act(async () => {
      await result.current.deleteTenant("t1");
    });
    await act(async () => releaseInitial());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBeNull();
  });

  it("keeps the error of another list when one list reloads successfully", async () => {
    const answers = defaultAnswers();
    answers["GET /api/properties"] = { status: 500, body: {} };
    stubApi(answers);
    const { result } = await loaded();
    expect(result.current.error).not.toBeNull();
    await act(async () => {
      await result.current.deleteTenant("t1");
    });
    expect(result.current.error).not.toBeNull();
    expect(result.current.tenantsError).toBeNull();
  });
});
