import type { VpiSuggestion } from "@/types";

/**
 * Formats the VPI change of a suggestion, e.g. "+2.50%" or "-0.30%".
 * @param s VPI suggestion.
 */
export function formatVpiPercent(s: Pick<VpiSuggestion, "percentageChange">): string {
  const sign = s.percentageChange > 0 ? "+" : "";
  return `${sign}${s.percentageChange.toFixed(2)}%`;
}

/**
 * Default reason text of a rent change that follows from a VPI suggestion.
 * @param s VPI suggestion.
 */
export function vpiReason(
  s: Pick<VpiSuggestion, "referenceVpi" | "currentVpi" | "percentageChange">
): string {
  return `VPI-Anpassung (${s.referenceVpi} → ${s.currentVpi}, ${formatVpiPercent(s)})`;
}
