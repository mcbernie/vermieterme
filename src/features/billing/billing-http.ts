/** Outcome of a write request: success, or a German message for the user. */
export interface WriteResult {
  ok: boolean;
  error: string | null;
}

/** Outcome of a read request; `status` is 0 when the request did not reach the server. */
export type LoadResult<T> = { ok: true; data: T } | { ok: false; status: number };

/**
 * Sends a JSON request and reports whether it succeeded.
 * @param url Endpoint.
 * @param method HTTP method.
 * @param body JSON body, omitted for `undefined`.
 * @param fallback Message used when the server sends no `error` text (or the request fails).
 */
export async function sendJson(
  url: string,
  method: string,
  body: unknown,
  fallback: string,
): Promise<WriteResult> {
  try {
    const res = await fetch(url, {
      method,
      ...(body === undefined
        ? {}
        : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }),
    });
    if (res.ok) return { ok: true, error: null };
    const data: unknown = await res.json().catch(() => null);
    return { ok: false, error: serverError(data) ?? fallback };
  } catch (error) {
    console.error(`Request ${method} ${url} failed:`, error);
    return { ok: false, error: fallback };
  }
}

/** The `error` text of an API error body, if there is one. */
function serverError(data: unknown): string | null {
  if (data && typeof data === "object" && "error" in data && typeof data.error === "string") {
    return data.error;
  }
  return null;
}

/**
 * Fetches JSON and reports the HTTP status on failure. An aborted request also counts as a failure;
 * callers check their signal before using the result.
 */
export async function loadJson<T>(url: string, signal?: AbortSignal): Promise<LoadResult<T>> {
  try {
    const res = await fetch(url, { signal });
    if (!res.ok) return { ok: false, status: res.status };
    return { ok: true, data: (await res.json()) as T };
  } catch (error) {
    if (!signal?.aborted) console.error(`Failed to fetch ${url}:`, error);
    return { ok: false, status: 0 };
  }
}
