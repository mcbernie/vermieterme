/**
 * Small fetch helpers for the settings and profile pages. Every request checks the status and detects an
 * expired session: the proxy redirects `/api/*` to `/login` and answers with HTML and status 200, so a
 * redirected response or a 2xx answer without JSON counts as an expired session.
 */

/** Message shown when the session ran out while a request was made. */
export const SESSION_EXPIRED = "Sitzung abgelaufen. Bitte melden Sie sich erneut an.";

/** Error with a German, user-facing message that is safe to show (no personal data). */
export class RequestError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "RequestError";
  }
}

/** The session ran out; callers tell it apart from other failures by type, not by its text. */
export class SessionExpiredError extends RequestError {
  constructor() {
    super(SESSION_EXPIRED);
    this.name = "SessionExpiredError";
  }
}

/** Whether a response came from the login redirect instead of the API. */
function isLoginRedirect(res: Response): boolean {
  if (res.redirected) return true;
  const type = res.headers?.get("Content-Type") ?? "";
  return res.ok && type !== "" && !type.includes("json");
}

/** Reads the `error` field of an API error body, if there is one. */
async function apiErrorText(res: Response): Promise<string | null> {
  try {
    const data: unknown = await res.json();
    if (data && typeof data === "object" && "error" in data && typeof data.error === "string") {
      return data.error;
    }
  } catch {
    // Body is not JSON; fall back to the generic message.
  }
  return null;
}

/**
 * Checks a response and throws a `RequestError` for an expired session or a failed request.
 * @param res Response to check.
 * @param fallback Message when the API sent no own error text.
 */
export async function ensureOk(res: Response, fallback: string): Promise<void> {
  if (isLoginRedirect(res)) throw new SessionExpiredError();
  if (!res.ok) throw new RequestError((await apiErrorText(res)) ?? fallback);
}

/**
 * Sends a request and returns the parsed JSON body.
 * @param url API path.
 * @param fallback Message used when the request fails without an API error text.
 * @param init Method, body and abort signal; a `json` body is serialised with the right header.
 * @throws RequestError on a failed request, an expired session or a network error.
 */
export async function requestJson<T>(
  url: string,
  fallback: string,
  init: { method?: string; json?: unknown; signal?: AbortSignal } = {},
): Promise<T> {
  let res: Response;
  try {
    res = await fetch(url, {
      method: init.method ?? "GET",
      signal: init.signal,
      ...(init.json === undefined
        ? {}
        : { headers: { "Content-Type": "application/json" }, body: JSON.stringify(init.json) }),
    });
  } catch (error) {
    if (init.signal?.aborted) throw error;
    throw new RequestError(fallback);
  }
  await ensureOk(res, fallback);
  try {
    return (await res.json()) as T;
  } catch {
    // A 2xx answer without a JSON body is read as the login page. All routes used here answer with JSON;
    // a future 204 answer would be misreported, see #17.
    throw new SessionExpiredError();
  }
}

/** Text of an error for an `InlineNotice`: the message of a `RequestError`, otherwise `fallback`. */
export function errorText(error: unknown, fallback: string): string {
  return error instanceof RequestError ? error.message : fallback;
}
