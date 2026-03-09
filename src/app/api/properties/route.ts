import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const properties = await prisma.property.findMany({
      include: {
        _count: {
          select: { units: true },
        },
      },
      orderBy: { createdAt: "desc" },
    });
    return jsonOk(properties);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { street, zip, city, totalShares } = body;

    const property = await prisma.property.create({
      data: {
        street,
        zip,
        city,
        totalShares,
      },
    });

    return jsonCreated(property);
  });
}
