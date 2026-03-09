import { describe, it, expect } from "vitest";
import { formatCurrency, formatDate, formatDateISO } from "@/lib/format";

describe("formatCurrency", () => {
  it("formats positive amounts in EUR", () => {
    const result = formatCurrency(1234.56);
    expect(result).toContain("1.234,56");
    expect(result).toContain("€");
  });

  it("formats zero", () => {
    const result = formatCurrency(0);
    expect(result).toContain("0,00");
  });

  it("formats negative amounts", () => {
    const result = formatCurrency(-50.5);
    expect(result).toContain("50,50");
  });

  it("rounds to 2 decimal places", () => {
    const result = formatCurrency(10.999);
    expect(result).toContain("11,00");
  });
});

describe("formatDate", () => {
  it("formats a Date object", () => {
    const result = formatDate(new Date(2024, 0, 15));
    expect(result).toBe("15.1.2024");
  });

  it("formats an ISO date string", () => {
    const result = formatDate("2024-06-30T00:00:00.000Z");
    expect(result).toMatch(/30\.6\.2024|1\.7\.2024/);
  });

  it("formats a date-only string", () => {
    const result = formatDate("2024-12-31");
    expect(result).toMatch(/31\.12\.2024|1\.1\.2025/);
  });
});

describe("formatDateISO", () => {
  it("returns YYYY-MM-DD format", () => {
    const result = formatDateISO(new Date(2024, 5, 15));
    expect(result).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(result).toContain("2024-06");
  });

  it("formats an ISO string to date-only", () => {
    const result = formatDateISO("2024-01-01T12:00:00.000Z");
    expect(result).toBe("2024-01-01");
  });
});
