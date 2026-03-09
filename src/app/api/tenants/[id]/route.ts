import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk } from "@/lib/api-utils";

export function PUT(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const body = await request.json();
    const {
      unitId,
      salutation,
      firstName,
      lastName,
      salutation2,
      firstName2,
      lastName2,
      moveInDate,
      moveOutDate,
    } = body;

    const tenant = await prisma.tenant.update({
      where: { id },
      data: {
        unitId,
        salutation,
        firstName,
        lastName,
        salutation2,
        firstName2,
        lastName2,
        moveInDate: moveInDate ? new Date(moveInDate) : undefined,
        moveOutDate: moveOutDate ? new Date(moveOutDate) : null,
      },
    });

    return jsonOk(tenant);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.tenant.delete({
      where: { id },
    });

    return jsonOk({ success: true });
  });
}
