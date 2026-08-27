import { apiHandler, ApiError, jsonOk } from "@/lib/api-utils";
import { prisma } from "@/lib/prisma";
import { requireTenantAuth } from "@/lib/tenant-auth";
import {
  calculateAllocationAmount,
  calculateMEAAmount,
  getMonthsInPeriod,
} from "@/lib/billing";
import { NextRequest } from "next/server";

export function GET(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    const { tenantId, unitId } = await requireTenantAuth();
    const { id } = await params;

    const tenant = await prisma.tenant.findUnique({
      where: { id: tenantId },
      include: {
        unit: {
          include: {
            property: true,
            allocationKeys: true,
          },
        },
      },
    });

    if (!tenant) {
      throw new ApiError("Mieter nicht gefunden", 404);
    }

    const bp = await prisma.billingPeriod.findUnique({
      where: { id },
      include: {
        property: true,
        costs: { include: { costCategory: true } },
        prepayments: { where: { unitId } },
        documents: true,
      },
    });

    if (!bp || bp.propertyId !== tenant.unit.propertyId) {
      throw new ApiError("Abrechnungszeitraum nicht gefunden", 404);
    }

    const unit = tenant.unit;
    const property = unit.property;

    const visibleCosts = bp.costs.filter(
      (cost) =>
        cost.enabled !== false && cost.costCategory.apportionable !== false
    );

    const costs = visibleCosts.map((cost) => {
      // Per-period override takes precedence over the category default.
      const distributionKey =
        cost.distributionKeyOverride ?? cost.costCategory.distributionKey;
      let unitAmount =
        property.accountingMode === "weg" &&
        cost.tenantAmountOverride !== null
          ? cost.tenantAmountOverride
          : cost.unitAmount;
      if (unitAmount == null && distributionKey.toLowerCase() === "mea") {
        unitAmount = calculateMEAAmount(cost.totalAmount, unit.shares, property.totalShares);
      } else if (
        distributionKey !== "laut Bescheid" &&
        distributionKey !== "siehe Anlage"
      ) {
        const allocationKey = unit.allocationKeys.find(
          (key) => key.key === distributionKey
        );
        if (allocationKey) {
          unitAmount = calculateAllocationAmount(
            cost.totalAmount,
            allocationKey.unitValue,
            allocationKey.totalValue
          );
        }
      }

      return {
        id: cost.id,
        category: cost.costCategory.name,
        requiresAttachment: cost.costCategory.requiresAttachment,
        distributionKey,
        totalAmount: cost.totalAmount,
        unitAmount: unitAmount ?? 0,
      };
    });

    const months = getMonthsInPeriod(
      bp.startDate.toISOString(),
      bp.endDate.toISOString()
    );
    const totalCosts = visibleCosts.reduce(
      (sum, cost) => sum + cost.totalAmount,
      0
    );
    const totalUnitCosts = costs.reduce(
      (sum, cost) => sum + cost.unitAmount,
      0
    );
    const totalPrepayment = bp.prepayments.reduce(
      (sum, prepayment) => sum + prepayment.monthlyAmount * months,
      0
    );

    const documents = bp.documents.map((doc) => ({
      id: doc.id,
      originalName: doc.originalName,
      mimeType: doc.mimeType,
      size: doc.size,
      category: doc.category,
      createdAt: doc.createdAt,
    }));

    return jsonOk({
      id: bp.id,
      startDate: bp.startDate,
      endDate: bp.endDate,
      billingDate: bp.billingDate,
      sentDate: bp.sentDate,
      paidDate: bp.paidDate,
      property: {
        street: bp.property.street,
        zip: bp.property.zip,
        city: bp.property.city,
      },
      costs,
      prepayments: bp.prepayments.map((p) => ({
        id: p.id,
        monthlyAmount: p.monthlyAmount,
      })),
      totals: {
        totalCosts,
        totalUnitCosts,
        totalPrepayment,
        difference: totalPrepayment - totalUnitCosts,
      },
      documents,
    });
  });
}
