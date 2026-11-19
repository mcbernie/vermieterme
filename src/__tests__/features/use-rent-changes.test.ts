import { afterEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useRentChanges } from "@/features/rent-changes/use-rent-changes";

afterEach(() => vi.unstubAllGlobals());

const json = (data: unknown, ok = true) => ({ ok, json: async () => data }) as Response;

describe("useRentChanges", () => {
  it("starts the rent change and property requests together", async () => {
    const calls: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        calls.push(url);
        return new Promise<Response>(() => undefined);
      })
    );
    renderHook(() => useRentChanges());
    await waitFor(() => expect(calls).toContain("/api/properties"));
    expect(calls).toEqual(["/api/rent-changes", "/api/properties"]);
  });

  it("skips property details that failed to load", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url === "/api/rent-changes") return json([]);
        if (url === "/api/properties") return json([{ id: "p1" }, { id: "p2" }]);
        if (url === "/api/properties/p1") return json({ id: "p1", units: [] });
        return json({ error: "x" }, false);
      })
    );
    const { result } = renderHook(() => useRentChanges());
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.properties.map((p) => p.id)).toEqual(["p1"]);
  });
});
