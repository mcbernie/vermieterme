import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/auth", () => ({
  auth: vi.fn().mockResolvedValue({ user: { id: "test-user" } }),
}));

vi.mock("@/lib/prisma", () => ({
  prisma: {
    billingPeriod: {
      findMany: vi.fn(),
      findFirst: vi.fn(),
      create: vi.fn(),
    },
    property: {
      findUnique: vi.fn(),
    },
  },
}));

import { GET, POST } from "@/app/api/billing-periods/route";
import { prisma } from "@/lib/prisma";

describe("GET /api/billing-periods", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns billing periods with property and cost count", async () => {
    const mockPeriods = [
      {
        id: "bp1",
        propertyId: "p1",
        startDate: new Date("2024-01-01"),
        endDate: new Date("2024-12-31"),
        billingDate: null,
        property: { street: "Teststr. 1", zip: "28195", city: "Bremen" },
        _count: { costs: 3 },
      },
    ];

    vi.mocked(prisma.billingPeriod.findMany).mockResolvedValue(
      mockPeriods as any
    );

    const response = await GET();
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data).toHaveLength(1);
    expect(data[0].property.street).toBe("Teststr. 1");
    expect(data[0]._count.costs).toBe(3);
  });

  it("returns 500 on database error", async () => {
    vi.mocked(prisma.billingPeriod.findMany).mockRejectedValue(
      new Error("DB error")
    );

    const response = await GET();
    expect(response.status).toBe(500);
  });
});

describe("POST /api/billing-periods", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: no overlapping period, property exists with units
    vi.mocked(prisma.billingPeriod.findFirst).mockResolvedValue(null);
    vi.mocked(prisma.property.findUnique).mockResolvedValue({
      id: "p1",
      _count: { units: 2 },
    } as any);
  });

  it("creates a billing period", async () => {
    const mockCreated = {
      id: "bp-new",
      propertyId: "p1",
      startDate: new Date("2025-01-01"),
      endDate: new Date("2025-12-31"),
      billingDate: null,
    };

    vi.mocked(prisma.billingPeriod.create).mockResolvedValue(
      mockCreated as any
    );

    const request = new Request("http://localhost/api/billing-periods", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        propertyId: "p1",
        startDate: "2025-01-01",
        endDate: "2025-12-31",
        billingDate: null,
      }),
    });

    const response = await POST(request);
    const data = await response.json();

    expect(response.status).toBe(201);
    expect(data.id).toBe("bp-new");
    expect(prisma.billingPeriod.create).toHaveBeenCalledOnce();
  });

  it("creates with billingDate when provided", async () => {
    vi.mocked(prisma.billingPeriod.create).mockResolvedValue({
      id: "bp-new",
      billingDate: new Date("2026-03-01"),
    } as any);

    const request = new Request("http://localhost/api/billing-periods", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        propertyId: "p1",
        startDate: "2025-01-01",
        endDate: "2025-12-31",
        billingDate: "2026-03-01",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(201);

    const createCall = vi.mocked(prisma.billingPeriod.create).mock.calls[0][0];
    expect(createCall.data.billingDate).toBeInstanceOf(Date);
  });

  it("rejects overlapping billing periods", async () => {
    vi.mocked(prisma.billingPeriod.findFirst).mockResolvedValue({
      id: "existing",
      startDate: new Date("2025-01-01"),
      endDate: new Date("2025-12-31"),
    } as any);

    const request = new Request("http://localhost/api/billing-periods", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        propertyId: "p1",
        startDate: "2025-06-01",
        endDate: "2026-05-31",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(409);
    const data = await response.json();
    expect(data.error).toContain("Überlappender");
  });

  it("rejects end date before start date", async () => {
    const request = new Request("http://localhost/api/billing-periods", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        propertyId: "p1",
        startDate: "2025-12-31",
        endDate: "2025-01-01",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(400);
    const data = await response.json();
    expect(data.error).toContain("Enddatum");
  });

  it("rejects property with no units", async () => {
    vi.mocked(prisma.property.findUnique).mockResolvedValue({
      id: "p1",
      _count: { units: 0 },
    } as any);

    const request = new Request("http://localhost/api/billing-periods", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        propertyId: "p1",
        startDate: "2025-01-01",
        endDate: "2025-12-31",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(400);
    const data = await response.json();
    expect(data.error).toContain("keine Wohnungen");
  });
});
