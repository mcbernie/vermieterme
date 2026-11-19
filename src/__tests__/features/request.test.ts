import { afterEach, describe, expect, it, vi } from "vitest";
import { isArray, requestJson, SESSION_EXPIRED, UNEXPECTED_ANSWER } from "@/features/tenants/request";

afterEach(() => vi.unstubAllGlobals());

/** Stubs `fetch` with one answer. */
function answer(res: Response) {
  const fetchMock = vi.fn().mockResolvedValue(res);
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("requestJson", () => {
  it("returns the parsed body on success and sends JSON", async () => {
    const fetchMock = answer(Response.json({ id: "x" }));
    expect(await requestJson("/api/x", { method: "POST", body: { a: 1 } })).toEqual({ ok: true, data: { id: "x" } });
    expect(fetchMock).toHaveBeenCalledWith("/api/x", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"a":1}',
    });
  });

  it("sends a GET without body by default", async () => {
    const fetchMock = answer(Response.json([]));
    await requestJson("/api/x");
    expect(fetchMock).toHaveBeenCalledWith("/api/x", { method: "GET" });
  });

  it("uses the server message of a client error", async () => {
    answer(Response.json({ error: "Mieter nicht gefunden" }, { status: 404 }));
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: "Mieter nicht gefunden" });
  });

  it("uses a generic German text for server errors", async () => {
    answer(Response.json({ error: "Internal Server Error" }, { status: 500 }));
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: "Die Anfrage ist fehlgeschlagen (Status 500)." });
  });

  it("treats an HTML page with status 200 as expired session", async () => {
    answer(new Response("<html><body>Anmelden</body></html>", { status: 200, headers: { "Content-Type": "text/html" } }));
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: SESSION_EXPIRED });
  });

  it("treats a redirected answer as expired session, even with JSON", async () => {
    const res = Response.json({ ok: true });
    Object.defineProperty(res, "redirected", { value: true });
    answer(res);
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: SESSION_EXPIRED });
  });

  it("treats an empty or unparsable JSON body as expired session", async () => {
    answer(new Response("", { status: 200, headers: { "Content-Type": "application/json" } }));
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: SESSION_EXPIRED });
    answer(new Response("{kaputt", { status: 200, headers: { "Content-Type": "application/json" } }));
    expect(await requestJson("/api/x")).toEqual({ ok: false, error: SESSION_EXPIRED });
  });

  it("rejects an answer of the wrong shape", async () => {
    answer(Response.json({ not: "a list" }));
    expect(await requestJson("/api/x", { accept: isArray })).toEqual({ ok: false, error: UNEXPECTED_ANSWER });
  });

  it("reports an unreachable server", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));
    const res = await requestJson("/api/x");
    expect(res.ok).toBe(false);
  });
});
