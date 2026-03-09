import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const categories = await prisma.costCategory.findMany({
      orderBy: { sortOrder: "asc" },
    });

    return jsonOk(categories);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { name, distributionKey, sortOrder } = body;

    const category = await prisma.costCategory.create({
      data: {
        name,
        distributionKey,
        sortOrder,
      },
    });

    return jsonCreated(category);
  });
}
