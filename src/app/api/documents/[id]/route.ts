import path from "path";
import { unlink } from "fs/promises";
import { prisma } from "@/lib/prisma";
import { apiHandler, requireAuth, jsonOk, ApiError } from "@/lib/api-utils";

const UPLOAD_DIR = path.join(process.cwd(), "data", "uploads");

export function DELETE(
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

    // Delete file from disk
    try {
      await unlink(path.join(UPLOAD_DIR, document.fileName));
    } catch {
      // File might already be deleted
    }

    await prisma.document.delete({ where: { id } });

    return jsonOk({ success: true });
  });
}
