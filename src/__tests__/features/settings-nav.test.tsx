import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { render } from "@testing-library/react";
import Link from "next/link";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useSettingsDirty } from "@/features/settings/settings-guard";
import { SettingsFrame } from "@/features/settings/settings-layout";
import { settingsTabFor } from "@/features/settings/settings-nav";
import { EMPTY_LANDLORD } from "@/features/settings/landlord";
import { json, mockFetch } from "./settings-helpers";
import { LandlordForm } from "@/features/settings/landlord-form";

const push = vi.fn();
const onLinkClick = vi.fn();
let pathname = "/settings";
vi.mock("next/navigation", () => ({ usePathname: () => pathname, useRouter: () => ({ push }) }));

afterEach(() => {
  vi.unstubAllGlobals();
  push.mockClear();
  onLinkClick.mockClear();
  pathname = "/settings";
});

/** A sub page reporting unsaved changes. */
function DirtyPage({ dirty }: { dirty: boolean }) {
  useSettingsDirty(dirty);
  return <p>Unterseite</p>;
}

/** The settings frame as the app renders it, with the given sub page and app links outside the frame. */
function tree(page = <DirtyPage dirty={false} />) {
  return (
    <UiProvider locale="de-DE">
      <ConfirmProvider>
        <nav aria-label="App">
          <Link href="/billing" onClick={onLinkClick}>
            Abrechnungen
          </Link>
          <Link href="/billing" target="_blank">
            Abrechnungen im neuen Tab
          </Link>
          <a href="https://example.org/impressum">Extern</a>
        </nav>
        <SettingsFrame>{page}</SettingsFrame>
      </ConfirmProvider>
    </UiProvider>
  );
}

/**
 * Dispatches a click on a link and reports whether the app held it back (default action prevented). A
 * click that reaches the window unprevented is prevented there, so the test browser does not load the page.
 */
function click(name: string, init: MouseEventInit = {}) {
  let reachedWindow = false;
  let preventedBefore = false;
  const record = (e: Event) => {
    reachedWindow = true;
    preventedBefore = e.defaultPrevented;
    e.preventDefault();
  };
  window.addEventListener("click", record);
  const event = new MouseEvent("click", { bubbles: true, cancelable: true, button: 0, ...init });
  link(name).dispatchEvent(event);
  window.removeEventListener("click", record);
  return reachedWindow ? preventedBefore : event.defaultPrevented;
}

const link = (name: string) => screen.getByRole("link", { name });

describe("settingsTabFor", () => {
  it("maps the paths of the sub pages", () => {
    expect(settingsTabFor("/settings")).toBe("general");
    expect(settingsTabFor("/settings/vpi")).toBe("vpi");
    expect(settingsTabFor("/settings/pdf-template")).toBe("pdf-template");
    expect(settingsTabFor(null)).toBe("general");
  });

  it("maps paths below a sub page by segment, not by prefix", () => {
    expect(settingsTabFor("/settings/vpi/2025")).toBe("vpi");
    expect(settingsTabFor("/settings/vpi-xyz")).toBe("general");
    expect(settingsTabFor("/settings/pdf-template-alt")).toBe("general");
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

describe("leaving the settings over app links (#37)", () => {
  it("asks before following a sidebar link; cancel stays, discard navigates", async () => {
    render(tree(<DirtyPage dirty />));
    expect(click("Abrechnungen")).toBe(true);
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /Weiter bearbeiten/ }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(push).not.toHaveBeenCalled();

    click("Abrechnungen");
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: /Verwerfen/ }));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/billing"));
  });

  it("still runs the own onClick of the link while holding back the navigation", async () => {
    render(tree(<DirtyPage dirty />));
    click("Abrechnungen");
    expect(onLinkClick).toHaveBeenCalledTimes(1);
    expect(push).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog");
    expect(push).not.toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: /Verwerfen/ }));
    await waitFor(() => expect(push).toHaveBeenCalledTimes(1));
  });

  it("asks exactly once when a settings tab is clicked with unsaved changes", async () => {
    render(tree(<DirtyPage dirty />));
    fireEvent.click(link("VPI-Werte"));
    expect(await screen.findAllByRole("dialog")).toHaveLength(1);
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: /Verwerfen/ }));
    await waitFor(() => expect(push).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("lets modifier clicks, new-tab and external links through", () => {
    render(tree(<DirtyPage dirty />));
    expect(click("Abrechnungen", { metaKey: true })).toBe(false);
    expect(click("Abrechnungen", { ctrlKey: true })).toBe(false);
    expect(click("Abrechnungen im neuen Tab")).toBe(false);
    expect(click("Extern")).toBe(false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(push).not.toHaveBeenCalled();
  });

  it("does not hold back app links without unsaved changes (counter-check)", () => {
    render(tree());
    expect(click("Abrechnungen")).toBe(false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("stops holding back app links once the changes are gone", async () => {
    const { rerender } = render(tree(<DirtyPage dirty />));
    rerender(tree(<DirtyPage dirty={false} />));
    expect(click("Abrechnungen")).toBe(false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

describe("after saving and on closing the tab (#39)", () => {
  const LANDLORD = { ...EMPTY_LANDLORD, name: "Max", street: "Weg 1", zip: "10115", city: "Berlin" };

  /** Dispatches `beforeunload` like the browser does and reports whether the page asked to stay. */
  function beforeUnloadBlocked() {
    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    return event.defaultPrevented;
  }

  it("warns on closing the tab only while there are unsaved changes", () => {
    const { rerender } = render(tree(<DirtyPage dirty />));
    expect(beforeUnloadBlocked()).toBe(true);
    rerender(tree(<DirtyPage dirty={false} />));
    expect(beforeUnloadBlocked()).toBe(false);
  });

  it("leaves without asking after the landlord form was saved", async () => {
    mockFetch({ "POST /api/settings": json({ id: "l1" }) });
    render(tree(<LandlordForm initial={LANDLORD} />));
    fireEvent.change(screen.getByLabelText(/Name/), { target: { value: "Moritz" } });
    expect(beforeUnloadBlocked()).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText("Gespeichert")).toBeInTheDocument();
    await waitFor(() => expect(beforeUnloadBlocked()).toBe(false));
    fireEvent.click(link("VPI-Werte"));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/settings/vpi"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("does not count formatting the IBAN on blur as a change", async () => {
    render(tree(<LandlordForm initial={{ ...LANDLORD, iban: "de89370400440532013000" }} />));
    const iban = screen.getByLabelText(/IBAN/);
    fireEvent.blur(iban);
    expect(iban).toHaveValue("DE89 3704 0044 0532 0130 00");
    fireEvent.click(link("VPI-Werte"));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/settings/vpi"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});


