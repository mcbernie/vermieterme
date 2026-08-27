import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

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

export function GET(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const { searchParams } = new URL(request.url);
    const propertyId = searchParams.get("propertyId");

    const units = await prisma.unit.findMany({
      where: propertyId ? { propertyId } : undefined,
      include: {
        allocationKeys: {
          orderBy: { key: "asc" },
        },
      },
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
    const allocationKeys = normalizeAllocationKeys(body.allocationKeys);

    const unit = await prisma.unit.create({
      data: {
        propertyId,
        name,
        floor,
        shares,
        allocationKeys: {
          create: allocationKeys,
        },
      },
      include: {
        allocationKeys: true,
      },
    });

    return jsonCreated(unit);
  });
}
