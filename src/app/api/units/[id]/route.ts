import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk } from "@/lib/api-utils";

interface AllocationKeyInput {
  key: string;
  unitValue: number;
  totalValue: number;
}

function normalizeAllocationKeys(input: unknown): AllocationKeyInput[] {
  if (!Array.isArray(input)) return [];

  return input
    .map((item) => {
      if (!item || typeof item !== "object") return null;
      const data = item as Partial<AllocationKeyInput>;
      const key = String(data.key ?? "").trim();
      const unitValue = Number(data.unitValue);
      const totalValue = Number(data.totalValue);

      if (!key || !Number.isFinite(unitValue) || !Number.isFinite(totalValue)) {
        return null;
      }

      return { key, unitValue, totalValue };
    })
    .filter((item): item is AllocationKeyInput => item !== null);
}

export function PUT(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const body = await request.json();
    const { propertyId, name, floor, shares } = body;
    const allocationKeys = normalizeAllocationKeys(body.allocationKeys);

    const unit = await prisma.$transaction(async (tx) => {
      const updated = await tx.unit.update({
        where: { id },
        data: {
          propertyId,
          name,
          floor,
          shares,
        },
      });

      await tx.unitAllocationKey.deleteMany({
        where: { unitId: id },
      });

      if (allocationKeys.length > 0) {
        await tx.unitAllocationKey.createMany({
          data: allocationKeys.map((key) => ({
            ...key,
            unitId: id,
          })),
        });
      }

      return tx.unit.findUnique({
        where: { id: updated.id },
        include: { allocationKeys: true },
      });
    });

    return jsonOk(unit);
  });
}

export function DELETE(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    await prisma.unit.delete({
      where: { id },
    });

    return jsonOk({ success: true });
  });
}
