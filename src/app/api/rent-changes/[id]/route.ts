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
    const { unitId, type, amount, effectiveDate, reason } = body;

    const rentChange = await prisma.rentChange.update({
      where: { id },
      data: {
        unitId,
        type,
        amount,
        effectiveDate: effectiveDate ? new Date(effectiveDate) : undefined,
        reason: reason !== undefined ? reason || null : undefined,
      },
    });

    return jsonOk(rentChange);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.rentChange.delete({ where: { id } });
    return jsonOk({ success: true });
  });
}
