import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const landlord = await prisma.landlordInfo.findFirst();

    return jsonOk(landlord);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();

    const existing = await prisma.landlordInfo.findFirst();

    let landlord;
    if (existing) {
      landlord = await prisma.landlordInfo.update({
        where: { id: existing.id },
        data: body,
      });
    } else {
      landlord = await prisma.landlordInfo.create({
        data: body,
      });
    }

    return jsonOk(landlord);
  });
}
