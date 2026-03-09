import { apiHandler, requireAuth, jsonOk, ApiError } from "@/lib/api-utils";
import { hashPassword, verifyPassword } from "@/lib/password";
import { prisma } from "@/lib/prisma";

export function GET() {
  return apiHandler(async () => {
    const session = await requireAuth();

    const user = await prisma.user.findUnique({
      where: { id: session.user.id },
      select: {
        id: true,
        name: true,
        email: true,
        image: true,
      },
    });

    if (!user) {
      throw new ApiError("Benutzer nicht gefunden", 404);
    }

    return jsonOk(user);
  });
}

export function PUT(request: Request) {
  return apiHandler(async () => {
    const session = await requireAuth();

    const body = await request.json();
    const { name, currentPassword, newPassword } = body;

    const updateData: { name?: string; password?: string } = {};

    if (name !== undefined) {
      updateData.name = name;
    }

    if (newPassword) {
      if (!currentPassword) {
        throw new ApiError("Aktuelles Passwort erforderlich", 400);
      }

      // Verify current password
      const user = await prisma.user.findUnique({
        where: { id: session.user.id },
      });

      if (!user) {
        throw new ApiError("Benutzer nicht gefunden", 404);
      }

      // Check against env admin password or hashed DB password
      const isAdmin =
        user.email === process.env.ADMIN_EMAIL &&
        currentPassword === process.env.ADMIN_PASSWORD;
      const isDbMatch =
        user.password && (await verifyPassword(currentPassword, user.password));

      if (!isAdmin && !isDbMatch) {
        throw new ApiError("Aktuelles Passwort ist falsch", 400);
      }

      updateData.password = await hashPassword(newPassword);
    }

    const updated = await prisma.user.update({
      where: { id: session.user.id },
      data: updateData,
      select: { id: true, name: true, email: true },
    });

    return jsonOk(updated);
  });
}
