/** Result of a request: the parsed body on success, otherwise a German error text for the UI. */
export type RequestResult<T> = { ok: true; data: T } | { ok: false; error: string };

/**
 * Sends a request and parses the JSON answer. A failed status or network error becomes a German
 * message; the server's `{ error }` text of a client error is used when present (it never carries
 * personal data).
 * @param url Endpoint below `/api`.
 * @param method HTTP method (default `GET`).
 * @param body JSON body; omitted when undefined.
 */
export async function requestJson<T>(url: string, method = "GET", body?: unknown): Promise<RequestResult<T>> {
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
  const data: unknown = await res.json().catch(() => null);
  if (!res.ok) return { ok: false, error: errorText(data, res.status) };
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
