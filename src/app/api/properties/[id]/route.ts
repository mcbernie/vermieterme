import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, ApiError, jsonOk } from "@/lib/api-utils";

export function GET(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const property = await prisma.property.findUnique({
      where: { id },
      include: {
        landlord: true,
        units: {
          include: {
            tenants: true,
            allocationKeys: {
              orderBy: { key: "asc" },
            },
          },
        },
      },
    });

    if (!property) {
      throw new ApiError("Property not found", 404);
    }

    return jsonOk(property);
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
    const { street, zip, city, totalShares, landlordId, accountingMode } = body;

    const property = await prisma.property.update({
      where: { id },
      data: {
        street,
        zip,
        city,
        totalShares,
        landlordId: landlordId || null,
        accountingMode: accountingMode || "standard",
      },
    });

    return jsonOk(property);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.property.delete({
      where: { id },
    });

    return jsonOk({ success: true });
  });
}
