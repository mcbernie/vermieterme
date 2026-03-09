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
        units: {
          include: {
            tenants: true,
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
    const { street, zip, city, totalShares } = body;

    const property = await prisma.property.update({
      where: { id },
      data: {
        street,
        zip,
        city,
        totalShares,
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
