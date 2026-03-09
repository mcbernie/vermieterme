import { NextResponse } from "next/server";
import { auth } from "@/lib/auth";

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number
  ) {
    super(message);
  }
}

export async function requireAuth() {
  const session = await auth();
  if (!session?.user?.id) {
    throw new ApiError("Nicht angemeldet", 401);
  }
  return session as { user: { id: string; email?: string | null } };
}

export function apiHandler(
  fn: () => Promise<NextResponse>
): Promise<NextResponse> {
  return fn().catch((error) => {
    if (error instanceof ApiError) {
      return NextResponse.json(
        { error: error.message },
        { status: error.status }
      );
    }
    console.error(error);
    return NextResponse.json(
      { error: "Internal Server Error" },
      { status: 500 }
    );
  });
}

export function jsonOk(data: unknown, status = 200) {
  return NextResponse.json(data, { status });
}

export function jsonCreated(data: unknown) {
  return NextResponse.json(data, { status: 201 });
}
