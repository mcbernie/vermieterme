import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useSettingsDirty } from "@/features/settings/settings-guard";
import { SettingsFrame } from "@/features/settings/settings-layout";
import { settingsTabFor } from "@/features/settings/settings-nav";
import { EMPTY_LANDLORD } from "@/features/settings/landlord";
import { LandlordForm } from "@/features/settings/landlord-form";

const push = vi.fn();
let pathname = "/settings";
vi.mock("next/navigation", () => ({ usePathname: () => pathname, useRouter: () => ({ push }) }));

afterEach(() => {
  push.mockClear();
  pathname = "/settings";
});

/** A sub page reporting unsaved changes. */
function DirtyPage({ dirty }: { dirty: boolean }) {
  useSettingsDirty(dirty);
  return <p>Unterseite</p>;
}

/** The settings frame as the app renders it, with the given sub page. */
function tree(page = <DirtyPage dirty={false} />) {
  return (
    <UiProvider locale="de-DE">
      <ConfirmProvider>
        <SettingsFrame>{page}</SettingsFrame>
      </ConfirmProvider>
    </UiProvider>
  );
}

const link = (name: string) => screen.getByRole("link", { name });

describe("settingsTabFor", () => {
  it("maps the paths of the sub pages", () => {
    expect(settingsTabFor("/settings")).toBe("general");
    expect(settingsTabFor("/settings/vpi")).toBe("vpi");
    expect(settingsTabFor("/settings/pdf-template")).toBe("pdf-template");
    expect(settingsTabFor(null)).toBe("general");
  });
});

describe("SettingsNav", () => {
  it("marks the open sub page as a link with aria-current, without tab roles or aria-controls", () => {
    render(tree());
    expect(link("Allgemein")).toHaveAttribute("aria-current", "page");
    expect(link("VPI-Werte")).not.toHaveAttribute("aria-current");
    expect(link("VPI-Werte")).toHaveAttribute("href", "/settings/vpi");
    expect(screen.queryByRole("tablist")).not.toBeInTheDocument();
    expect(document.querySelector("[aria-controls]")).toBeNull();
  });

  it("does not navigate on an arrow key", () => {
    render(tree());
    link("Allgemein").focus();
    fireEvent.keyDown(link("Allgemein"), { key: "ArrowRight" });
    fireEvent.keyDown(link("Allgemein"), { key: "End" });
    expect(push).not.toHaveBeenCalled();
  });

  it("navigates on click and keeps the focus on the link after the sub page changed", async () => {
    const { rerender } = render(tree());
    const vpi = link("VPI-Werte");
    vpi.focus();
    fireEvent.click(vpi);
    await waitFor(() => expect(push).toHaveBeenCalledWith("/settings/vpi"));
    pathname = "/settings/vpi";
    rerender(tree(<DirtyPage dirty={false} key="vpi" />));
    expect(link("VPI-Werte")).toBe(vpi);
    expect(document.activeElement).toBe(vpi);
    expect(vpi).toHaveAttribute("aria-current", "page");
  });

  it("leaves modifier clicks (new tab) to the browser", () => {
    render(tree());
    const event = new MouseEvent("click", { bubbles: true, cancelable: true, ctrlKey: true });
    link("VPI-Werte").dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
    expect(push).not.toHaveBeenCalled();
  });

  it("asks before leaving a sub page with unsaved changes", async () => {
    render(tree(<DirtyPage dirty />));
    fireEvent.click(link("PDF-Vorlage"));
    const dialog = await screen.findByRole("dialog");
    expect(push).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: /Weiter bearbeiten/ }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(push).not.toHaveBeenCalled();

    fireEvent.click(link("PDF-Vorlage"));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: /Verwerfen/ }));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/settings/pdf-template"));
  });

  it("asks after the landlord form was changed, not before", async () => {
    render(tree(<LandlordForm initial={{ ...EMPTY_LANDLORD, name: "Max" }} />));
    fireEvent.click(link("VPI-Werte"));
    await waitFor(() => expect(push).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/Name/), { target: { value: "Moritz" } });
    fireEvent.click(link("VPI-Werte"));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(push).toHaveBeenCalledTimes(1);
  });
});
