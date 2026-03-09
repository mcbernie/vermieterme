import {
  apiHandler,
  requireAuth,
  jsonOk,
  jsonCreated,
  ApiError,
} from "@/lib/api-utils";
import { hashPassword } from "@/lib/password";
import { prisma } from "@/lib/prisma";
import { PASSWORD_MIN_LENGTH } from "@/lib/constants";

export function GET() {
  return apiHandler(async () => {
    await requireAuth();

    const users = await prisma.user.findMany({
      select: {
        id: true,
        name: true,
        email: true,
        image: true,
      },
      orderBy: { email: "asc" },
    });

    return jsonOk(users);
  });
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();

    const { name, email, password } = await request.json();

    if (!email || !password) {
      throw new ApiError("E-Mail und Passwort sind erforderlich", 400);
    }

    if (password.length < PASSWORD_MIN_LENGTH) {
      throw new ApiError(
        "Passwort muss mindestens 8 Zeichen lang sein",
        400
      );
    }

    const existing = await prisma.user.findUnique({ where: { email } });
    if (existing) {
      throw new ApiError(
        "Ein Benutzer mit dieser E-Mail existiert bereits",
        409
      );
    }

    const hashedPassword = await hashPassword(password);
    const user = await prisma.user.create({
      data: {
        name: name || null,
        email,
        password: hashedPassword,
      },
      select: {
        id: true,
        name: true,
        email: true,
      },
    });

    return jsonCreated(user);
  });
}
