import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useBillingList } from "@/features/billing/use-billing-list";
import type { PreviousYearSuggestion } from "@/features/billing/billing-list-utils";

const json = (data: unknown, ok = true) => ({ ok, status: ok ? 200 : 500, json: async () => data }) as Response;
const prop = { id: "p1", street: "Weg 1", zip: "1", city: "C", totalShares: 100 };
const period = (id: string) => ({ id, propertyId: "p1", startDate: "2024-01-01", endDate: "2024-12-31", property: prop });

const suggestion = {
  property: prop,
  previousYear: 2025,
  sourcePeriod: period("bp1"),
} as unknown as PreviousYearSuggestion;

describe("useBillingList", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("aborts the initial load on unmount", () => {
    const signals: AbortSignal[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((_url: string, init?: RequestInit) => {
        if (init?.signal) signals.push(init.signal);
        return new Promise<Response>(() => {});
      }),
    );
    const { unmount } = renderHook(() => useBillingList());
    unmount();
    expect(signals.length).toBe(2);
    expect(signals.every((s) => s.aborted)).toBe(true);
  });

  it("reports a failed load", async () => {
    vi.stubGlobal("fetch", vi.fn(async (url: string) => (url === "/api/properties" ? json([], false) : json([]))));
    const { result } = renderHook(() => useBillingList());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.loadFailed).toBe(true);
  });

  it("drops an outdated load that answers after a newer one", async () => {
    const pending: ((r: Response) => void)[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        if (url === "/api/properties") return Promise.resolve(json([prop]));
        return new Promise<Response>((r) => pending.push(r));
      }),
    );
    const { result } = renderHook(() => useBillingList());
    let second: Promise<void> = Promise.resolve();
    act(() => {
      second = result.current.reload();
    });
    await act(async () => {
      pending[1]?.(json([period("new")]));
      await second;
    });
    await act(async () => pending[0]?.(json([period("old")])));
    expect(result.current.periods.map((p) => p.id)).toEqual(["new"]);
  });

  it("creates a suggested period only once on a double click", async () => {
    let resolvePost: (r: Response) => void = () => {};
    const fetchMock = vi.fn((url: string, init?: RequestInit) => {
      if (init?.method === "POST") return new Promise<Response>((r) => (resolvePost = r));
      return Promise.resolve(json(url === "/api/properties" ? [prop] : [period("bp1")]));
    });
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useBillingList());
    await waitFor(() => expect(result.current.loading).toBe(false));
    let first: Promise<void> = Promise.resolve();
    act(() => {
      first = result.current.quickCreate(suggestion);
    });
    await act(async () => result.current.quickCreate(suggestion));
    expect(result.current.creatingFor).toBe("p1");
    await act(async () => {
      resolvePost(json({}));
      await first;
    });
    const posts = fetchMock.mock.calls.filter((c) => c[1]?.method === "POST");
    expect(posts).toHaveLength(1);
    expect(JSON.parse(posts[0]?.[1]?.body as string)).toEqual({
      propertyId: "p1",
      startDate: "2025-01-01",
      endDate: "2025-12-31",
      billingDate: null,
      copyFromId: "bp1",
    });
    expect(result.current.creatingFor).toBeNull();
  });

  it("shows the server message when deleting fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_url: string, init?: RequestInit) =>
        // Server texts are shown for client errors only; 5xx answers carry an English generic text.
        init?.method === "DELETE"
          ? ({ ok: false, status: 404, json: async () => ({ error: "Abrechnung nicht gefunden" }) } as Response)
          : json([]),
      ),
    );
    const { result } = renderHook(() => useBillingList());
    await waitFor(() => expect(result.current.loading).toBe(false));
    await act(async () => result.current.remove("bp1"));
    expect(result.current.actionError).toBe("Abrechnung nicht gefunden");
  });

  it("starts no new load after unmount", async () => {
    const fetchMock = vi.fn(async () => json([]));
    vi.stubGlobal("fetch", fetchMock);
    const { result, unmount } = renderHook(() => useBillingList());
    await waitFor(() => expect(result.current.loading).toBe(false));
    const calls = fetchMock.mock.calls.length;
    unmount();
    await result.current.reload();
    expect(fetchMock.mock.calls.length).toBe(calls);
  });
});
