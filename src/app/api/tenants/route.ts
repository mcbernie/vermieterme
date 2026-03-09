import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const { searchParams } = new URL(request.url);
    const unitId = searchParams.get("unitId");

    const tenants = await prisma.tenant.findMany({
      where: unitId ? { unitId } : undefined,
      include: {
        unit: {
          include: {
            property: true,
          },
        },
      },
      orderBy: { createdAt: "desc" },
    });

    return jsonOk(tenants);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const {
      unitId,
      salutation,
      firstName,
      lastName,
      salutation2,
      firstName2,
      lastName2,
      moveInDate,
      moveOutDate,
    } = body;

    const tenant = await prisma.tenant.create({
      data: {
        unitId,
        salutation,
        firstName,
        lastName,
        salutation2,
        firstName2,
        lastName2,
        moveInDate: new Date(moveInDate),
        moveOutDate: moveOutDate ? new Date(moveOutDate) : null,
      },
    });

    return jsonCreated(tenant);
  });
}
