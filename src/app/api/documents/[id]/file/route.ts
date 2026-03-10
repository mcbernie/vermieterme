import path from "path";
import { readFile } from "fs/promises";
import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, ApiError } from "@/lib/api-utils";

const UPLOAD_DIR = path.join(process.cwd(), "data", "uploads");

export function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  return apiHandler(async () => {
    await requireAuth();
    const { id } = await params;

    const document = await prisma.document.findUnique({ where: { id } });
    if (!document) {
      throw new ApiError("Dokument nicht gefunden", 404);
    }

    const filePath = path.join(UPLOAD_DIR, document.fileName);
    const buffer = await readFile(filePath);

    return new Response(buffer, {
      headers: {
        "Content-Type": document.mimeType,
        "Content-Disposition": `inline; filename="${encodeURIComponent(document.originalName)}"`,
        "Content-Length": String(buffer.length),
      },
    });
  });
}
