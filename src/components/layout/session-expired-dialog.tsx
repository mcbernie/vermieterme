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
 * Hint shown when a request found the session expired, with the way back to the login page. Signing in
 * returns to the current page. It does not redirect on its own, so the user can still read or copy unsaved
 * inputs; closing keeps the page, the next failed request shows the hint again.
 * @param navigate Opens the login page (replaceable in tests).
 */
export function SessionExpiredDialog({ navigate = (url: string) => window.location.assign(url) }: {
  navigate?: (url: string) => void;
}) {
  const [open, setOpen] = useState(false);

  useEffect(() => onSessionExpired(() => setOpen(true)), []);

  return (
    <Modal
      open={open}
      onOpenChange={setOpen}
      size="sm"
      title="Sitzung abgelaufen"
      description="Ihre Sitzung ist abgelaufen. Bitte melden Sie sich erneut an, um weiterzuarbeiten."
      footer={
        <>
          <Button type="button" variant="secondary" onClick={() => setOpen(false)}>
            Später
          </Button>
          <Button type="button" variant="primary" onClick={() => navigate(loginUrl(currentPath()))}>
            Erneut anmelden
          </Button>
        </>
      }
    />
  );
}
