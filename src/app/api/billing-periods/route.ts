import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const billingPeriods = await prisma.billingPeriod.findMany({
      include: {
        property: true,
        costs: {
          include: { costCategory: true },
        },
        prepayments: true,
        _count: {
          select: { costs: true },
        },
      },
      orderBy: { startDate: "desc" },
    });

    return jsonOk(billingPeriods);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { propertyId, startDate, endDate, billingDate, copyFromId } = body;

    const billingPeriod = await prisma.billingPeriod.create({
      data: {
        propertyId,
        startDate: new Date(startDate),
        endDate: new Date(endDate),
        billingDate: billingDate ? new Date(billingDate) : null,
        copiedFromId: copyFromId || null,
      },
    });

    // Copy costs and prepayments from source period
    if (copyFromId) {
      const source = await prisma.billingPeriod.findUnique({
        where: { id: copyFromId },
        include: { costs: true, prepayments: true },
      });

      if (source) {
        if (source.costs.length > 0) {
          await prisma.cost.createMany({
            data: source.costs.map((c) => ({
              billingPeriodId: billingPeriod.id,
              costCategoryId: c.costCategoryId,
              totalAmount: c.totalAmount,
              unitAmount: c.unitAmount,
              reviewed: false,
            })),
          });
        }

        if (source.prepayments.length > 0) {
          await prisma.prepayment.createMany({
            data: source.prepayments.map((p) => ({
              billingPeriodId: billingPeriod.id,
              unitId: p.unitId,
              monthlyAmount: p.monthlyAmount,
              reviewed: false,
            })),
          });
        }
      }
    }

    return jsonCreated(billingPeriod);
  });
}
