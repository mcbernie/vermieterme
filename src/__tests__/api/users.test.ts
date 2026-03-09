import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/prisma", () => ({
  prisma: {
    user: {
      findMany: vi.fn(),
      findUnique: vi.fn(),
      create: vi.fn(),
      delete: vi.fn(),
    },
  },
}));

vi.mock("@/lib/auth", () => ({
  auth: vi.fn(),
}));

import { GET, POST } from "@/app/api/users/route";
import { DELETE } from "@/app/api/users/[id]/route";
import { prisma } from "@/lib/prisma";
import { auth } from "@/lib/auth";

describe("GET /api/users", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns 401 when not authenticated", async () => {
    vi.mocked(auth).mockResolvedValue(null as any);
    const response = await GET();
    expect(response.status).toBe(401);
  });

  it("returns all users", async () => {
    vi.mocked(auth).mockResolvedValue({ user: { id: "u1" } } as any);
    vi.mocked(prisma.user.findMany).mockResolvedValue([
      { id: "u1", name: "Admin", email: "admin@test.local", image: null },
      { id: "u2", name: "User", email: "user@test.local", image: null },
    ] as any);

    const response = await GET();
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data).toHaveLength(2);
  });
});

describe("POST /api/users", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("creates a user with valid data", async () => {
    vi.mocked(auth).mockResolvedValue({ user: { id: "u1" } } as any);
    vi.mocked(prisma.user.findUnique).mockResolvedValue(null);
    vi.mocked(prisma.user.create).mockResolvedValue({
      id: "u-new",
      name: "Neuer Nutzer",
      email: "neu@test.local",
    } as any);

    const request = new Request("http://localhost/api/users", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: "Neuer Nutzer",
        email: "neu@test.local",
        password: "sicheres-passwort",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(201);

    const createCall = vi.mocked(prisma.user.create).mock.calls[0][0];
    expect(createCall.data!.email).toBe("neu@test.local");
    expect(createCall.data!.password).not.toBe("sicheres-passwort"); // hashed
  });

  it("rejects short password", async () => {
    vi.mocked(auth).mockResolvedValue({ user: { id: "u1" } } as any);

    const request = new Request("http://localhost/api/users", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: "neu@test.local",
        password: "kurz",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(400);
  });

  it("rejects duplicate email", async () => {
    vi.mocked(auth).mockResolvedValue({ user: { id: "u1" } } as any);
    vi.mocked(prisma.user.findUnique).mockResolvedValue({ id: "existing" } as any);

    const request = new Request("http://localhost/api/users", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: "existing@test.local",
        password: "sicheres-passwort",
      }),
    });

    const response = await POST(request);
    expect(response.status).toBe(409);
  });
});

describe("DELETE /api/users/:id", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("deletes a user", async () => {
    vi.mocked(auth).mockResolvedValue({ user: { id: "u1" } } as any);
    vi.mocked(prisma.user.delete).mockResolvedValue({} as any);

    const request = new Request("http://localhost/api/users/u2", {
      method: "DELETE",
    });

    const response = await DELETE(request, {
      params: Promise.resolve({ id: "u2" }),
    });
    expect(response.status).toBe(200);
  });

  it("prevents self-deletion", async () => {
    vi.mocked(auth).mockResolvedValue({ user: { id: "u1" } } as any);

    const request = new Request("http://localhost/api/users/u1", {
      method: "DELETE",
    });

    const response = await DELETE(request, {
      params: Promise.resolve({ id: "u1" }),
    });
    expect(response.status).toBe(400);
  });
});
