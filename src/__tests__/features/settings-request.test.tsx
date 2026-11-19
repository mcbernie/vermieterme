import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ensureOk, errorText, RequestError, requestJson, SESSION_EXPIRED } from "@/features/settings/request";
import { useAction } from "@/features/settings/use-action";
import { LOAD_FAILED, REFRESH_FAILED, useLoad } from "@/features/settings/use-load";
import { fakeResponse, json, loginPage, mockFetch } from "./settings-helpers";

afterEach(() => vi.unstubAllGlobals());

describe("requestJson", () => {
  it("returns the parsed body and sends JSON with the right header", async () => {
    const { fetchMock } = mockFetch({ "POST /api/x": json({ id: "1" }) });
    await expect(requestJson("/api/x", "fehlt", { method: "POST", json: { a: 1 } })).resolves.toEqual({ id: "1" });
    const init = fetchMock.mock.calls[0]?.[1];
    expect(init?.headers).toEqual({ "Content-Type": "application/json" });
    expect(init?.body).toBe('{"a":1}');
  });

  it("treats a redirect to the login page as an expired session", async () => {
    mockFetch({ "GET /api/x": loginPage() });
    await expect(requestJson("/api/x", "fehlt")).rejects.toThrow(SESSION_EXPIRED);
  });

  it("treats a 2xx answer without JSON as an expired session", async () => {
    mockFetch({ "GET /api/x": fakeResponse("<html></html>", { contentType: "text/html" }) });
    await expect(requestJson("/api/x", "fehlt")).rejects.toThrow(SESSION_EXPIRED);
    mockFetch({ "GET /api/x": fakeResponse("kaputt", { contentType: "application/json" }) });
    await expect(requestJson("/api/x", "fehlt")).rejects.toThrow(SESSION_EXPIRED);
  });

  it("uses the API error text, otherwise the fallback", async () => {
    mockFetch({ "GET /api/a": json({ error: "Gibt es schon" }, 409), "GET /api/b": fakeResponse("x", { status: 500, contentType: "text/plain" }) });
    await expect(requestJson("/api/a", "fehlt")).rejects.toThrow("Gibt es schon");
    await expect(requestJson("/api/b", "Fehler beim Speichern")).rejects.toThrow("Fehler beim Speichern");
  });

  it("turns a network error into the fallback", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => Promise.reject(new TypeError("offline"))));
    await expect(requestJson("/api/x", "Fehler beim Laden")).rejects.toThrow("Fehler beim Laden");
  });

  it("lets ensureOk accept a JSON download and reject the login page", async () => {
    await expect(ensureOk(json({}), "x")).resolves.toBeUndefined();
    await expect(ensureOk(loginPage(), "x")).rejects.toBeInstanceOf(RequestError);
  });

  it("detects the login page by the redirect and by the HTML type separately", async () => {
    await expect(ensureOk(fakeResponse({}, { redirected: true }), "x")).rejects.toThrow(SESSION_EXPIRED);
    await expect(ensureOk(fakeResponse("<html></html>", { contentType: "text/html" }), "x")).rejects.toThrow(
      SESSION_EXPIRED,
    );
  });

  it("shows only messages of request errors", () => {
    expect(errorText(new RequestError("Klartext"), "Fallback")).toBe("Klartext");
    expect(errorText(new Error("intern"), "Fallback")).toBe("Fallback");
  });
});

describe("useAction", () => {
  it("runs once while busy and reports the error text", async () => {
    let release: () => void = () => {};
    const action = vi.fn(() => new Promise<void>((resolve) => (release = resolve)));
    const { result } = renderHook(() => useAction("Fehler"));
    let first: Promise<boolean> = Promise.resolve(false);
    let second: Promise<boolean> = Promise.resolve(false);
    act(() => {
      first = result.current.run(action);
      second = result.current.run(action);
    });
    expect(action).toHaveBeenCalledTimes(1);
    await act(async () => {
      release();
      await first;
    });
    await expect(second).resolves.toBe(false);
    await act(async () => {
      await result.current.run(async () => {
        throw new RequestError(SESSION_EXPIRED);
      });
    });
    expect(result.current.error).toBe(SESSION_EXPIRED);
    expect(result.current.busy).toBe(false);
  });
});

describe("useLoad", () => {
  it("aborts the request on unmount", async () => {
    let signal: AbortSignal | undefined;
    const load = vi.fn((s: AbortSignal) => {
      signal = s;
      return new Promise<string>(() => {});
    });
    const { unmount } = renderHook(() => useLoad(load));
    expect(signal?.aborted).toBe(false);
    unmount();
    expect(signal?.aborted).toBe(true);
  });

  it("drops the answer of an older request", async () => {
    const resolvers: ((value: string) => void)[] = [];
    const load = vi.fn(() => new Promise<string>((resolve) => resolvers.push(resolve)));
    const { result } = renderHook(() => useLoad(load));
    act(() => void result.current.refresh());
    await act(async () => resolvers[1]?.("neu"));
    await act(async () => resolvers[0]?.("alt"));
    expect(result.current.data).toBe("neu");
  });

  it("shows a load error and keeps data when only a refresh fails", async () => {
    const load = vi.fn().mockRejectedValueOnce(new Error("x")).mockResolvedValueOnce("da").mockRejectedValueOnce(new Error("y"));
    const { result } = renderHook(() => useLoad(load));
    await waitFor(() => expect(result.current.error).toBe(LOAD_FAILED));
    act(() => result.current.retry());
    await waitFor(() => expect(result.current.data).toBe("da"));
    await act(async () => result.current.refresh());
    expect(result.current.data).toBe("da");
    expect(result.current.error).toBeNull();
    expect(result.current.refreshError).toBe(REFRESH_FAILED);
  });
});
