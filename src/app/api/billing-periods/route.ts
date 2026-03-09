import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const billingPeriods = await prisma.billingPeriod.findMany({
      include: {
        property: true,
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
    const { propertyId, startDate, endDate, billingDate } = body;

    const billingPeriod = await prisma.billingPeriod.create({
      data: {
        propertyId,
        startDate: new Date(startDate),
        endDate: new Date(endDate),
        billingDate: billingDate ? new Date(billingDate) : null,
      },
    });

    return jsonCreated(billingPeriod);
  });
}
