import { afterEach, describe, expect, it, vi } from "vitest";
import { requestJson } from "@/features/tenants/request";

afterEach(() => vi.unstubAllGlobals());

/** Stubs `fetch` with one answer. */
function answer(status: number, body: unknown) {
  const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status }));
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("requestJson", () => {
  it("returns the parsed body on success and sends JSON", async () => {
    const fetchMock = answer(200, { id: "x" });
    expect(await requestJson("/api/x", "POST", { a: 1 })).toEqual({ ok: true, data: { id: "x" } });
    expect(fetchMock).toHaveBeenCalledWith("/api/x", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"a":1}',
    });
  });

  it("uses the server message of a client error", async () => {
    answer(404, { error: "Mieter nicht gefunden" });
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: "Mieter nicht gefunden" });
  });

  it("uses a generic German text for server errors", async () => {
    answer(500, { error: "Internal Server Error" });
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: "Die Anfrage ist fehlgeschlagen (Status 500)." });
  });

  it("reports an unreachable server", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));
    const res = await requestJson("/api/x");
    expect(res.ok).toBe(false);
  });
});
