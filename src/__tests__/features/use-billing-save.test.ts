import { act, renderHook } from "@testing-library/react";
import { useEffect } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AUTOSAVE_DELAY, useBillingSave, type SaveValues } from "@/features/billing/use-billing-save";

const json = (ok = true) => ({ ok, json: async () => ({}) }) as Response;

const values: SaveValues = {
  costs: {
    c1: { totalAmount: 100, unitAmount: null, enabled: true, distributionKeyOverride: null },
    c2: { totalAmount: null, unitAmount: 5, enabled: true, distributionKeyOverride: null },
  },
  prepayments: { u1: 50, u2: null },
};

/** Parsed bodies of all POST calls. */
function bodies(fetchMock: ReturnType<typeof vi.fn>) {
  return fetchMock.mock.calls.map((c) => [c[0], JSON.parse((c[1] as RequestInit).body as string)]);
}

describe("useBillingSave", () => {
  let fetchMock: ReturnType<typeof vi.fn>;
  beforeEach(() => {
    vi.useFakeTimers();
    fetchMock = vi.fn(async () => json());
    vi.stubGlobal("fetch", fetchMock);
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("sends only filled amounts and reports success", async () => {
    const { result } = renderHook(() => useBillingSave("bp", values));
    await act(async () => result.current.save());
    expect(bodies(fetchMock)).toEqual([
      ["/api/billing-periods/bp/costs", { costCategoryId: "c1", totalAmount: 100, unitAmount: null }],
      ["/api/billing-periods/bp/prepayments", { unitId: "u1", monthlyAmount: 50 }],
    ]);
    expect(result.current.state).toBe("saved");
  });

  it("saves automatically once after the last change", async () => {
    const { result } = renderHook(() => useBillingSave("bp", values));
    act(() => {
      result.current.schedule();
      result.current.schedule();
    });
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY - 1));
    expect(fetchMock).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(1));
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("never runs two saves at once and repeats once with the latest values", async () => {
    const pending: ((r: Response) => void)[] = [];
    fetchMock.mockImplementation(() => new Promise<Response>((r) => pending.push(r)));
    const { result } = renderHook(() => useBillingSave("bp", values));
    let first: Promise<void> = Promise.resolve();
    act(() => {
      first = result.current.save();
    });
    await act(async () => {
      void result.current.save();
      void result.current.save();
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    fetchMock.mockImplementation(async () => json());
    await act(async () => {
      pending.forEach((r) => r(json()));
      await first;
    });
    expect(fetchMock).toHaveBeenCalledTimes(4);
    expect(result.current).toMatchObject({ pending: false, saving: false, state: "saved" });
  });

  it("shows no unsaved changes after an automatic save that ran during a slow save", async () => {
    const pending: ((r: Response) => void)[] = [];
    fetchMock.mockImplementation(() => new Promise<Response>((r) => pending.push(r)));
    const { result } = renderHook(() => useBillingSave("bp", values));
    let first: Promise<void> = Promise.resolve();
    act(() => {
      first = result.current.save();
    });
    act(() => result.current.schedule());
    expect(result.current.pending).toBe(true);
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY));
    fetchMock.mockImplementation(async () => json());
    await act(async () => {
      pending.forEach((r) => r(json()));
      await first;
    });
    expect(fetchMock).toHaveBeenCalledTimes(4);
    expect(result.current).toMatchObject({ pending: false, saving: false, state: "saved" });
  });

  it("saves nothing when the page is left with an unreadable amount", async () => {
    const { unmount } = renderHook(() => {
      const saver = useBillingSave("bp", values);
      const { reportProblem } = saver;
      // Like an amount field: reports its problem and withdraws it on unmount, after the hook cleanup.
      useEffect(() => {
        reportProblem("cost-total-c1", true);
        return () => reportProblem("cost-total-c1", false);
      }, [reportProblem]);
      return saver;
    });
    unmount();
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY));
    expect(fetchMock).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("shows an error when a request fails", async () => {
    fetchMock.mockResolvedValueOnce(json(false));
    const { result } = renderHook(() => useBillingSave("bp", values));
    await act(async () => result.current.save());
    expect(result.current.state).toBe("error");
  });

  it("refuses to save while an amount is unreadable and saves after it is fixed", async () => {
    const { result } = renderHook(() => useBillingSave("bp", values));
    act(() => result.current.reportProblem("cost-total-c1", true));
    await act(async () => result.current.save());
    expect(fetchMock).not.toHaveBeenCalled();
    expect(result.current.state).toBe("invalid");
    act(() => result.current.reportProblem("cost-total-c1", false));
    expect(result.current.state).toBeNull();
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY));
    expect(fetchMock).toHaveBeenCalled();
  });

  it("still sends a pending automatic save when unmounting", () => {
    const { result, unmount } = renderHook(() => useBillingSave("bp", values));
    act(() => result.current.schedule());
    unmount();
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  describe("leaving the page while a save runs", () => {
    const one = (amount: number): SaveValues => ({
      costs: { c1: { totalAmount: amount, unitAmount: null, enabled: true, distributionKeyOverride: null } },
      prepayments: {},
    });

    /** Holds every request open; returns the resolvers in call order. */
    function hold() {
      const pending: ((r: Response) => void)[] = [];
      fetchMock.mockImplementation(() => new Promise<Response>((r) => pending.push(r)));
      return pending;
    }

    const amounts = () => bodies(fetchMock).map(([, b]) => (b as { totalAmount: number }).totalAmount);

    for (const trigger of ["save", "schedule"] as const) {
      it(`sends the latest values once after the running save (${trigger}, then unmount)`, async () => {
        const pending = hold();
        const { result, rerender, unmount } = renderHook(({ v }) => useBillingSave("bp", v), {
          initialProps: { v: one(1) },
        });
        act(() => void result.current.save());
        rerender({ v: one(2) });
        act(() => void result.current[trigger]());
        unmount();
        expect(fetchMock).toHaveBeenCalledTimes(1);
        await act(async () => pending[0]?.(json()));
        expect(amounts()).toEqual([1, 2]);
        await act(async () => pending[1]?.(json()));
        expect(fetchMock).toHaveBeenCalledTimes(2);
      });
    }
  });
  it("does not resend an intermediate value once the field became unreadable", async () => {
    const one = (amount: number): SaveValues => ({
      costs: { c1: { totalAmount: amount, unitAmount: null, enabled: true, distributionKeyOverride: null } },
      prepayments: {},
    });
    const open: ((r: Response) => void)[] = [];
    fetchMock.mockImplementation(() => new Promise<Response>((r) => open.push(r)));
    const { result, rerender } = renderHook(({ v }) => useBillingSave("bp", v), { initialProps: { v: one(1) } });
    let first: Promise<void> = Promise.resolve();
    act(() => {
      first = result.current.save();
    });
    rerender({ v: one(12) }); // readable "12" while the first save runs
    act(() => result.current.schedule());
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY)); // requests a repeat
    act(() => result.current.reportProblem("cost-total-c1", true)); // typed on to "12x"
    fetchMock.mockImplementation(async () => json());
    await act(async () => {
      open.forEach((r) => r(json()));
      await first;
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(result.current.state).toBe("invalid");
    expect(result.current.pending).toBe(true);
  });
});
