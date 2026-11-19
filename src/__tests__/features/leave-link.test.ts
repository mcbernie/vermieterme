import { describe, expect, it } from "vitest";
import { inAppLinkTarget } from "@/features/settings/leave-link";

const here = new URL("http://localhost:3000/settings/pdf-template");

/** A click on a fresh anchor with the given attributes. */
function clickOn(attrs: Record<string, string>, init: Partial<Omit<Parameters<typeof inAppLinkTarget>[0], "target">> = {}) {
  const a = document.createElement("a");
  for (const [k, v] of Object.entries(attrs)) a.setAttribute(k, v);
  const span = document.createElement("span");
  a.append(span);
  return { button: 0, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, defaultPrevented: false, ...init, target: span };
}

describe("inAppLinkTarget", () => {
  it("returns the path of a plain in-app link (also from an element inside the link)", () => {
    expect(inAppLinkTarget(clickOn({ href: "/billing?year=2025#top" }), here)).toBe("/billing?year=2025#top");
    expect(inAppLinkTarget(clickOn({ href: "/settings/vpi", target: "_self" }), here)).toBe("/settings/vpi");
  });

  it("leaves other clicks to the browser", () => {
    expect(inAppLinkTarget(clickOn({ href: "/billing" }, { altKey: true }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "/billing" }, { shiftKey: true }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "/billing" }, { button: 1 }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "/billing" }, { defaultPrevented: true }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "/billing", download: "" }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "https://example.org/x" }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "#section" }), here)).toBeNull();
    expect(inAppLinkTarget(clickOn({ href: "/settings/pdf-template" }), here)).toBeNull();
    expect(inAppLinkTarget({ ...clickOn({}), target: null }, here)).toBeNull();
  });
});
