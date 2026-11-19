import { describe, expect, it } from "vitest";
import { validateLogin } from "@/features/auth/login-validation";

describe("validateLogin", () => {
  it("accepts a plausible e-mail and a password", () => {
    expect(validateLogin(" a@b.de ", "x")).toEqual({});
  });

  it("flags empty fields", () => {
    const errors = validateLogin("  ", "");
    expect(errors.email).toBeDefined();
    expect(errors.password).toBeDefined();
  });

  it("flags a malformed e-mail", () => {
    expect(validateLogin("abc", "x").email).toBe("Das ist keine gültige E-Mail-Adresse.");
  });
});
