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
    const { name, distributionKey, sortOrder } = body;

    const category = await prisma.costCategory.update({
      where: { id },
      data: {
        name,
        distributionKey,
        sortOrder,
      },
    });

    return jsonOk(category);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.costCategory.delete({
      where: { id },
    });

    return jsonOk({ success: true });
  });
}
