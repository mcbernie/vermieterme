import { afterEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useVpiAdjustment } from "@/features/dashboard/use-vpi-adjustment";
import { onSessionExpired } from "@/lib/session-expiry";
import type { VpiSuggestion } from "@/types";
import { loginPage } from "./settings-helpers";

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
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("Bezugswert");
    });
    expect(refetch).toHaveBeenCalledTimes(1);
    expect(result.current.error).toContain("Bezugswert");
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

  it("does not count the login page as answer to the POST as created rent change", async () => {
    const expired = vi.fn();
    const off = onSessionExpired(expired);
    const fetchMock = vi.fn().mockResolvedValue(loginPage());
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useVpiAdjustment(vi.fn().mockResolvedValue(undefined)));
    await act(async () => {
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("nicht angelegt");
    });
    off();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(expired).toHaveBeenCalledTimes(1);
    expect(result.current.appliedIds.size).toBe(0);
  });

  it("does not count the login page as answer to the PUT as saved reference value", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const fetchMock = vi.fn().mockResolvedValueOnce(reply(true)).mockResolvedValueOnce(loginPage());
    vi.stubGlobal("fetch", fetchMock);
    const refetch = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useVpiAdjustment(refetch));
    await act(async () => {
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("Bezugswert");
    });
    expect(fetchMock.mock.calls.map((c) => c[1].method)).toEqual(["POST", "PUT"]);
    expect(result.current.appliedIds.has("t1")).toBe(false);
  });

  it("repeats only the PUT after a failed PUT, without a second POST", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(reply(true))
      .mockResolvedValueOnce(reply(false))
      .mockResolvedValueOnce(reply(true));
    vi.stubGlobal("fetch", fetchMock);
    const { result } = renderHook(() => useVpiAdjustment(vi.fn().mockResolvedValue(undefined)));
    await act(async () => {
      await expect(result.current.apply(suggestion, value)).rejects.toThrow("Bezugswert");
    });
    await act(() => result.current.apply(suggestion, value));
    expect(fetchMock.mock.calls.map((c) => c[1].method)).toEqual(["POST", "PUT", "PUT"]);
    expect(result.current.appliedIds.has("t1")).toBe(true);
    expect(result.current.error).toBeNull();
  });
});
