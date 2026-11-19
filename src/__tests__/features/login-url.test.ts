import { describe, expect, it } from "vitest";
import { loginUrl } from "@/features/auth/login-url";

describe("loginUrl", () => {
  it("returns to a local path after signing in", () => {
    expect(loginUrl("/tenants")).toBe("/login?callbackUrl=%2Ftenants");
    expect(loginUrl("/billing/1?tab=a#x")).toBe("/login?callbackUrl=%2Fbilling%2F1%3Ftab%3Da%23x");
  });

  it.each(["https://x.de/a", "//x.de", "/\\x.de", "javascript:alert(1)", "/\t/x.de", "", null, undefined, "/"])(
    "drops the non-local or empty target %s",
    (target) => {
      expect(loginUrl(target)).toBe("/login");
    },
  );
});
