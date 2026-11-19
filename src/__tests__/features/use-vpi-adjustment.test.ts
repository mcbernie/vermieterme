import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useVpiAdjustment } from "@/features/dashboard/use-vpi-adjustment";
import type { VpiSuggestion } from "@/types";

const suggestion = {
  tenantId: "t1",
  unitId: "u1",
  referenceVpi: 100,
  currentVpi: 103,
  percentageChange: 3,
} as unknown as VpiSuggestion;
const value = { amount: 530, effectiveDate: "2026-02-01", reason: null };

const reply = (ok: boolean) => ({ ok }) as Response;

afterEach(() => vi.unstubAllGlobals());

describe("useVpiAdjustment.apply", () => {
  it("creates the rent change, updates the tenant and marks it applied", async () => {
    const fetchMock = vi.fn().mockResolvedValue(reply(true));
    vi.stubGlobal("fetch", fetchMock);
    const refetch = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useVpiAdjustment(refetch));
    await act(() => result.current.apply(suggestion, value));
    expect(fetchMock.mock.calls.map((c) => c[1].method)).toEqual(["POST", "PUT"]);
    expect(JSON.parse(fetchMock.mock.calls[0][1].body).reason).toBe("VPI-Anpassung (100 → 103, +3.00%)");
    expect(result.current.appliedIds.has("t1")).toBe(true);
    expect(result.current.error).toBeNull();
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("throws and reports an error when the POST fails, without a PUT", async () => {
    const fetchMock = vi.fn().mockResolvedValue(reply(false));
    vi.stubGlobal("fetch", fetchMock);
    const refetch = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useVpiAdjustment(refetch));
    await act(async () => {
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("nicht angelegt");
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(result.current.error).toContain("nicht angelegt");
    expect(result.current.appliedIds.size).toBe(0);
  });

  it("throws, refetches and does not mark the tenant when the PUT fails", async () => {
    const fetchMock = vi.fn().mockResolvedValueOnce(reply(true)).mockResolvedValueOnce(reply(false));
    vi.stubGlobal("fetch", fetchMock);
    const refetch = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useVpiAdjustment(refetch));
    await act(async () => {
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("Referenz-VPI");
    });
    expect(refetch).toHaveBeenCalledTimes(1);
    expect(result.current.error).toContain("Referenz-VPI");
    expect(result.current.appliedIds.has("t1")).toBe(false);
  });

  it("reports a network error", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
    const { result } = renderHook(() => useVpiAdjustment(vi.fn()));
    await act(async () => {
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("nicht durchgeführt");
    });
    expect(result.current.error).toContain("nicht durchgeführt");
  });
});
