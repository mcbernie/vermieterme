import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, ApiError, jsonOk } from "@/lib/api-utils";

export function PUT(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const body = await request.json();

    const landlord = await prisma.landlordInfo.update({
      where: { id },
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

    return jsonOk(landlord);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;

    const propertyCount = await prisma.property.count({
      where: { landlordId: id },
    });

    if (propertyCount > 0) {
      throw new ApiError(
        "Dieser Vermieter ist noch einem Objekt zugeordnet.",
        409
      );
    }

    await prisma.landlordInfo.delete({ where: { id } });
    return jsonOk({ success: true });
  });
}
