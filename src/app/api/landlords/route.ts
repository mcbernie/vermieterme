import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonCreated, jsonOk } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();
    const landlords = await prisma.landlordInfo.findMany({
      orderBy: { createdAt: "asc" },
    });

    return jsonOk(landlords);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();

    const landlord = await prisma.landlordInfo.create({
      data: {
        name: body.name,
        street: body.street,
        zip: body.zip,
        city: body.city,
        phone: body.phone || null,
        email: body.email || null,
        bankName: body.bankName || null,
        iban: body.iban || null,
        accountHolder: body.accountHolder || null,
      },
    });

    return jsonCreated(landlord);
  });
}
