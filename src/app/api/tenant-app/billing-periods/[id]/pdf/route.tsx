import { prisma } from "@/lib/prisma";
import React from "react";
import { renderToBuffer } from "@react-pdf/renderer";
import { getDefaultConfig } from "@/lib/pdf-template";
import type { PdfTemplateConfig } from "@/types/pdf-template";
import { BillingPdf, daysBetween } from "@/lib/billing-pdf";
import { requireTenantAuth } from "@/lib/tenant-auth";
import { calculateAllocationAmount, calculateMEAAmount } from "@/lib/billing";
import { ApiError } from "@/lib/api-utils";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  try {
    const { tenantId, unitId } = await requireTenantAuth();
    const { id } = await params;

    const tenant = await prisma.tenant.findUnique({
      where: { id: tenantId },
      include: {
        unit: {
          include: {
            property: {
              include: {
                landlord: true,
              },
            },
            allocationKeys: true,
          },
        },
      },
    });

    if (!tenant) {
      return new Response(JSON.stringify({ error: "Mieter nicht gefunden" }), {
        status: 404,
        headers: { "Content-Type": "application/json" },
      });
    }

    const templateRow = await prisma.pdfTemplate.findFirst({
      orderBy: { updatedAt: "desc" },
    });
    const templateConfig: PdfTemplateConfig = templateRow
      ? JSON.parse(templateRow.config)
      : getDefaultConfig();

    const billingPeriod = await prisma.billingPeriod.findUnique({
      where: { id },
      include: {
        property: true,
        costs: {
          include: { costCategory: true },
          orderBy: { costCategory: { sortOrder: "asc" } },
        },
        prepayments: { where: { unitId } },
      },
    });

    if (!billingPeriod || billingPeriod.propertyId !== tenant.unit.propertyId) {
      return new Response(
        JSON.stringify({ error: "Abrechnungszeitraum nicht gefunden" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const landlord =
      tenant.unit.property.landlord ?? (await prisma.landlordInfo.findFirst());
    if (!landlord) {
      return new Response(
        JSON.stringify({ error: "Vermieterinformationen nicht konfiguriert" }),
        { status: 400, headers: { "Content-Type": "application/json" } }
      );
    }

    const unit = tenant.unit;
    const property = unit.property;
    const startDate = new Date(billingPeriod.startDate);
    const endDate = new Date(billingPeriod.endDate);

    const costs = billingPeriod.costs
      .filter(
        (cost) =>
          cost.enabled !== false && cost.costCategory.apportionable !== false
      )
      .map((cost) => {
        const distributionKey =
          cost.distributionKeyOverride ?? cost.costCategory.distributionKey;
        const allocationKey = unit.allocationKeys.find(
          (key) => key.key === distributionKey
        );
        const unitAmount =
          property.accountingMode === "weg" &&
          cost.tenantAmountOverride !== null
            ? cost.tenantAmountOverride
            : distributionKey === "MEA"
              ? calculateMEAAmount(cost.totalAmount, unit.shares, property.totalShares)
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
          distributionKey,
          totalAmount: cost.totalAmount,
          unitAmount,
        };
      });

    const totalCosts = costs.reduce((sum, c) => sum + c.totalAmount, 0);
    const totalUnitCosts = costs.reduce((sum, c) => sum + c.unitAmount, 0);

    const prepayment = billingPeriod.prepayments[0];
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
        unit={unit}
        tenant={tenant}
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
    if (error instanceof ApiError) {
      return new Response(JSON.stringify({ error: error.message }), {
        status: error.status,
        headers: { "Content-Type": "application/json" },
      });
    }
    console.error("Failed to generate tenant billing PDF:", error);
    return new Response(
      JSON.stringify({ error: "PDF-Generierung fehlgeschlagen" }),
      { status: 500, headers: { "Content-Type": "application/json" } }
    );
  }
}
