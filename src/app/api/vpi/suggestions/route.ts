import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk } from "@/lib/api-utils";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();

    // Get all active index-lease tenants
    const tenants = await prisma.tenant.findMany({
      where: {
        leaseType: "index",
        moveOutDate: null,
        indexBaseYear: { not: null },
        indexReferenceValue: { not: null },
      },
      include: {
        unit: {
          include: { property: true },
        },
      },
    });

    if (tenants.length === 0) {
      return jsonOk([]);
    }

    // Get latest rent change per unit (type = "rent")
    const unitIds = [...new Set(tenants.map((t) => t.unitId))];
    const rentChanges = await prisma.rentChange.findMany({
      where: { unitId: { in: unitIds }, type: "rent" },
      orderBy: { effectiveDate: "desc" },
    });

    // Get all VPI entries
    const vpiEntries = await prisma.vpiEntry.findMany({
      orderBy: [{ year: "desc" }, { month: "desc" }],
    });

    const suggestions = tenants
      .map((tenant) => {
        const baseYear = tenant.indexBaseYear!;
        const refValue = tenant.indexReferenceValue!;
        const refDate = tenant.indexReferenceDate;

        // Find latest VPI for same base year
        const latestVpi = vpiEntries.find((e) => e.baseYear === baseYear);
        if (!latestVpi) return null;

        // Find current rent
        const latestRent = rentChanges.find(
          (rc) => rc.unitId === tenant.unitId
        );
        const currentRent = latestRent?.amount ?? 0;
        if (currentRent === 0) return null;

        // Calculate percentage change
        const percentageChange = ((latestVpi.value - refValue) / refValue) * 100;
        const suggestedRent =
          Math.round(currentRent * (latestVpi.value / refValue) * 100) / 100;

        // Calculate months since last adjustment
        const referenceDate = refDate ?? tenant.moveInDate;
        const now = new Date();
        const refD = new Date(referenceDate);
        const monthsSince =
          (now.getFullYear() - refD.getFullYear()) * 12 +
          (now.getMonth() - refD.getMonth());

        const eligible = monthsSince >= tenant.indexMinMonths;

        return {
          tenantId: tenant.id,
          unitId: tenant.unitId,
          tenantName: `${tenant.firstName} ${tenant.lastName}`,
          unitName: tenant.unit.name,
          propertyAddress: `${tenant.unit.property.street}, ${tenant.unit.property.city}`,
          leaseType: "index" as const,
          currentRent,
          referenceVpi: refValue,
          currentVpi: latestVpi.value,
          percentageChange: Math.round(percentageChange * 100) / 100,
          suggestedRent,
          monthsSinceLastAdjustment: monthsSince,
          minMonths: tenant.indexMinMonths,
          eligible,
          baseYear,
        };
      })
      .filter(Boolean);

    return jsonOk(suggestions);
  });
}
