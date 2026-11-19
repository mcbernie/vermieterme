import { formatCurrency } from "@/lib/format";
import { tenantName, totalsOf, type UnitResult } from "./billing-calc";

/** Result text of one unit: back payment (> 0), refund (< 0) or zero. */
export function resultText(result: number): string {
  if (result > 0) return `Nachzahlung: ${formatCurrency(result)}`;
  if (result < 0) return `Erstattung: ${formatCurrency(Math.abs(result))}`;
  return formatCurrency(0);
}

/** Costs, prepayments and result per unit with totals. */
export function SummaryTable({ results }: { results: UnitResult[] }) {
  if (results.length === 0) {
    return <p className="panel text-sm text-ink2">Keine Wohnungen vorhanden.</p>;
  }
  const totals = totalsOf(results);
  const balance = totals.costs - totals.prepayments;
  return (
    <div className="overflow-x-auto border border-line bg-layer">
      <table className="table-plain">
        <thead>
          <tr>
            <th>Wohnung</th>
            <th>Mieter</th>
            <th className="text-right">Kosten</th>
            <th className="text-right">Vorauszahlungen</th>
            <th className="text-right">Ergebnis</th>
          </tr>
        </thead>
        <tbody>
          {results.map((r) => (
            <tr key={r.unit.id}>
              <td className="text-ink">{r.unit.name}</td>
              <td className="text-ink2">{tenantName(r.tenant)}</td>
              <td className="num text-right">{formatCurrency(r.costs)}</td>
              <td className="num text-right">{formatCurrency(r.prepayment)}</td>
              <td className={`num text-right font-medium ${r.result > 0 ? "text-bad" : "text-ok"}`}>
                {resultText(r.result)}
              </td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr className="border-t border-strong font-medium">
            <td colSpan={2}>Gesamt</td>
            <td className="num text-right">{formatCurrency(totals.costs)}</td>
            <td className="num text-right">{formatCurrency(totals.prepayments)}</td>
            <td className={`num text-right ${balance > 0 ? "text-bad" : "text-ok"}`}>
              {formatCurrency(Math.abs(balance))}
            </td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}
