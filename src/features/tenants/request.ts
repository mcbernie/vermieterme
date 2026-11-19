/** Result of a request: the checked body on success, otherwise a German error text for the UI. */
export type RequestResult<T> = { ok: true; data: T } | { ok: false; error: string };

/** Options of [`requestJson`]. */
export interface RequestOptions<T> {
  /** HTTP method (default `GET`). */
  method?: string;
  /** JSON body; omitted when undefined. */
  body?: unknown;
  /** Checks the shape of the answer; a mismatch counts as error, so callers never use unchecked data. */
  accept?: (data: unknown) => data is T;
}

/** Shown when the answer is not the API's JSON, typically the login page after an expired session. */
export const SESSION_EXPIRED = "Sitzung abgelaufen. Bitte melden Sie sich erneut an.";

/** Shown when the JSON answer does not have the expected shape. */
export const UNEXPECTED_ANSWER = "Der Server hat eine unerwartete Antwort geliefert.";

/** Accepts any JSON array (lists of the API). */
export function isArray<T>(data: unknown): data is T[] {
  return Array.isArray(data);
}

/**
 * Sends a request and parses the JSON answer. A failed status or network error becomes a German
 * message; the server's `{ error }` text of a client error is used when present (it never carries
 * personal data). A redirected answer or one without JSON (the proxy sends an expired session to the
 * login page, which `fetch` follows with status 200) counts as expired session.
 */
export async function requestJson<T = unknown>(url: string, options: RequestOptions<T> = {}): Promise<RequestResult<T>> {
  const { method = "GET", body, accept } = options;
  let res: Response;
  try {
    res = await fetch(url, {
      method,
      ...(body === undefined
        ? {}
        : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }),
    });
  } catch {
    return { ok: false, error: "Der Server ist nicht erreichbar. Bitte versuchen Sie es erneut." };
  }
  if (res.redirected) return { ok: false, error: SESSION_EXPIRED };
  const isJson = (res.headers.get("content-type") ?? "").includes("application/json");
  const data: unknown = isJson ? await res.json().catch(() => undefined) : undefined;
  if (!res.ok) return { ok: false, error: errorText(data, res.status) };
  if (data === undefined || data === null) return { ok: false, error: SESSION_EXPIRED };
  if (accept && !accept(data)) return { ok: false, error: UNEXPECTED_ANSWER };
  return { ok: true, data: data as T };
}

/**
 * Error text of a failed answer: the server message for client errors (< 500, German from `ApiError`),
 * otherwise a generic text with the status.
 */
function errorText(data: unknown, status: number): string {
  const message = data && typeof data === "object" && "error" in data ? data.error : null;
  if (status < 500 && typeof message === "string" && message) {
    return message;
  }
  return `Die Anfrage ist fehlgeschlagen (Status ${status}).`;
}
