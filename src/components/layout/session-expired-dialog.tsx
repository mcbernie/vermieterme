"use client";

import { Button, Modal } from "@zweiund40gmbh/ui";
import { useEffect, useState } from "react";
import { loginUrl } from "@/features/auth/login-url";
import { onSessionExpired } from "@/lib/session-expiry";

/** Current page as local path with query and fragment, used as return target after signing in. */
function currentPath(): string {
  const { pathname, search, hash } = window.location;
  return `${pathname}${search}${hash}`;
}

/**
 * Hint shown when a request found the session expired, with the way back to the login page. The login page
 * opens in a new tab (return target: the current page), so unsaved inputs here are kept: after signing in
 * the user comes back and saves again. The dialog then closes; the next failed request shows it again.
 */
export function SessionExpiredDialog() {
  const [open, setOpen] = useState(false);
  // Fixed when the dialog opens and kept on close: closing on click re-renders the link before the browser
  // follows it, so a href derived from `open` would change to a placeholder and open the wrong page.
  const [href, setHref] = useState(() => loginUrl("/"));

  useEffect(
    () =>
      onSessionExpired(() => {
        setHref(loginUrl(currentPath()));
        setOpen(true);
      }),
    [],
  );

  return (
    <Modal
      open={open}
      onOpenChange={setOpen}
      size="sm"
      title="Sitzung abgelaufen"
      description="Ihre Sitzung ist abgelaufen. Die Anmeldung öffnet sich in einem neuen Tab; Ihre Eingaben hier bleiben erhalten. Kehren Sie nach der Anmeldung hierher zurück und speichern Sie erneut."
      footer={
        <>
          <Button type="button" variant="secondary" onClick={() => setOpen(false)}>
            Später
          </Button>
          {/* A real link: a new tab always opens (a popup could be blocked or become a window). */}
          <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className="zui-button"
            data-variant="primary"
            data-size="md"
            onClick={() => setOpen(false)}
          >
            Erneut anmelden
          </a>
        </>
      }
    />
  );
}
