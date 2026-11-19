import { afterEach, describe, expect, it, vi } from "vitest";
import {
  apiFetch,
  apiJson,
  apiLoad,
  apiResult,
  apiSend,
  errorText,
  logRequestFailure,
  RequestError,
  SESSION_EXPIRED,
  SessionExpiredError,
  UNEXPECTED_ANSWER,
  UNREACHABLE,
} from "@/lib/api-client";
import { onSessionExpired } from "@/lib/session-expiry";
import { fakeResponse, json, loginPage } from "../features/settings-helpers";

afterEach(() => vi.unstubAllGlobals());

/** Stubs `fetch` with one answer and returns the mock. */
function answer(res: Response) {
  const fetchMock = vi.fn().mockResolvedValue(res);
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

/** Registers a session listener for one test; returns the spy and the cleanup. */
function sessionSpy() {
  const spy = vi.fn();
  const off = onSessionExpired(spy);
  return { spy, off };
}

describe("apiJson: request", () => {
  it("sends GET without body and JSON with the right header", async () => {
    const get = answer(json([]));
    await apiJson("/api/x");
    expect(get).toHaveBeenCalledWith("/api/x", { method: "GET" });
    const post = answer(json({ id: "1" }, 201));
    await expect(apiJson("/api/x", { method: "POST", json: { a: 1 } })).resolves.toEqual({ id: "1" });
    expect(post).toHaveBeenCalledWith("/api/x", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: '{"a":1}',
    });
  });

  it("sends form data without own header and passes the signal", async () => {
    const fetchMock = answer(json({}));
    const form = new FormData();
    const signal = new AbortController().signal;
    await apiJson("/api/x", { method: "POST", form, signal });
    expect(fetchMock).toHaveBeenCalledWith("/api/x", { method: "POST", body: form, signal });
  });
});

describe("apiJson: failures", () => {
  it("uses the server text of a client error, typed as status error", async () => {
    answer(json({ error: "Gibt es schon" }, 409));
    const error = await apiJson("/api/x", { fallback: "Fehler" }).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(RequestError);
    expect(error).toMatchObject({ message: "Gibt es schon", kind: "status", status: 409 });
  });

  it("ignores the English text of a server error", async () => {
    answer(json({ error: "Internal Server Error" }, 500));
    await expect(apiJson("/api/x")).rejects.toThrow("Die Anfrage ist fehlgeschlagen (Status 500).");
    answer(json({ error: "Internal Server Error" }, 500));
    await expect(apiJson("/api/x", { fallback: "Fehler beim Speichern" })).rejects.toThrow("Fehler beim Speichern");
  });

  it("uses the fallback when an error body is no JSON or has no text", async () => {
    answer(fakeResponse("kaputt", { status: 400, contentType: "text/plain" }));
    await expect(apiJson("/api/x", { fallback: "F" })).rejects.toThrow("F");
    answer(json({ error: "" }, 404));
    await expect(apiJson("/api/x", { fallback: "F" })).rejects.toThrow("F");
  });

  it("turns a network error into the fallback or the generic text", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));
    await expect(apiJson("/api/x", { fallback: "Fehler beim Laden" })).rejects.toMatchObject({
      message: "Fehler beim Laden",
      kind: "network",
      status: 0,
    });
    await expect(apiJson("/api/x")).rejects.toThrow(UNREACHABLE);
  });

  it("rethrows the abort error of an aborted request", async () => {
    const controller = new AbortController();
    controller.abort();
    const abort = new DOMException("aborted", "AbortError");
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(abort));
    await expect(apiJson("/api/x", { signal: controller.signal })).rejects.toBe(abort);
  });

  it("rethrows an abort while reading the error text instead of a status error", async () => {
    const controller = new AbortController();
    const abort = new DOMException("aborted", "AbortError");
    const res = fakeResponse({ error: "x" }, { status: 400 });
    res.json = async () => {
      controller.abort();
      throw abort;
    };
    answer(res);
    await expect(apiJson("/api/x", { signal: controller.signal })).rejects.toBe(abort);
  });

  it("rethrows an abort while reading the body, without reporting an expired session", async () => {
    const { spy, off } = sessionSpy();
    const controller = new AbortController();
    const abort = new DOMException("aborted", "AbortError");
    const res = fakeResponse({ id: "1" });
    res.json = async () => {
      controller.abort();
      throw abort;
    };
    answer(res);
    await expect(apiJson("/api/x", { signal: controller.signal })).rejects.toBe(abort);
    expect(spy).not.toHaveBeenCalled();
    off();
  });

  it("rejects an answer of the wrong shape", async () => {
    answer(json({ not: "a list" }));
    await expect(apiJson("/api/x", { accept: Array.isArray as (d: unknown) => d is unknown[] })).rejects.toMatchObject({
      message: UNEXPECTED_ANSWER,
      kind: "unexpected",
    });
  });
});

describe("apiJson: answers without data", () => {
  it("returns JSON null as valid answer", async () => {
    answer(json(null));
    await expect(apiJson("/api/settings")).resolves.toBeNull();
  });

  it("returns null for 204 No Content", async () => {
    answer(fakeResponse("", { status: 204, contentType: "" }));
    await expect(apiJson("/api/x", { method: "DELETE" })).resolves.toBeNull();
  });

  it("treats an empty or unparsable JSON body of a 2xx answer as expired session", async () => {
    answer(fakeResponse("", { contentType: "application/json" }));
    await expect(apiJson("/api/x")).rejects.toBeInstanceOf(SessionExpiredError);
    answer(fakeResponse("{kaputt", { contentType: "application/json" }));
    await expect(apiJson("/api/x")).rejects.toThrow(SESSION_EXPIRED);
  });
});

