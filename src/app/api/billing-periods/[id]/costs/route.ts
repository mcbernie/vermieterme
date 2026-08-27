import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated } from "@/lib/api-utils";

export function GET(
  _request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const costs = await prisma.cost.findMany({
      where: { billingPeriodId: id },
      include: {
        costCategory: true,
      },
    });

    return jsonOk(costs);
  });
}

export function POST(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await paramsPromise;
    const body = await request.json();
    const {
      costCategoryId,
      totalAmount,
      unitAmount,
      ownerAmount,
      tenantAmountOverride,
      reviewed,
      enabled,
      distributionKeyOverride,
    } = body;

    const existing = await prisma.cost.findUnique({
      where: {
        billingPeriodId_costCategoryId: {
          billingPeriodId: id,
          costCategoryId,
        },
      },
    });

    // Allow partial updates: when a field is omitted we keep the existing
    // value (or fall back to a sensible default for newly created rows).
    // This lets the client toggle `enabled` or change the distribution-key
    // override without re-sending amount values.
    const nextTotalAmount =
      typeof totalAmount === "number" ? totalAmount : existing?.totalAmount ?? 0;
    const nextUnitAmount =
      unitAmount === undefined ? existing?.unitAmount ?? null : unitAmount;
    const nextOwnerAmount =
      ownerAmount === undefined ? existing?.ownerAmount ?? null : ownerAmount;
    const nextTenantAmountOverride =
      tenantAmountOverride === undefined
        ? existing?.tenantAmountOverride ?? null
        : tenantAmountOverride;
    const nextEnabled =
      typeof enabled === "boolean" ? enabled : existing?.enabled ?? true;
    const nextOverride =
      distributionKeyOverride === undefined
        ? existing?.distributionKeyOverride ?? null
        : distributionKeyOverride;

    // `reviewed` is only set when the client explicitly requests it (the
    // "Bestätigen" action). A regular save must never silently mark
    // unreviewed positions as reviewed.
    let nextReviewed: boolean;
    if (typeof reviewed === "boolean") {
      nextReviewed = reviewed;
    } else if (existing) {
      // Editing an existing position invalidates a previous confirmation:
      // the value changed, so it needs to be reviewed again.
      const valueChanged =
        existing.totalAmount !== nextTotalAmount ||
        (existing.unitAmount ?? null) !== (nextUnitAmount ?? null) ||
        (existing.ownerAmount ?? null) !== (nextOwnerAmount ?? null) ||
        (existing.tenantAmountOverride ?? null) !==
          (nextTenantAmountOverride ?? null);
      nextReviewed = valueChanged ? false : existing.reviewed;
    } else {
      // Brand-new position the user just entered manually — counts as
      // reviewed by virtue of being typed in this session.
      nextReviewed = true;
    }

    const cost = await prisma.cost.upsert({
      where: {
        billingPeriodId_costCategoryId: {
          billingPeriodId: id,
          costCategoryId,
        },
      },
      update: {
        totalAmount: nextTotalAmount,
        unitAmount: nextUnitAmount,
        ownerAmount: nextOwnerAmount,
        tenantAmountOverride: nextTenantAmountOverride,
        reviewed: nextReviewed,
        enabled: nextEnabled,
        distributionKeyOverride: nextOverride,
      },
      create: {
        billingPeriodId: id,
        costCategoryId,
        totalAmount: nextTotalAmount,
        unitAmount: nextUnitAmount,
        ownerAmount: nextOwnerAmount,
        tenantAmountOverride: nextTenantAmountOverride,
        reviewed: nextReviewed,
        enabled: nextEnabled,
        distributionKeyOverride: nextOverride,
      },
    });

    return jsonCreated(cost);
  });
}
