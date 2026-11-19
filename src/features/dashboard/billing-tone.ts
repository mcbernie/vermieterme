import type { TagTone } from "@zweiund40gmbh/ui";

/** Tag tone per billing status label as returned by `getBillingStatus` in `@/lib/billing`. */
const TONES: Record<string, TagTone> = {
  Bezahlt: "ok",
  Versendet: "info",
  Abgeschlossen: "ok",
  "In Bearbeitung": "warn",
  Offen: "neutral",
};

/**
 * Maps a billing status label to the `Tag` tone.
 * @param label Status label such as "Bezahlt" or "Offen".
 * @returns The matching tone, `neutral` for unknown labels.
 */
export function billingStatusTone(label: string): TagTone {
  return TONES[label] ?? "neutral";
}
