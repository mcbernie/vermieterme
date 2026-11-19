/**
 * The one client for requests of the web pages to the own API routes. It gives every failure the same
 * form (`RequestError` with a German text that is safe to show) and detects an expired session: the proxy
 * redirects `/api/*` to `/login`, which `fetch` follows and answers with the HTML login page and status 200;
 * a route reached without session answers 401. Both raise `SessionExpiredError` and notify the application
 * frame, which offers to sign in again.
 */
import { notifySessionExpired } from "./session-expiry";

/** Shown when the session ran out while a request was made. */
export const SESSION_EXPIRED = "Sitzung abgelaufen. Bitte melden Sie sich erneut an.";

/** Shown when the JSON answer does not have the expected shape. */
export const UNEXPECTED_ANSWER = "Der Server hat eine unerwartete Antwort geliefert.";

/** Shown when the server could not be reached and the caller gave no own text. */
export const UNREACHABLE = "Der Server ist nicht erreichbar. Bitte versuchen Sie es erneut.";

/** Kind of a failed request; lets callers react without comparing texts. */
export type RequestErrorKind = "session" | "network" | "status" | "unexpected";

/** Failed request with a German, user-facing message (never personal data). */
export class RequestError extends Error {
  /**
   * @param message Text for the user.
   * @param kind What went wrong; defaults to `"unexpected"`, since `"status"` needs an HTTP answer.
   * @param status HTTP status of the answer, `0` when there was none.
   */
  constructor(
    message: string,
    public readonly kind: RequestErrorKind = "unexpected",
    public readonly status = 0,
  ) {
    super(message);
    this.name = "RequestError";
  }
}

/** The session ran out; callers tell it apart from other failures by type, not by its text. */
export class SessionExpiredError extends RequestError {
  constructor(status = 0) {
    super(SESSION_EXPIRED, "session", status);
    this.name = "SessionExpiredError";
  }
}

/** Options of a request. */
export interface RequestOptions {
  /** HTTP method (default `GET`). */
  method?: string;
  /** JSON body, serialised with the right header; omitted when undefined. */
  json?: unknown;
  /** Form data body (file upload); the browser sets the header. Ignored when `json` is set. */
  form?: FormData;
  /** Aborts the request; an aborted request rethrows the abort error unchanged. */
  signal?: AbortSignal;
  /** Text when the request fails without an own server text; otherwise a generic text is used. */
  fallback?: string;
}

/** Options of a request with a JSON answer. */
export interface JsonOptions<T> extends RequestOptions {
  /** Checks the shape of the answer; a mismatch counts as unexpected answer. */
  accept?: (data: unknown) => data is T;
}

/** Accepts any JSON array (lists of the API). */
export function isArray<T>(data: unknown): data is T[] {
  return Array.isArray(data);
}

/** Builds the `fetch` options; keys without value are left out. */
function fetchInit({ method = "GET", json, form, signal }: RequestOptions): RequestInit {
  const init: RequestInit = { method };
  if (json !== undefined) {
    init.headers = { "Content-Type": "application/json" };
    init.body = JSON.stringify(json);
  } else if (form) {
    init.body = form;
  }
  if (signal) init.signal = signal;
  return init;
}

/** Content type of an answer in lower case (`""` when missing). */
function contentType(res: Response): string {
  return (res.headers?.get("Content-Type") ?? "").toLowerCase();
}

/** Reports the expired session to the frame and returns the error to throw. */
function expired(status: number): SessionExpiredError {
  notifySessionExpired();
  return new SessionExpiredError(status);
}

/** The `error` text of an API error body, if there is one; an abort while reading is rethrown. */
async function serverText(res: Response, signal: AbortSignal | undefined): Promise<string | null> {
  try {
    const data: unknown = await res.json();
    if (data && typeof data === "object" && "error" in data && typeof data.error === "string" && data.error) {
      return data.error;
    }
  } catch (error) {
    if (signal?.aborted) throw error;
    // Body is not JSON; the caller falls back to a generic text.
  }
  return null;
}

/**
 * Error of a failed status: the server text for client errors (< 500, German from `ApiError`), otherwise
 * the fallback or a generic text with the status. Server errors only carry an English generic text.
 */
async function statusError(res: Response, options: RequestOptions): Promise<RequestError> {
  const { fallback, signal } = options;
  const text = res.status < 500 ? await serverText(res, signal) : null;
  const message = text ?? fallback ?? `Die Anfrage ist fehlgeschlagen (Status ${res.status}).`;
  return new RequestError(message, "status", res.status);
}

/**
 * Sends a request and checks the answer: a network error, an expired session or a failed status throw a
 * `RequestError`. Use it directly for answers that are not JSON (file downloads, empty answers).
 * @returns The successful answer, body unread.
 */
export async function apiFetch(url: string, options: RequestOptions = {}): Promise<Response> {
  let res: Response;
  try {
    res = await fetch(url, fetchInit(options));
  } catch (error) {
    if (options.signal?.aborted) throw error;
    throw new RequestError(options.fallback ?? UNREACHABLE, "network");
  }
  if (res.redirected || res.status === 401) throw expired(res.status);
  if (res.ok && contentType(res).includes("text/html")) throw expired(res.status);
  if (!res.ok) throw await statusError(res, options);
  return res;
}

