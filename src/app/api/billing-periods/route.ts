import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated, ApiError } from "@/lib/api-utils";

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

    if (!propertyId || !startDate || !endDate) {
      throw new ApiError("Objekt, Beginn und Ende sind erforderlich", 400);
    }

    const start = new Date(startDate);
    const end = new Date(endDate);

    if (end <= start) {
      throw new ApiError("Das Enddatum muss nach dem Beginndatum liegen", 400);
    }

    // Check for overlapping billing periods for the same property
    const overlapping = await prisma.billingPeriod.findFirst({
      where: {
        propertyId,
        AND: [
          { startDate: { lt: end } },
          { endDate: { gt: start } },
        ],
      },
    });

    if (overlapping) {
      const overlapStart = new Date(overlapping.startDate).toLocaleDateString("de-DE");
      const overlapEnd = new Date(overlapping.endDate).toLocaleDateString("de-DE");
      throw new ApiError(
        `Überlappender Abrechnungszeitraum existiert bereits: ${overlapStart} – ${overlapEnd}`,
        409
      );
    }

    // Validate property exists and has units
    const property = await prisma.property.findUnique({
      where: { id: propertyId },
      include: { _count: { select: { units: true } } },
    });

    if (!property) {
      throw new ApiError("Objekt nicht gefunden", 404);
    }

    if (property._count.units === 0) {
      throw new ApiError("Das Objekt hat keine Wohnungen. Bitte legen Sie zuerst Wohnungen an.", 400);
    }

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
