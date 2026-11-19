import { describe, expect, it } from "vitest";
import { safeCallbackUrl } from "@/features/auth/safe-callback-url";

describe("safeCallbackUrl", () => {
  it.each(["/billing", "/a?b=1", "/tenants#x", "/"])("keeps the local path %s", (url) => {
    expect(safeCallbackUrl(url)).toBe(url);
  });

  it.each([
    "https://x.de",
    "http://x.de/a",
    "//x.de",
    "///x.de",
    "/\\x.de",
    "\\\\x.de",
    "javascript:alert(1)",
    "billing",
    "/\t/x.de",
    "/\n/x.de",
    "/\u0000a",
    "",
    null,
    undefined,
  ])("falls back to / for %j", (url) => {
    expect(safeCallbackUrl(url)).toBe("/");
  });
});
