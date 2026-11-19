const CONTROL_CHARS = /[\u0000-\u001f\u007f]/;

/**
 * Reduces a `callbackUrl` query value to a same-origin path. Only values with exactly one leading `/`
 * pass; `//host`, `/\host`, schemes, the login page itself, control characters (browsers strip tabs and newlines, which could
 * turn `/<TAB>/host` into `//host`) and empty values fall back to `/`.
 *
 * Matching is case-sensitive, as Next.js routes are: `/LOGIN` is not the login page and is kept as a path
 * (it yields a 404, not a redirect loop). Percent-encoded forms such as `/%2f/x` or `/%5Cevil` are not decoded
 * here and stay relative paths of the same origin; the browser does not turn them into a host.
 */
export function safeCallbackUrl(raw: string | null | undefined): string {
  if (!raw || !raw.startsWith("/")) return "/";
  if (raw.startsWith("//") || raw.includes("\\") || CONTROL_CHARS.test(raw)) return "/";
  // Returning to the login page after signing in would be a dead end.
  if (/^\/login(?:[/?#]|$)/.test(raw)) return "/";
  return raw;
}
