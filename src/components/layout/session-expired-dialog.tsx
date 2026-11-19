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

/** Opens the login page in a new tab, so this page and its unsaved inputs stay as they are. */
function openInNewTab(url: string) {
  window.open(url, "_blank", "noopener");
}

/**
 * Hint shown when a request found the session expired, with the way back to the login page. The login page
 * opens in a new tab (return target: the current page), so unsaved inputs here are kept: after signing in
 * the user comes back and saves again. The dialog then closes; the next failed request shows it again.
 * @param navigate Opens the login page (replaceable in tests).
 */
export function SessionExpiredDialog({ navigate = openInNewTab }: { navigate?: (url: string) => void }) {
  const [open, setOpen] = useState(false);

  useEffect(() => onSessionExpired(() => setOpen(true)), []);

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
          <Button type="button" variant="primary" onClick={() => {
              navigate(loginUrl(currentPath()));
              setOpen(false);
            }}>
            Erneut anmelden
          </Button>
        </>
      }
    />
  );
}
