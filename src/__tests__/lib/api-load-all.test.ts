import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiLoadAll } from "@/lib/api-client";
import { onSessionExpired } from "@/lib/session-expiry";
import { json, loginPage, mockFetch } from "../features/settings-helpers";

interface Data {
  properties: { id: string }[];
  landlord: { name: string } | null;
}

const urls = { properties: "/api/properties", landlord: "/api/settings" };

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("apiLoadAll", () => {
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));

  it("loads every endpoint under its key; JSON null is no failure", async () => {
    mockFetch({ "GET /api/properties": json([{ id: "p1" }]), "GET /api/settings": json(null) });
    await expect(apiLoadAll<Data>(urls)).resolves.toEqual({ data: { properties: [{ id: "p1" }] }, failed: false });
    expect(console.error).not.toHaveBeenCalled();
  });

  it("keeps the other keys and reports and logs the failure of one endpoint", async () => {
    mockFetch({ "GET /api/properties": json({ error: "x" }, 500), "GET /api/settings": json({ name: "A" }) });
    await expect(apiLoadAll<Data>(urls)).resolves.toEqual({ data: { landlord: { name: "A" } }, failed: true });
    expect(console.error).toHaveBeenCalledTimes(1);
  });

  it("reports an expired session to the frame", async () => {
    const expired = vi.fn();
    const off = onSessionExpired(expired);
    mockFetch({ "GET /api/properties": loginPage(), "GET /api/settings": loginPage() });
    await expect(apiLoadAll<Data>(urls)).resolves.toEqual({ data: {}, failed: true });
    off();
    expect(expired).toHaveBeenCalled();
  });

  it("passes the signal and does not log an aborted load", async () => {
    const controller = new AbortController();
    controller.abort();
    const { fetchMock } = mockFetch({});
    fetchMock.mockRejectedValue(new DOMException("aborted", "AbortError"));
    await expect(apiLoadAll<Data>(urls, controller.signal)).resolves.toMatchObject({ failed: true });
    expect(fetchMock.mock.calls[0]?.[1]?.signal).toBe(controller.signal);
    expect(console.error).not.toHaveBeenCalled();
  });
});
