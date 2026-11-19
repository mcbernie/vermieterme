/** Click properties that decide whether a link click is a plain in-app navigation. */
type LinkClick = Pick<MouseEvent, "button" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "defaultPrevented"> & {
  target: EventTarget | null;
};

/**
 * Target of a click that navigates inside the app to another page: a plain left click (no modifier key)
 * on an `<a href>` of the same origin, without `target` other than `_self` and without `download`.
 * Links to the page that is already open (or only to an anchor on it) do not count.
 * @param e The click event.
 * @param here The current location.
 * @returns Path, query and hash to navigate to, or `null` when the click must be left to the browser.
 */
export function inAppLinkTarget(e: LinkClick, here: Location | URL): string | null {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return null;
  const anchor = e.target instanceof Element ? e.target.closest("a[href]") : null;
  if (!(anchor instanceof HTMLAnchorElement)) return null;
  const target = anchor.getAttribute("target");
  if ((target && target !== "_self") || anchor.hasAttribute("download")) return null;
  const url = new URL(anchor.getAttribute("href") ?? "", here.href);
  if (url.origin !== here.origin) return null;
  if (url.pathname === here.pathname && url.search === here.search) return null;
  return url.pathname + url.search + url.hash;
}
