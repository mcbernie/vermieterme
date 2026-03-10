import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const prepayments = await prisma.prepayment.findMany({
      where: { billingPeriodId: id },
      include: {
        unit: true,
      },
    });

    return jsonOk(prepayments);
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
    const { unitId, monthlyAmount } = body;

    const prepayment = await prisma.prepayment.upsert({
      where: {
        billingPeriodId_unitId: {
          billingPeriodId: id,
          unitId,
        },
      },
      update: {
        monthlyAmount,
        reviewed: true,
      },
      create: {
        billingPeriodId: id,
        unitId,
        monthlyAmount,
        reviewed: true,
      },
    });

    return jsonCreated(prepayment);
  });
}
