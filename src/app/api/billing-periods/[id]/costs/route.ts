import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const costs = await prisma.cost.findMany({
      where: { billingPeriodId: id },
      include: {
        costCategory: true,
      },
    });

    return jsonOk(costs);
  });
}

export function POST(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const body = await request.json();
    const { costCategoryId, totalAmount, unitAmount } = body;

    const cost = await prisma.cost.upsert({
      where: {
        billingPeriodId_costCategoryId: {
          billingPeriodId: id,
          costCategoryId,
        },
      },
      update: {
        totalAmount,
        unitAmount,
        reviewed: true,
      },
      create: {
        billingPeriodId: id,
        costCategoryId,
        totalAmount,
        unitAmount,
        reviewed: true,
      },
    });

    return jsonCreated(cost);
  });
}
