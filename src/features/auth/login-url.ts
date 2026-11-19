import { safeCallbackUrl } from "./safe-callback-url";

/**
 * Address of the login page that returns to `returnTo` after signing in. Only a local path is kept (same
 * rules as the login page applies to `callbackUrl`); anything else returns to the start page.
 * @param returnTo Path of the current page, e.g. `/tenants?x=1`.
 */
export function loginUrl(returnTo: string | null | undefined): string {
  const target = safeCallbackUrl(returnTo);
  return target === "/" ? "/login" : `/login?callbackUrl=${encodeURIComponent(target)}`;
}
