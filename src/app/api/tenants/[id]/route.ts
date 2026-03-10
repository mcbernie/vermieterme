import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk } from "@/lib/api-utils";

export function PUT(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
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

    const tenant = await prisma.tenant.update({
      where: { id },
      data: {
        unitId,
        salutation,
        firstName,
        lastName,
        salutation2,
        firstName2,
        lastName2,
        phone: phone !== undefined ? phone || null : undefined,
        email: email !== undefined ? email || null : undefined,
        bankName: bankName !== undefined ? bankName || null : undefined,
        iban: iban !== undefined ? iban || null : undefined,
        accountHolder:
          accountHolder !== undefined ? accountHolder || null : undefined,
        moveInDate: moveInDate ? new Date(moveInDate) : undefined,
        moveOutDate: moveOutDate ? new Date(moveOutDate) : null,
        leaseType: leaseType !== undefined ? leaseType : undefined,
        indexBaseYear: indexBaseYear !== undefined ? indexBaseYear : undefined,
        indexReferenceValue:
          indexReferenceValue !== undefined ? indexReferenceValue : undefined,
        indexReferenceDate:
          indexReferenceDate !== undefined
            ? indexReferenceDate
              ? new Date(indexReferenceDate)
              : null
            : undefined,
        indexMinMonths:
          indexMinMonths !== undefined ? indexMinMonths : undefined,
      },
    });

    return jsonOk(tenant);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.tenant.delete({
      where: { id },
    });

    return jsonOk({ success: true });
  });
}
