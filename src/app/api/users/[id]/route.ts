import { apiHandler, requireAuth, jsonOk, ApiError } from "@/lib/api-utils";
import { prisma } from "@/lib/prisma";

export function DELETE(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    const session = await requireAuth();

    const { id } = await params;

    if (id === session.user.id) {
      throw new ApiError("Sie können sich nicht selbst löschen", 400);
    }

    await prisma.user.delete({ where: { id } });
    return jsonOk({ success: true });
  });
}
