import type { ReactNode } from "react";
import { SettingsFrame } from "@/features/settings/settings-layout";

/** Layout of all settings sub pages: shared header and navigation that stay mounted between them. */
export default function SettingsLayout({ children }: { children: ReactNode }) {
  return <SettingsFrame>{children}</SettingsFrame>;
}
