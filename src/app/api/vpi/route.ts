import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated, ApiError } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const entries = await prisma.vpiEntry.findMany({
      orderBy: [{ baseYear: "desc" }, { year: "desc" }, { month: "desc" }],
    });
    return jsonOk(entries);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { year, month, value, baseYear } = body;

    if (!year || !month || value == null || !baseYear) {
      throw new ApiError("Alle Felder sind erforderlich", 400);
    }

    const entry = await prisma.vpiEntry.upsert({
      where: {
        year_month_baseYear: { year, month, baseYear },
      },
      update: { value },
      create: { year, month, value, baseYear },
    });

    return jsonCreated(entry);
  });
}
