const CONTROL_CHARS = /[\u0000-\u001f\u007f]/;

/**
 * Reduces a `callbackUrl` query value to a same-origin path. Only values with exactly one leading `/`
 * pass; `//host`, `/\host`, schemes, the login page itself, control characters (browsers strip tabs and newlines, which could
 * turn `/<TAB>/host` into `//host`) and empty values fall back to `/`.
 */
export function safeCallbackUrl(raw: string | null | undefined): string {
  if (!raw || !raw.startsWith("/")) return "/";
  if (raw.startsWith("//") || raw.includes("\\") || CONTROL_CHARS.test(raw)) return "/";
  // Returning to the login page after signing in would be a dead end.
  if (/^\/login(?:[/?#]|$)/.test(raw)) return "/";
  return raw;
}
