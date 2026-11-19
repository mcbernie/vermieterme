import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  apiFetch,
  apiJson,
  errorText,
  RequestError,
  SESSION_EXPIRED,
  SessionExpiredError,
} from "@/lib/api-client";
import { useAction } from "@/features/settings/use-action";
import { LOAD_FAILED, REFRESH_FAILED, useLoad } from "@/features/settings/use-load";
import { fakeResponse, json, loginPage, mockFetch } from "./settings-helpers";

afterEach(() => vi.unstubAllGlobals());

describe("apiJson", () => {
  it("returns the parsed body and sends JSON with the right header", async () => {
    const { fetchMock } = mockFetch({ "POST /api/x": json({ id: "1" }) });
    await expect(apiJson("/api/x", { fallback: "fehlt", method: "POST", json: { a: 1 } })).resolves.toEqual({ id: "1" });
    const init = fetchMock.mock.calls[0]?.[1];
    expect(init?.headers).toEqual({ "Content-Type": "application/json" });
    expect(init?.body).toBe('{"a":1}');
  });

  it("treats a redirect to the login page as an expired session, marked by type", async () => {
    mockFetch({ "GET /api/x": loginPage() });
    await expect(apiJson("/api/x", { fallback: "fehlt" })).rejects.toThrow(SESSION_EXPIRED);
    await expect(apiJson("/api/x", { fallback: "fehlt" })).rejects.toBeInstanceOf(SessionExpiredError);
    mockFetch({ "GET /api/x": json({ error: SESSION_EXPIRED }, 400) });
    await expect(apiJson("/api/x", { fallback: "fehlt" })).rejects.not.toBeInstanceOf(SessionExpiredError);
  });

  it("treats a 2xx answer without JSON as an expired session", async () => {
    mockFetch({ "GET /api/x": fakeResponse("<html></html>", { contentType: "text/html" }) });
    await expect(apiJson("/api/x", { fallback: "fehlt" })).rejects.toThrow(SESSION_EXPIRED);
    mockFetch({ "GET /api/x": fakeResponse("kaputt", { contentType: "application/json" }) });
    await expect(apiJson("/api/x", { fallback: "fehlt" })).rejects.toThrow(SESSION_EXPIRED);
  });

  it("uses the API error text, otherwise the fallback", async () => {
    mockFetch({ "GET /api/a": json({ error: "Gibt es schon" }, 409), "GET /api/b": fakeResponse("x", { status: 500, contentType: "text/plain" }) });
    await expect(apiJson("/api/a", { fallback: "fehlt" })).rejects.toThrow("Gibt es schon");
    await expect(apiJson("/api/b", { fallback: "Fehler beim Speichern" })).rejects.toThrow("Fehler beim Speichern");
  });

  it("turns a network error into the fallback", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => Promise.reject(new TypeError("offline"))));
    await expect(apiJson("/api/x", { fallback: "Fehler beim Laden" })).rejects.toThrow("Fehler beim Laden");
  });

  it("lets apiFetch accept a JSON download and reject the login page", async () => {
    mockFetch({ "GET /api/a": json({}), "GET /api/b": loginPage() });
    await expect(apiFetch("/api/a", { fallback: "x" })).resolves.toBeDefined();
    await expect(apiFetch("/api/b", { fallback: "x" })).rejects.toBeInstanceOf(RequestError);
  });

  it("detects the login page by the redirect and by the HTML type separately", async () => {
    mockFetch({
      "GET /api/a": fakeResponse({}, { redirected: true }),
      "GET /api/b": fakeResponse("<html></html>", { contentType: "text/html" }),
    });
    await expect(apiFetch("/api/a", { fallback: "x" })).rejects.toThrow(SESSION_EXPIRED);
    await expect(apiFetch("/api/b", { fallback: "x" })).rejects.toThrow(SESSION_EXPIRED);
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

  it("drops a late answer of an older request even when the loader ignores the signal", async () => {
    // The loader never looks at the signal, so only the hook's own check can drop the old answer.
    const resolvers: ((value: string) => void)[] = [];
    const load = vi.fn(() => new Promise<string>((resolve) => resolvers.push(resolve)));
    const { result } = renderHook(() => useLoad(load));
    act(() => void result.current.refresh());
    await act(async () => resolvers[1]?.("neu"));
    await act(async () => resolvers[0]?.("alt"));
    expect(result.current.data).toBe("neu");

    act(() => result.current.retry());
    act(() => result.current.retry());
    await act(async () => resolvers[3]?.("neuer"));
    await act(async () => resolvers[2]?.("veraltet"));
    expect(result.current.data).toBe("neuer");
    expect(result.current.loading).toBe(false);
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
