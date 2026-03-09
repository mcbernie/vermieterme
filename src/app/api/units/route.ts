import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const { searchParams } = new URL(request.url);
    const propertyId = searchParams.get("propertyId");

    const units = await prisma.unit.findMany({
      where: propertyId ? { propertyId } : undefined,
      orderBy: { createdAt: "desc" },
    });

    return jsonOk(units);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { propertyId, name, floor, shares } = body;

    const unit = await prisma.unit.create({
      data: {
        propertyId,
        name,
        floor,
        shares,
      },
    });

    return jsonCreated(unit);
  });
}
