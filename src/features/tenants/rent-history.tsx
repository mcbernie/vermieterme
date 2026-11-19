import { formatCurrency, formatDate } from "@/lib/format";
import type { RentChange } from "@/types";

/** Static table of the rent and prepayment changes of a unit (newest first). */
export function RentHistory({ changes }: { changes: readonly RentChange[] }) {
  if (changes.length === 0) {
    return <p className="text-sm text-ink3">Noch keine Miete oder Vorauszahlung erfasst.</p>;
  }
  return (
    <div className="overflow-x-auto border border-line">
      <table className="table-plain">
        <thead>
          <tr>
            <th>Gültig ab</th>
            <th>Art</th>
            <th className="text-right">Betrag</th>
            <th>Grund</th>
          </tr>
        </thead>
        <tbody>
          {changes.map((rc) => (
            <tr key={rc.id}>
              <td className="num text-ink2">{formatDate(rc.effectiveDate)}</td>
              <td className="text-ink2">{rc.type === "rent" ? "Kaltmiete" : "NK-Vorauszahlung"}</td>
              <td className="num text-right text-ink">{formatCurrency(rc.amount)}</td>
              <td className="text-ink3">{rc.reason || "–"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
