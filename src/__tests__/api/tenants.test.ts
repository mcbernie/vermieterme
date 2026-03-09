import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/auth", () => ({
  auth: vi.fn().mockResolvedValue({ user: { id: "test-user" } }),
}));

vi.mock("@/lib/prisma", () => ({
  prisma: {
    tenant: {
      findMany: vi.fn(),
      create: vi.fn(),
    },
  },
}));

import { GET, POST } from "@/app/api/tenants/route";
import { prisma } from "@/lib/prisma";

describe("GET /api/tenants", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns all tenants with unit and property", async () => {
    const mockTenants = [
      {
        id: "t1",
        salutation: "Herr",
        firstName: "Max",
        lastName: "Mustermann",
        moveInDate: new Date("2023-01-01"),
        moveOutDate: null,
        unit: {
          id: "u1",
          name: "Wohnung I",
          property: { street: "Teststr. 1", zip: "28195", city: "Bremen" },
        },
      },
    ];

    vi.mocked(prisma.tenant.findMany).mockResolvedValue(mockTenants as any);

    const request = new Request("http://localhost/api/tenants");
    const response = await GET(request);
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data).toHaveLength(1);
    expect(data[0].firstName).toBe("Max");
    expect(data[0].unit.property.city).toBe("Bremen");
  });

  it("filters by unitId when provided", async () => {
    vi.mocked(prisma.tenant.findMany).mockResolvedValue([]);

    const request = new Request("http://localhost/api/tenants?unitId=u1");
    await GET(request);

    expect(prisma.tenant.findMany).toHaveBeenCalledWith(
      expect.objectContaining({
        where: { unitId: "u1" },
      })
    );
  });

  it("returns all tenants when no unitId filter", async () => {
    vi.mocked(prisma.tenant.findMany).mockResolvedValue([]);

    const request = new Request("http://localhost/api/tenants");
    await GET(request);

    expect(prisma.tenant.findMany).toHaveBeenCalledWith(
      expect.objectContaining({
        where: undefined,
      })
    );
  });
});

describe("POST /api/tenants", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("creates a tenant with required fields", async () => {
    const mockTenant = {
      id: "t-new",
      unitId: "u1",
      salutation: "Frau",
      firstName: "Anna",
      lastName: "Schmidt",
      moveInDate: new Date("2024-04-01"),
      moveOutDate: null,
    };

    vi.mocked(prisma.tenant.create).mockResolvedValue(mockTenant as any);

    const request = new Request("http://localhost/api/tenants", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        unitId: "u1",
        salutation: "Frau",
        firstName: "Anna",
        lastName: "Schmidt",
        moveInDate: "2024-04-01",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(201);

    const createCall = vi.mocked(prisma.tenant.create).mock.calls[0][0];
    expect(createCall.data.firstName).toBe("Anna");
    expect(createCall.data.moveInDate).toBeInstanceOf(Date);
    expect(createCall.data.moveOutDate).toBeNull();
  });

  it("creates a tenant with moveOutDate", async () => {
    vi.mocked(prisma.tenant.create).mockResolvedValue({ id: "t-new" } as any);

    const request = new Request("http://localhost/api/tenants", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        unitId: "u1",
        salutation: "Herr",
        firstName: "Jan",
        lastName: "Müller",
        moveInDate: "2020-01-01",
        moveOutDate: "2024-12-31",
      }),
    });

    await POST(request);

    const createCall = vi.mocked(prisma.tenant.create).mock.calls[0][0];
    expect(createCall.data.moveOutDate).toBeInstanceOf(Date);
  });
});
