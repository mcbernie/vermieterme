import { apiHandler, requireAuth, jsonOk } from "@/lib/api-utils";
import { prisma } from "@/lib/prisma";
import { getDefaultConfig } from "@/lib/pdf-template";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();

    const template = await prisma.pdfTemplate.findFirst({
      orderBy: { updatedAt: "desc" },
    });

    if (template) {
      return jsonOk({ id: template.id, name: template.name, config: JSON.parse(template.config) });
    }

    return jsonOk({ id: null, name: "Standard", config: getDefaultConfig() });
  });
}

export function PUT(request: Request) {
  return apiHandler(async () => {
    await requireAuth();

    const body = await request.json();
    const { config, name } = body;

    if (!config || typeof config !== "object") {
      throw new Error("Invalid config");
    }

    const configJson = JSON.stringify(config);

    const existing = await prisma.pdfTemplate.findFirst({
      orderBy: { updatedAt: "desc" },
    });

    if (existing) {
      const updated = await prisma.pdfTemplate.update({
        where: { id: existing.id },
        data: { config: configJson, name: name || existing.name },
      });
      return jsonOk({ id: updated.id, name: updated.name });
    }

    const created = await prisma.pdfTemplate.create({
      data: { config: configJson, name: name || "Standard" },
    });
    return jsonOk({ id: created.id, name: created.name });
  });
}
