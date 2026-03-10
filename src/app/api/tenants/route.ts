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
      phone,
      email,
      bankName,
      iban,
      accountHolder,
      moveInDate,
      moveOutDate,
      leaseType,
      indexBaseYear,
      indexReferenceValue,
      indexReferenceDate,
      indexMinMonths,
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
        phone: phone || null,
        email: email || null,
        bankName: bankName || null,
        iban: iban || null,
        accountHolder: accountHolder || null,
        moveInDate: new Date(moveInDate),
        moveOutDate: moveOutDate ? new Date(moveOutDate) : null,
        leaseType: leaseType || "standard",
        indexBaseYear: indexBaseYear ?? null,
        indexReferenceValue: indexReferenceValue ?? null,
        indexReferenceDate: indexReferenceDate
          ? new Date(indexReferenceDate)
          : null,
        indexMinMonths: indexMinMonths ?? 12,
      },
    });

    return jsonCreated(tenant);
  });
}