/**
 * Sends a request and returns the parsed JSON answer. A 2xx answer without JSON or with unreadable JSON is
 * the login page and counts as expired session; `204 No Content` yields `null`. JSON `null` is a valid
 * answer (e.g. settings not created yet) unless `accept` rejects it.
 * @throws RequestError on any failure, `SessionExpiredError` for an expired session.
 */
export async function apiJson<T = unknown>(url: string, options: JsonOptions<T> = {}): Promise<T> {
  const res = await apiFetch(url, options);
  let data: unknown = null;
  if (res.status !== 204) {
    const type = contentType(res);
    if (type !== "" && !type.includes("json")) throw expired(res.status);
    try {
      data = await res.json();
    } catch (error) {
      // Aborted while reading the body: no sign of an expired session, rethrow unchanged as in `apiFetch`.
      if (options.signal?.aborted) throw error;
      throw expired(res.status);
    }
  }
  if (options.accept && !options.accept(data)) throw new RequestError(UNEXPECTED_ANSWER, "unexpected", res.status);
  return data as T;
}

/** Result of {@link apiResult}: the checked body on success, otherwise a German error text for the UI. */
export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: string };

/**
 * Like {@link apiJson}, but returns the failure as value instead of throwing. JSON `null` counts as unexpected
 * answer here: callers of this form always expect data.
 */
export async function apiResult<T = unknown>(url: string, options: JsonOptions<T> = {}): Promise<ApiResult<T>> {
  try {
    const data = await apiJson<T>(url, options);
    if (data === null) return { ok: false, error: UNEXPECTED_ANSWER };
    return { ok: true, data };
  } catch (error) {
    return { ok: false, error: errorText(error, options.fallback ?? UNREACHABLE) };
  }
}

/**
 * Logs a failed request, the one log rule of all callers: what was requested (method and path; paths carry
 * IDs, never personal data) and the user-facing text of a `RequestError`, other errors as they are.
 * @param what E.g. `Request POST /api/x` or `Fetch /api/x`.
 */
export function logRequestFailure(what: string, error: unknown) {
  console.error(`${what} failed:`, error instanceof RequestError ? error.message : error);
}

/**
 * Sends a request for pages that only need to know whether it worked; the failure is logged.
 * @returns `true` when the API accepted the request.
 */
export async function apiSend(url: string, options: RequestOptions = {}): Promise<boolean> {
  try {
    await apiFetch(url, options);
    return true;
  } catch (error) {
    logRequestFailure(`Request ${options.method ?? "GET"} ${url}`, error);
    return false;
  }
}

/** Outcome of {@link apiAttempt}: the answer, or the error that was thrown. */
export type Attempt<T> = { ok: true; data: T } | { ok: false; error: unknown };

/**
 * Loads JSON and returns the failure as value; the one place that logs failed loads (unless the request
 * was aborted). Basis of {@link apiLoad}, {@link apiLoadAll} and the loaders of the billing pages.
 */
export async function apiAttempt<T = unknown>(url: string, options: JsonOptions<T> = {}): Promise<Attempt<T>> {
  try {
    return { ok: true, data: await apiJson<T>(url, options) };
  } catch (error) {
    if (!options.signal?.aborted) logRequestFailure(`Fetch ${url}`, error);
    return { ok: false, error };
  }
}

/**
 * Loads JSON for pages that show nothing on failure; the failure is logged unless the request was aborted.
 * @returns The answer, or `null` on any failure.
 */
export async function apiLoad<T>(url: string, options: JsonOptions<T> = {}): Promise<T | null> {
  const attempt = await apiAttempt<T>(url, options);
  return attempt.ok ? attempt.data : null;
}

/** Result of {@link apiLoadAll}: the loaded answers per key and whether any endpoint failed. */
export interface LoadAllResult<T> {
  data: Partial<T>;
  failed: boolean;
}

/**
 * Loads several JSON endpoints at once for pages that show what they got: a failed endpoint leaves its
 * key empty, is logged (unless aborted) and sets `failed`; JSON `null` is a valid answer and also leaves
 * the key empty. An expired session is reported like for every other request.
 * @param urls Endpoint per key of the result.
 */
export async function apiLoadAll<T extends object>(
  urls: { [K in keyof T]: string },
  signal?: AbortSignal,
): Promise<LoadAllResult<T>> {
  const entries = Object.entries(urls) as [keyof T, string][];
  const results = await Promise.all(
    entries.map(async ([key, url]) => ({ key, attempt: await apiAttempt(url, { signal }) })),
  );
  const data: Partial<T> = {};
  for (const { key, attempt } of results) {
    if (attempt.ok && attempt.data !== null) (data as Record<keyof T, unknown>)[key] = attempt.data;
  }
  return { data, failed: results.some(({ attempt }) => !attempt.ok) };
}

/** Text of an error for the UI: the message of a `RequestError`, otherwise `fallback`. */
export function errorText(error: unknown, fallback: string): string {
  return error instanceof RequestError ? error.message : fallback;
}
