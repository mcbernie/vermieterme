import { describe, expect, it } from "vitest";
import { navKeyFor } from "@/components/layout/nav-items";

describe("navKeyFor", () => {
  it("marks the dashboard only for the root path", () => {
    expect(navKeyFor("/")).toBe("dashboard");
    expect(navKeyFor("/unknown")).toBeUndefined();
  });

  it("matches sub pages by path prefix", () => {
    expect(navKeyFor("/billing/abc")).toBe("billing");
    expect(navKeyFor("/settings/vpi")).toBe("settings");
  });

  it("does not match a mere string prefix", () => {
    expect(navKeyFor("/tenantsx")).toBeUndefined();
  });
});
