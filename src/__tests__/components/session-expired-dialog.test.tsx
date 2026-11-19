import { act, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { afterEach, describe, expect, it } from "vitest";
import { SessionExpiredDialog } from "@/components/layout/session-expired-dialog";
import { notifySessionExpired } from "@/lib/session-expiry";

afterEach(() => window.history.replaceState(null, "", "/"));

/** Renders the dialog. */
function renderDialog() {
  return render(
    <UiProvider locale="de-DE">
      <SessionExpiredDialog />
    </UiProvider>,
  );
}

describe("SessionExpiredDialog", () => {
  it("stays hidden until a request found the session expired", () => {
    renderDialog();
    expect(screen.queryByText("Sitzung abgelaufen")).toBeNull();
    act(() => notifySessionExpired());
    expect(screen.getByText("Sitzung abgelaufen")).toBeTruthy();
  });

  it("links to the login page in a new tab with the current page as return target, and closes", () => {
    window.history.replaceState(null, "", "/tenants?tab=1#a");
    renderDialog();
    act(() => notifySessionExpired());
    const link = screen.getByRole("link", { name: "Erneut anmelden" });
    expect(link.getAttribute("href")).toBe("/login?callbackUrl=%2Ftenants%3Ftab%3D1%23a");
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toBe("noopener noreferrer");
    fireEvent.click(link);
    expect(screen.queryByText("Sitzung abgelaufen")).toBeNull();
    // The link is followed after the click handler's state update: its target must not have changed.
    expect(link.getAttribute("href")).toBe("/login?callbackUrl=%2Ftenants%3Ftab%3D1%23a");
  });

  it("never points the link at a placeholder", () => {
    renderDialog();
    act(() => notifySessionExpired());
    const link = screen.getByRole("link", { name: "Erneut anmelden" });
    expect(link.getAttribute("href")).not.toBe("#");
    expect(link.getAttribute("href")).toBe("/login");
  });

  it("closes on „Später“ and shows again on the next expired request", () => {
    renderDialog();
    act(() => notifySessionExpired());
    fireEvent.click(screen.getByRole("button", { name: "Später" }));
    expect(screen.queryByText("Sitzung abgelaufen")).toBeNull();
    act(() => notifySessionExpired());
    expect(screen.getByText("Sitzung abgelaufen")).toBeTruthy();
  });

  it("stops listening after unmount", () => {
    const { unmount } = renderDialog();
    unmount();
    expect(() => act(() => notifySessionExpired())).not.toThrow();
  });
});
