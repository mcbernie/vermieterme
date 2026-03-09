export { auth as proxy } from "@/lib/auth";

export const config = {
  matcher: ["/((?!api/auth|login|impressum|_next/static|_next/image|favicon\\.ico|favicon\\.svg|vermieterme-icon\\.svg).*)"],
};
