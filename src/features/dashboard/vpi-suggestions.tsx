import Link from "next/link";
import { Button, Tag } from "@zweiund40gmbh/ui";
import { formatCurrency } from "@/lib/format";
import type { VpiSuggestion } from "@/types";
import { formatVpiPercent } from "./vpi-text";

interface VpiSuggestionsProps {
  suggestions: VpiSuggestion[];
  appliedIds: Set<string>;
  onApply: (suggestion: VpiSuggestion) => void;
}

/** Status cell: adjusted, adjustable or waiting period still running. */
function Status({ s, applied, onApply }: { s: VpiSuggestion; applied: boolean; onApply: () => void }) {
  if (!s.eligible) {
    const rest = Math.max(0, s.minMonths - s.monthsSinceLastAdjustment);
    return <Tag tone="warn">Noch {rest} Mon. Wartezeit</Tag>;
  }
  if (applied) return <Tag tone="ok">Angepasst</Tag>;
  return (
    <Button size="sm" variant="primary" onClick={onApply}>
      Anpassung durchführen
    </Button>
  );
}

/** Table of rent adjustment hints derived from the consumer price index. */
export function VpiSuggestions({ suggestions, appliedIds, onApply }: VpiSuggestionsProps) {
  return (
    <section>
      <div className="mb-3 flex items-baseline justify-between">
        <h2 className="section-title">Mietanpassungs-Hinweise</h2>
        <Link href="/settings/vpi" className="text-sm text-accentstrong hover:underline">
          VPI-Werte verwalten
        </Link>
      </div>
      <div className="overflow-x-auto border border-line bg-layer">
        <table className="table-plain">
          <thead>
            <tr>
              <th>Mieter</th>
              <th className="text-right">Miete aktuell</th>
              <th className="text-right">Vorschlag</th>
              <th className="text-right">VPI</th>
              <th className="text-right">Basis</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {suggestions.map((s) => (
              <tr key={s.tenantId}>
                <td className="py-2">
                  <div className="text-ink">{s.tenantName}</div>
                  <div className="text-xs text-ink3">
                    {s.unitName} · {s.propertyAddress}
                  </div>
                </td>
                <td className="num text-right">{formatCurrency(s.currentRent)}</td>
                <td className="num text-right text-ink">{formatCurrency(s.suggestedRent)}</td>
                <td className="num text-right">
                  {s.referenceVpi} → {s.currentVpi} ({formatVpiPercent(s)})
                </td>
                <td className="num text-right text-ink2">{s.baseYear}</td>
                <td>
                  <Status s={s} applied={appliedIds.has(s.tenantId)} onApply={() => onApply(s)} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
