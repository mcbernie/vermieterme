import type { ReactNode } from "react";
import { AppFrame } from "@/components/layout/app-frame";

/** Layout of the signed-in area: every page inside the application frame. */
export default function AppLayout({ children }: { children: ReactNode }) {
  return <AppFrame>{children}</AppFrame>;
}
