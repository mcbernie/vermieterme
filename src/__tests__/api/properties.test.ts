import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/auth", () => ({
  auth: vi.fn().mockResolvedValue({ user: { id: "test-user" } }),
}));

vi.mock("@/lib/prisma", () => ({
  prisma: {
    property: {
      findMany: vi.fn(),
      create: vi.fn(),
    },
  },
}));

import { GET, POST } from "@/app/api/properties/route";
import { prisma } from "@/lib/prisma";

describe("GET /api/properties", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns all properties with unit count", async () => {
    const mockProperties = [
      {
        id: "p1",
        street: "Hamburger Str. 277",
        zip: "28205",
        city: "Bremen",
        totalShares: 100,
        _count: { units: 2 },
      },
    ];

    vi.mocked(prisma.property.findMany).mockResolvedValue(
      mockProperties as any
    );

    const response = await GET();
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data).toHaveLength(1);
    expect(data[0].street).toBe("Hamburger Str. 277");
    expect(data[0]._count.units).toBe(2);
  });

  it("returns empty array when no properties exist", async () => {
    vi.mocked(prisma.property.findMany).mockResolvedValue([]);

    const response = await GET();
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data).toEqual([]);
  });
});

describe("POST /api/properties", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("creates a property with all fields", async () => {
    const mockProperty = {
      id: "p-new",
      street: "Neue Str. 1",
      zip: "28195",
      city: "Bremen",
      totalShares: 100,
    };

    vi.mocked(prisma.property.create).mockResolvedValue(mockProperty as any);

    const request = new Request("http://localhost/api/properties", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        street: "Neue Str. 1",
        zip: "28195",
        city: "Bremen",
        totalShares: 100,
      }),
    });

    const response = await POST(request);
    const data = await response.json();

    expect(response.status).toBe(201);
    expect(data.street).toBe("Neue Str. 1");
  });
});
