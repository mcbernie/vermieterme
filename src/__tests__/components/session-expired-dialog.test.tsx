import { act, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SessionExpiredDialog } from "@/components/layout/session-expired-dialog";
import { notifySessionExpired } from "@/lib/session-expiry";

afterEach(() => window.history.replaceState(null, "", "/"));

/** Renders the dialog with a navigation spy. */
function renderDialog() {
  const navigate = vi.fn();
  const view = render(
    <UiProvider locale="de-DE">
      <SessionExpiredDialog navigate={navigate} />
    </UiProvider>,
  );
  return { navigate, ...view };
}

describe("SessionExpiredDialog", () => {
  it("stays hidden until a request found the session expired", () => {
    renderDialog();
    expect(screen.queryByText("Sitzung abgelaufen")).toBeNull();
    act(() => notifySessionExpired());
    expect(screen.getByText("Sitzung abgelaufen")).toBeTruthy();
  });

  it("opens the login page with the current page as return target", () => {
    window.history.replaceState(null, "", "/tenants?tab=1#a");
    const { navigate } = renderDialog();
    act(() => notifySessionExpired());
    fireEvent.click(screen.getByRole("button", { name: "Erneut anmelden" }));
    expect(navigate).toHaveBeenCalledWith("/login?callbackUrl=%2Ftenants%3Ftab%3D1%23a");
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
