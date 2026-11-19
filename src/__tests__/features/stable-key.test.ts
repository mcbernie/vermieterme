import { describe, expect, it } from "vitest";
import { stableKey } from "@/features/settings/stable-key";

describe("stableKey", () => {
  it("gives the same key for the same content in a different key order", () => {
    expect(stableKey({ a: 1, b: { c: [1, { d: 2, e: 3 }] } })).toBe(stableKey({ b: { c: [1, { e: 3, d: 2 }] }, a: 1 }));
  });

  it("gives different keys for different content (counter-check)", () => {
    expect(stableKey({ a: 1 })).not.toBe(stableKey({ a: 2 }));
    expect(stableKey([1, 2])).not.toBe(stableKey([2, 1]));
  });

  it("returns null for missing values", () => {
    expect(stableKey(null)).toBeNull();
    expect(stableKey(undefined)).toBeNull();
  });
});
