import { apiFetch, apiJson, errorText, logRequestFailure, RequestError } from "@/lib/api-client";

/** Outcome of a write request: success, or a German message for the user. */
export interface WriteResult {
  ok: boolean;
  error: string | null;
}

/** Outcome of a read request; `status` is 0 when the request did not reach the server. */
export type LoadResult<T> = { ok: true; data: T } | { ok: false; status: number };

/**
 * Sends a JSON request and reports whether it succeeded, in the result form of the billing hooks.
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
    await apiFetch(url, { method, json: body, fallback });
    return { ok: true, error: null };
  } catch (error) {
    logRequestFailure(`Request ${method} ${url}`, error);
    return { ok: false, error: errorText(error, fallback) };
  }
}

/**
 * Fetches JSON and reports the HTTP status on failure. An aborted request also counts as a failure;
 * callers check their signal before using the result.
 */
export async function loadJson<T>(url: string, signal?: AbortSignal): Promise<LoadResult<T>> {
  try {
    return { ok: true, data: await apiJson<T>(url, { signal }) };
  } catch (error) {
    if (!signal?.aborted) logRequestFailure(`Fetch ${url}`, error);
    return { ok: false, status: error instanceof RequestError ? error.status : 0 };
  }
}
