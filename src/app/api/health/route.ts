import { prisma } from "@/lib/prisma";

export async function GET() {
  try {
    await prisma.$queryRawUnsafe("SELECT 1");
    return Response.json({
      status: "ok",
      version: process.env.APP_VERSION ?? "unknown",
    });
  } catch {
    return Response.json({ status: "error" }, { status: 503 });
  }
}
