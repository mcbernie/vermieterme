import { apiHandler, requireAuth, jsonOk, ApiError } from "@/lib/api-utils";
import { prisma } from "@/lib/prisma";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();

    const [
      properties,
      units,
      tenants,
      costCategories,
      billingPeriods,
      costs,
      prepayments,
      landlordInfo,
      pdfTemplates,
    ] = await Promise.all([
      prisma.property.findMany(),
      prisma.unit.findMany(),
      prisma.tenant.findMany(),
      prisma.costCategory.findMany(),
      prisma.billingPeriod.findMany(),
      prisma.cost.findMany(),
      prisma.prepayment.findMany(),
      prisma.landlordInfo.findMany(),
      prisma.pdfTemplate.findMany(),
    ]);

    const backup = {
      version: 1,
      exportedAt: new Date().toISOString(),
      data: {
        properties,
        units,
        tenants,
        costCategories,
        billingPeriods,
        costs,
        prepayments,
        landlordInfo,
        pdfTemplates,
      },
    };

    const date = new Date().toISOString().slice(0, 10);
    return new Response(JSON.stringify(backup, null, 2), {
      headers: {
        "Content-Type": "application/json",
        "Content-Disposition": `attachment; filename="vermieterme-backup-${date}.json"`,
      },
    });
  });
}

const REQUIRED_KEYS = [
  "properties",
  "units",
  "tenants",
  "costCategories",
  "billingPeriods",
  "costs",
  "prepayments",
  "landlordInfo",
];

function validateBackup(
  body: unknown
// eslint-disable-next-line @typescript-eslint/no-explicit-any
): body is { version: number; data: Record<string, any[]> } {
  if (!body || typeof body !== "object") return false;
  const obj = body as Record<string, unknown>;
  if (obj.version !== 1) return false;
  if (!obj.data || typeof obj.data !== "object") return false;
  const d = obj.data as Record<string, unknown>;
  return REQUIRED_KEYS.every((key) => Array.isArray(d[key]));
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();

    const body = await request.json();

    if (!validateBackup(body)) {
      throw new ApiError("Ungültiges Backup-Format", 400);
    }

    const { data } = body;

    await prisma.$transaction([
      // Delete children first
      prisma.cost.deleteMany(),
      prisma.prepayment.deleteMany(),
      prisma.tenant.deleteMany(),
      prisma.billingPeriod.deleteMany(),
      prisma.unit.deleteMany(),
      prisma.property.deleteMany(),
      prisma.costCategory.deleteMany(),
      prisma.landlordInfo.deleteMany(),
      prisma.pdfTemplate.deleteMany(),
      // Create parents first
      ...(data.properties.length > 0
        ? [prisma.property.createMany({ data: data.properties })]
        : []),
      ...(data.costCategories.length > 0
        ? [prisma.costCategory.createMany({ data: data.costCategories })]
        : []),
      ...(data.landlordInfo.length > 0
        ? [prisma.landlordInfo.createMany({ data: data.landlordInfo })]
        : []),
      ...(data.units.length > 0
        ? [prisma.unit.createMany({ data: data.units })]
        : []),
      ...(data.billingPeriods.length > 0
        ? [prisma.billingPeriod.createMany({ data: data.billingPeriods })]
        : []),
      ...(data.tenants.length > 0
        ? [prisma.tenant.createMany({ data: data.tenants })]
        : []),
      ...(data.costs.length > 0
        ? [prisma.cost.createMany({ data: data.costs })]
        : []),
      ...(data.prepayments.length > 0
        ? [prisma.prepayment.createMany({ data: data.prepayments })]
        : []),
      ...((data.pdfTemplates?.length ?? 0) > 0
        ? [prisma.pdfTemplate.createMany({ data: data.pdfTemplates })]
        : []),
    ]);

    return jsonOk({ success: true, message: "Backup erfolgreich importiert" });
  });
}