describe("session expiry", () => {
  it.each([
    ["the login redirect", loginPage()],
    ["a redirected JSON answer", fakeResponse({ ok: true }, { redirected: true })],
    ["the HTML login page with status 200", fakeResponse("<html></html>", { contentType: "text/html" })],
    ["status 401", json({ error: "Nicht angemeldet" }, 401)],
    ["a 2xx answer that is not JSON", fakeResponse("x", { contentType: "text/plain" })],
  ])("detects %s and notifies the frame", async (_name, res) => {
    const { spy, off } = sessionSpy();
    answer(res);
    const error = await apiJson("/api/x", { fallback: "F" }).catch((e: unknown) => e);
    off();
    expect(error).toBeInstanceOf(SessionExpiredError);
    expect(error).toMatchObject({ message: SESSION_EXPIRED, kind: "session" });
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("does not treat a client error with the same text as expired session", async () => {
    const { spy, off } = sessionSpy();
    answer(json({ error: SESSION_EXPIRED }, 400));
    await expect(apiJson("/api/x")).rejects.not.toBeInstanceOf(SessionExpiredError);
    off();
    expect(spy).not.toHaveBeenCalled();
  });

  it("stops notifying a removed listener", async () => {
    const { spy, off } = sessionSpy();
    off();
    answer(loginPage());
    await expect(apiJson("/api/x")).rejects.toBeInstanceOf(SessionExpiredError);
    expect(spy).not.toHaveBeenCalled();
  });
});

describe("apiFetch", () => {
  it("returns the unread answer of a download and rejects the login page", async () => {
    answer(fakeResponse("{}", { headers: { "Content-Disposition": 'attachment; filename="a.json"' } }));
    const res = await apiFetch("/api/backup");
    expect(res.headers.get("Content-Disposition")).toContain("a.json");
    answer(loginPage());
    await expect(apiFetch("/api/backup")).rejects.toBeInstanceOf(SessionExpiredError);
  });
});

describe("apiResult", () => {
  it("returns the data or the error text as value", async () => {
    answer(json({ id: "x" }));
    await expect(apiResult("/api/x")).resolves.toEqual({ ok: true, data: { id: "x" } });
    answer(json({ error: "Mieter nicht gefunden" }, 404));
    await expect(apiResult("/api/x")).resolves.toEqual({ ok: false, error: "Mieter nicht gefunden" });
  });

  it("reports JSON null and 204 as unexpected answer", async () => {
    answer(json(null));
    await expect(apiResult("/api/x")).resolves.toEqual({ ok: false, error: UNEXPECTED_ANSWER });
    answer(fakeResponse("", { status: 204, contentType: "" }));
    await expect(apiResult("/api/x")).resolves.toEqual({ ok: false, error: UNEXPECTED_ANSWER });
  });

  it("reports an expired session and an unreachable server", async () => {
    answer(loginPage());
    await expect(apiResult("/api/x")).resolves.toEqual({ ok: false, error: SESSION_EXPIRED });
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("offline")));
    await expect(apiResult("/api/x")).resolves.toEqual({ ok: false, error: UNREACHABLE });
  });
});

describe("apiSend and apiLoad", () => {
  it("report success as boolean and data, failures as false and null with a log entry", async () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    answer(json({ id: "1" }, 201));
    await expect(apiSend("/api/x", { method: "POST", json: {} })).resolves.toBe(true);
    answer(json({ id: "1" }));
    await expect(apiLoad("/api/x")).resolves.toEqual({ id: "1" });
    answer(json({ error: "x" }, 400));
    await expect(apiSend("/api/x", { method: "DELETE" })).resolves.toBe(false);
    answer(loginPage());
    await expect(apiLoad("/api/x")).resolves.toBeNull();
    expect(log).toHaveBeenCalledTimes(2);
    log.mockRestore();
  });

  it("does not log an aborted load", async () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    const controller = new AbortController();
    controller.abort();
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new DOMException("aborted", "AbortError")));
    await expect(apiLoad("/api/x", { signal: controller.signal })).resolves.toBeNull();
    expect(log).not.toHaveBeenCalled();
    log.mockRestore();
  });
});

describe("errorText", () => {
  it("shows only messages of request errors", () => {
    expect(errorText(new RequestError("Klartext"), "Fallback")).toBe("Klartext");
    expect(errorText(new Error("intern"), "Fallback")).toBe("Fallback");
  });
});

describe("RequestError", () => {
  it("is of kind unexpected without status unless the caller names an HTTP answer", () => {
    expect(new RequestError("Klartext")).toMatchObject({ kind: "unexpected", status: 0 });
    expect(new RequestError("Klartext", "status", 409)).toMatchObject({ kind: "status", status: 409 });
  });
});

describe("logRequestFailure", () => {
  it("logs what was requested with the text of a request error, other errors unchanged", () => {
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    logRequestFailure("Fetch /api/x", new RequestError("Klartext", "status", 400));
    const other = new Error("intern");
    logRequestFailure("Request POST /api/x", other);
    expect(log.mock.calls).toEqual([
      ["Fetch /api/x failed:", "Klartext"],
      ["Request POST /api/x failed:", other],
    ]);
    log.mockRestore();
  });
});
