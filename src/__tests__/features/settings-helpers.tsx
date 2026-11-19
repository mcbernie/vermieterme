import type { ReactElement } from "react";
import { render } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { vi } from "vitest";

/** Options of a fake response. */
interface FakeOptions {
  status?: number;
  contentType?: string;
  redirected?: boolean;
  headers?: Record<string, string>;
}

/** Builds a minimal `Response` (happy-dom cannot set `redirected` on a real one). */
export function fakeResponse(body: unknown, options: FakeOptions = {}): Response {
  const status = options.status ?? 200;
  const contentType = options.contentType ?? "application/json";
  const text = typeof body === "string" ? body : JSON.stringify(body);
  const headers = new Headers({ "Content-Type": contentType, ...options.headers });
  return {
    ok: status >= 200 && status < 300,
    status,
    redirected: options.redirected ?? false,
    headers,
    json: async () => JSON.parse(text),
    text: async () => text,
    blob: async () => new Blob([text], { type: contentType }),
  } as unknown as Response;
}

/** JSON answer of the API. */
export const json = (data: unknown, status = 200) => fakeResponse(data, { status });

/** What the proxy sends after the session expired: the login page as HTML with status 200. */
export const loginPage = () => fakeResponse("<!doctype html><html></html>", { contentType: "text/html", redirected: true });

/** A route of the fetch mock: method and path to a response (or a function building one). */
export type Routes = Record<string, Response | ((body: unknown) => Response | Promise<Response>)>;

/**
 * Stubs `fetch` with fixed routes (`"GET /api/x"`); unknown routes answer 404. Returns the recorded calls.
 */
export function mockFetch(routes: Routes) {
  const calls: { method: string; url: string; body: unknown }[] = [];
  const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    const body = typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
    calls.push({ method, url, body });
    const route = routes[`${method} ${url}`];
    if (!route) return json({ error: "nicht gefunden" }, 404);
    return typeof route === "function" ? route(body) : route;
  });
  vi.stubGlobal("fetch", fetchMock);
  return { calls, fetchMock };
}

/** Renders inside the UI provider (German) and the confirm provider, like the app does. */
export function renderWithUi(ui: ReactElement) {
  return render(
    <UiProvider locale="de-DE">
      <ConfirmProvider>{ui}</ConfirmProvider>
    </UiProvider>,
  );
}
