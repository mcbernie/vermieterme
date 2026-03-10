import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const rentChanges = await prisma.rentChange.findMany({
      include: {
        unit: {
          include: { property: true },
        },
      },
      orderBy: { effectiveDate: "desc" },
    });
    return jsonOk(rentChanges);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { unitId, type, amount, effectiveDate, reason } = body;

    const rentChange = await prisma.rentChange.create({
      data: {
        unitId,
        type,
        amount,
        effectiveDate: new Date(effectiveDate),
        reason: reason || null,
      },
    });

    return jsonCreated(rentChange);
  });
}
