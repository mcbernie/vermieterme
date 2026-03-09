import { describe, it, expect, vi, beforeEach } from "vitest";
import bcrypt from "bcryptjs";

vi.mock("@/lib/prisma", () => ({
  prisma: {
    user: {
      findUnique: vi.fn(),
      update: vi.fn(),
    },
  },
}));

vi.mock("@/lib/auth", () => ({
  auth: vi.fn(),
}));

import { GET, PUT } from "@/app/api/profile/route";
import { prisma } from "@/lib/prisma";
import { auth } from "@/lib/auth";

describe("GET /api/profile", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns 401 when not authenticated", async () => {
    vi.mocked(auth).mockResolvedValue(null as any);

    const response = await GET();
    expect(response.status).toBe(401);
  });

  it("returns user profile when authenticated", async () => {
    vi.mocked(auth).mockResolvedValue({
      user: { id: "user1" },
    } as any);

    vi.mocked(prisma.user.findUnique).mockResolvedValue({
      id: "user1",
      name: "Admin",
      email: "admin@test.local",
      image: null,
    } as any);

    const response = await GET();
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data.name).toBe("Admin");
    expect(data.email).toBe("admin@test.local");
  });

  it("returns 404 when user not found", async () => {
    vi.mocked(auth).mockResolvedValue({
      user: { id: "nonexistent" },
    } as any);

    vi.mocked(prisma.user.findUnique).mockResolvedValue(null);

    const response = await GET();
    expect(response.status).toBe(404);
  });
});

describe("PUT /api/profile", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("updates user name", async () => {
    vi.mocked(auth).mockResolvedValue({
      user: { id: "user1" },
    } as any);

    vi.mocked(prisma.user.update).mockResolvedValue({
      id: "user1",
      name: "Neuer Name",
      email: "admin@test.local",
    } as any);

    const request = new Request("http://localhost/api/profile", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: "Neuer Name" }),
    });

    const response = await PUT(request);
    const data = await response.json();

    expect(response.status).toBe(200);
    expect(data.name).toBe("Neuer Name");
  });

  it("requires current password to change password", async () => {
    vi.mocked(auth).mockResolvedValue({
      user: { id: "user1" },
    } as any);

    const request = new Request("http://localhost/api/profile", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ newPassword: "newpass123" }),
    });

    const response = await PUT(request);
    expect(response.status).toBe(400);
  });

  it("rejects wrong current password", async () => {
    vi.mocked(auth).mockResolvedValue({
      user: { id: "user1" },
    } as any);

    const hashedPw = await bcrypt.hash("correct", 12);
    vi.mocked(prisma.user.findUnique).mockResolvedValue({
      id: "user1",
      email: "user@test.local",
      password: hashedPw,
    } as any);

    const request = new Request("http://localhost/api/profile", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        currentPassword: "wrong",
        newPassword: "newpass123",
      }),
    });

    const response = await PUT(request);
    expect(response.status).toBe(400);
  });

  it("changes password with correct current password", async () => {
    vi.mocked(auth).mockResolvedValue({
      user: { id: "user1" },
    } as any);

    const hashedPw = await bcrypt.hash("oldpass", 12);
    vi.mocked(prisma.user.findUnique).mockResolvedValue({
      id: "user1",
      email: "user@test.local",
      password: hashedPw,
    } as any);

    vi.mocked(prisma.user.update).mockResolvedValue({
      id: "user1",
      name: null,
      email: "user@test.local",
    } as any);

    const request = new Request("http://localhost/api/profile", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        currentPassword: "oldpass",
        newPassword: "newpass123",
      }),
    });

    const response = await PUT(request);
    expect(response.status).toBe(200);

    const updateCall = vi.mocked(prisma.user.update).mock.calls[0][0];
    expect(updateCall.data.password).toBeDefined();
    expect(updateCall.data.password).not.toBe("newpass123");
  });
});
