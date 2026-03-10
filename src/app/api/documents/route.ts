import { NextRequest } from "next/server";
import path from "path";
import { writeFile, mkdir } from "fs/promises";
import { randomUUID } from "crypto";
import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, jsonCreated, ApiError } from "@/lib/api-utils";

const UPLOAD_DIR = path.join(process.cwd(), "data", "uploads");
const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10 MB
const ALLOWED_TYPES = [
  "application/pdf",
  "image/jpeg",
  "image/png",
  "image/webp",
];

export function GET(request: NextRequest) {
  return apiHandler(async () => {
    await requireAuth();
    const { searchParams } = new URL(request.url);
    const billingPeriodId = searchParams.get("billingPeriodId");
    const tenantId = searchParams.get("tenantId");

    const where: Record<string, string> = {};
    if (billingPeriodId) where.billingPeriodId = billingPeriodId;
    if (tenantId) where.tenantId = tenantId;

    const documents = await prisma.document.findMany({
      where,
      orderBy: { createdAt: "desc" },
    });

    return jsonOk(documents);
  });
}

export function POST(request: NextRequest) {
  return apiHandler(async () => {
    await requireAuth();

    const formData = await request.formData();
    const file = formData.get("file") as File | null;
    const billingPeriodId = formData.get("billingPeriodId") as string | null;
    const tenantId = formData.get("tenantId") as string | null;
    const category = (formData.get("category") as string) || "other";

    if (!file) {
      throw new ApiError("Keine Datei hochgeladen", 400);
    }

    if (!ALLOWED_TYPES.includes(file.type)) {
      throw new ApiError(
        "Dateityp nicht erlaubt. Erlaubt: PDF, JPEG, PNG, WebP",
        400
      );
    }

    if (file.size > MAX_FILE_SIZE) {
      throw new ApiError("Datei zu groß (max. 10 MB)", 400);
    }

    await mkdir(UPLOAD_DIR, { recursive: true });

    const ext = path.extname(file.name) || ".bin";
    const fileName = `${randomUUID()}${ext}`;
    const filePath = path.join(UPLOAD_DIR, fileName);

    const buffer = Buffer.from(await file.arrayBuffer());
    await writeFile(filePath, buffer);

    const document = await prisma.document.create({
      data: {
        fileName,
        originalName: file.name,
        mimeType: file.type,
        size: file.size,
        category,
        billingPeriodId: billingPeriodId || null,
        tenantId: tenantId || null,
      },
    });

    return jsonCreated(document);
  });
}
