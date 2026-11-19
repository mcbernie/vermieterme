import { vi } from "vitest";
import { PROPERTY, RENT_CHANGES, TENANT } from "./tenant-fixtures";

/** One stubbed answer: JSON body with status, or a raw `Response` (e.g. an HTML page). */
export type StubAnswer = { status: number; body: unknown } | Response;

/** Default answers of the API used by the tenants page, keyed by `METHOD url`. */
export function defaultAnswers(): Record<string, StubAnswer> {
  return {
    "GET /api/tenants": { status: 200, body: [TENANT] },
    "GET /api/properties": { status: 200, body: [{ id: "p1", street: PROPERTY.street }] },
    "GET /api/properties/p1": { status: 200, body: PROPERTY },
    "GET /api/rent-changes": { status: 200, body: RENT_CHANGES },
    "DELETE /api/tenants/t1": { status: 200, body: { success: true } },
    "POST /api/tenants": { status: 201, body: { id: "t2" } },
    "PUT /api/tenants/t1": { status: 200, body: { id: "t1" } },
    "POST /api/rent-changes": { status: 201, body: { id: "r9" } },
  };
}

/**
 * Stubs `fetch` with the given answers. The answers object is live: tests may replace an entry
 * between steps (e.g. an empty list after a delete). Unknown requests get a 404.
 */
export function stubApi(answers: Record<string, StubAnswer> = defaultAnswers()) {
  const fetchMock = vi.fn((url: string, init?: RequestInit) => {
    const a = answers[`${init?.method ?? "GET"} ${url}`] ?? { status: 404, body: { error: "Unbekannt" } };
    return Promise.resolve(a instanceof Response ? a.clone() : Response.json(a.body, { status: a.status }));
  });
  vi.stubGlobal("fetch", fetchMock);
  return { fetchMock, answers };
}

/** Parsed JSON bodies of all calls with the given method and URL. */
export function sentBodies(fetchMock: ReturnType<typeof stubApi>["fetchMock"], method: string, url: string) {
  return fetchMock.mock.calls
    .filter(([u, init]) => u === url && init?.method === method)
    .map(([, init]) => JSON.parse(String(init?.body)) as unknown);
}
