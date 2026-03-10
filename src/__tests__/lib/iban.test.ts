import { describe, it, expect } from "vitest";
import { validateIBAN, formatIBAN } from "@/lib/iban";

describe("validateIBAN", () => {
  it("validates a correct German IBAN", () => {
    expect(validateIBAN("DE89370400440532013000")).toBe(true);
  });

  it("validates with spaces", () => {
    expect(validateIBAN("DE89 3704 0044 0532 0130 00")).toBe(true);
  });

  it("validates lowercase input", () => {
    expect(validateIBAN("de89370400440532013000")).toBe(true);
  });

  it("validates an Austrian IBAN", () => {
    expect(validateIBAN("AT611904300234573201")).toBe(true);
  });

  it("validates a Swiss IBAN", () => {
    expect(validateIBAN("CH9300762011623852957")).toBe(true);
  });

  it("rejects empty string", () => {
    expect(validateIBAN("")).toBe(false);
  });

  it("rejects random text", () => {
    expect(validateIBAN("hello world")).toBe(false);
  });

  it("rejects wrong check digits", () => {
    expect(validateIBAN("DE00370400440532013000")).toBe(false);
  });

  it("rejects wrong length for country", () => {
    // German IBAN must be 22 characters, this is 21
    expect(validateIBAN("DE8937040044053201300")).toBe(false);
  });

  it("rejects invalid country code", () => {
    expect(validateIBAN("XX89370400440532013000")).toBe(false);
  });

  it("validates another correct German IBAN", () => {
    expect(validateIBAN("DE02120300000000202051")).toBe(true);
  });
});

describe("formatIBAN", () => {
  it("formats with spaces every 4 characters", () => {
    expect(formatIBAN("DE89370400440532013000")).toBe(
      "DE89 3704 0044 0532 0130 00"
    );
  });

  it("handles already formatted input", () => {
    expect(formatIBAN("DE89 3704 0044 0532 0130 00")).toBe(
      "DE89 3704 0044 0532 0130 00"
    );
  });

  it("converts to uppercase", () => {
    expect(formatIBAN("de89370400440532013000")).toBe(
      "DE89 3704 0044 0532 0130 00"
    );
  });
});
