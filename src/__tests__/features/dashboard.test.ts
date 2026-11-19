import { describe, expect, it } from "vitest";
import { billingStatusTone } from "@/features/dashboard/billing-tone";
import { formatVpiPercent, vpiReason } from "@/features/dashboard/vpi-text";

describe("billingStatusTone", () => {
  it("maps known status labels to tones", () => {
    expect(billingStatusTone("Bezahlt")).toBe("ok");
    expect(billingStatusTone("Versendet")).toBe("info");
    expect(billingStatusTone("In Bearbeitung")).toBe("warn");
    expect(billingStatusTone("Offen")).toBe("neutral");
  });

  it("falls back to neutral", () => {
    expect(billingStatusTone("Unbekannt")).toBe("neutral");
  });
});

describe("vpi text", () => {
  it("prefixes positive changes with a plus sign", () => {
    expect(formatVpiPercent({ percentageChange: 2.5 })).toBe("+2.50%");
    expect(formatVpiPercent({ percentageChange: -0.3 })).toBe("-0.30%");
  });

  it("builds the default reason", () => {
    expect(vpiReason({ referenceVpi: 100, currentVpi: 103, percentageChange: 3 })).toBe(
      "VPI-Anpassung (100 → 103, +3.00%)"
    );
  });
});
