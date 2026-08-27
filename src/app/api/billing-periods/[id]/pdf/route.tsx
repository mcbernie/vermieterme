import { prisma } from "@/lib/prisma";
import { auth } from "@/lib/auth";
import React from "react";
import { renderToBuffer } from "@react-pdf/renderer";
import { getDefaultConfig } from "@/lib/pdf-template";
import { calculateAllocationAmount, calculateMEAAmount } from "@/lib/billing";
import type { PdfTemplateConfig } from "@/types/pdf-template";
import { BillingPdf, daysBetween } from "@/lib/billing-pdf";

// --- Route Handler ---

export async function GET(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  try {
    const session = await auth();
    if (!session?.user?.id) {
      return new Response(JSON.stringify({ error: "Nicht angemeldet" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      });
    }

    const { id } = await paramsPromise;

    // Load PDF template config
    const templateRow = await prisma.pdfTemplate.findFirst({
      orderBy: { updatedAt: "desc" },
    });
    const templateConfig: PdfTemplateConfig = templateRow
      ? JSON.parse(templateRow.config)
      : getDefaultConfig();

    // Fetch billing period with all related data
    const billingPeriod = await prisma.billingPeriod.findUnique({
      where: { id },
      include: {
        property: {
          include: {
            landlord: true,
            units: {
              include: {
                tenants: true,
                allocationKeys: {
                  orderBy: { key: "asc" },
                },
                prepayments: {
                  where: { billingPeriodId: id },
                },
              },
            },
          },
        },
        costs: {
          include: {
            costCategory: true,
          },
          orderBy: {
            costCategory: {
              sortOrder: "asc",
            },
          },
        },
      },
    });

    if (!billingPeriod) {
      return new Response(
        JSON.stringify({ error: "Billing period not found" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const landlord =
      billingPeriod.property.landlord ?? (await prisma.landlordInfo.findFirst());

    if (!landlord) {
      return new Response(
        JSON.stringify({ error: "Landlord info not configured" }),
        { status: 400, headers: { "Content-Type": "application/json" } }
      );
    }

    const property = billingPeriod.property;
    const startDate = new Date(billingPeriod.startDate);
    const endDate = new Date(billingPeriod.endDate);

    let targetUnit = null;
    let activeTenant = null;

    for (const unit of property.units) {
      const t = unit.tenants.find(
        (t: { moveInDate: Date; moveOutDate: Date | null }) => {
          const moveIn = new Date(t.moveInDate);
          const moveOut = t.moveOutDate ? new Date(t.moveOutDate) : null;
          return moveIn <= endDate && (moveOut === null || moveOut >= startDate);
        }
      );

      if (t) {
        targetUnit = unit;
        activeTenant = t;
        break;
      }
    }

    if (!targetUnit || !activeTenant) {
      return new Response(
        JSON.stringify({
          error: "No active tenant found for this billing period",
        }),
        { status: 400, headers: { "Content-Type": "application/json" } }
      );
    }

    const costs = billingPeriod.costs
      .filter(
        (cost: { enabled: boolean; costCategory: { apportionable?: boolean } }) =>
          cost.enabled !== false && cost.costCategory.apportionable !== false
      )
      .map(
        (cost: {
          costCategory: {
            name: string;
            distributionKey: string;
            requiresAttachment: boolean;
          };
          totalAmount: number;
          unitAmount: number | null;
          tenantAmountOverride: number | null;
          distributionKeyOverride: string | null;
        }) => {
          const distributionKey =
            cost.distributionKeyOverride ?? cost.costCategory.distributionKey;
          const allocationKey = targetUnit.allocationKeys?.find(
            (key: { key: string }) => key.key === distributionKey
          );
          const unitAmount =
            property.accountingMode === "weg" &&
            cost.tenantAmountOverride !== null
              ? cost.tenantAmountOverride
              : distributionKey === "MEA"
                ? calculateMEAAmount(
                    cost.totalAmount,
                    targetUnit.shares,
                    property.totalShares
                  )
                : allocationKey
                  ? calculateAllocationAmount(
                      cost.totalAmount,
                      allocationKey.unitValue,
                      allocationKey.totalValue
                    )
                  : cost.unitAmount ?? 0;

          return {
            categoryName: cost.costCategory.name,
            requiresAttachment: cost.costCategory.requiresAttachment,
            // Per-period override takes precedence over the category default.
            distributionKey,
            totalAmount: cost.totalAmount,
            unitAmount,
          };
        }
      );

    const totalCosts = costs.reduce(
      (sum: number, c: { totalAmount: number }) => sum + c.totalAmount,
      0
    );
    const totalUnitCosts = costs.reduce(
      (sum: number, c: { unitAmount: number }) => sum + c.unitAmount,
      0
    );

    const prepayment = targetUnit.prepayments.find(
      (p: { billingPeriodId: string }) => p.billingPeriodId === id
    );
    const months = daysBetween(startDate, endDate) / 30.44;
    const totalPrepayment = prepayment
      ? prepayment.monthlyAmount * Math.round(months)
      : 0;

    const year = startDate.getFullYear();

    const buffer = await renderToBuffer(
      <BillingPdf
        landlord={landlord}
        property={property}
        billingPeriod={billingPeriod}
        unit={targetUnit}
        tenant={activeTenant}
        costs={costs}
        totalCosts={totalCosts}
        totalUnitCosts={totalUnitCosts}
        totalPrepayment={totalPrepayment}
        templateConfig={templateConfig}
      />
    );

    return new Response(new Uint8Array(buffer), {
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `attachment; filename="Betriebskostenabrechnung-${year}.pdf"`,
      },
    });
  } catch (error) {
    console.error("Failed to generate billing PDF:", error);
    return new Response(
      JSON.stringify({ error: "Failed to generate PDF" }),
      { status: 500, headers: { "Content-Type": "application/json" } }
    );
  }
}
