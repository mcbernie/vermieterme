import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, ApiError, jsonOk } from "@/lib/api-utils";

export function GET(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const billingPeriod = await prisma.billingPeriod.findUnique({
      where: { id },
      include: {
        property: {
          include: {
            units: {
              include: {
                tenants: true,
              },
            },
          },
        },
        costs: {
          include: {
            costCategory: true,
          },
        },
        prepayments: {
          include: {
            unit: true,
          },
        },
      },
    });

    if (!billingPeriod) {
      throw new ApiError("Billing period not found", 404);
    }

    return jsonOk(billingPeriod);
  });
}

export function PUT(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const body = await request.json();
    const { propertyId, startDate, endDate, billingDate, sentDate, paidDate } = body;

    const billingPeriod = await prisma.billingPeriod.update({
      where: { id },
      data: {
        propertyId,
        startDate: startDate ? new Date(startDate) : undefined,
        endDate: endDate ? new Date(endDate) : undefined,
        billingDate: billingDate !== undefined ? (billingDate ? new Date(billingDate) : null) : undefined,
        sentDate: sentDate !== undefined ? (sentDate ? new Date(sentDate) : null) : undefined,
        paidDate: paidDate !== undefined ? (paidDate ? new Date(paidDate) : null) : undefined,
      },
    });

    return jsonOk(billingPeriod);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.billingPeriod.delete({
      where: { id },
    });

    return jsonOk({ success: true });
  });
}
