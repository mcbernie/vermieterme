import Link from "next/link";
import { EmptyState, Tag } from "@zweiund40gmbh/ui";
import { calculateBillingTotals, getBillingStatus } from "@/lib/billing";
import { formatCurrency, formatDate } from "@/lib/format";
import type { BillingPeriodWithProperty } from "@/types";
import { billingStatusTone } from "./billing-tone";

/** Result cell: back payment (bad) or refund (ok), empty without cost data. */
function Result({ bp }: { bp: BillingPeriodWithProperty }) {
  if (!bp.costs || bp.costs.length === 0) return <span className="text-ink3">–</span>;
  const t = calculateBillingTotals(bp.costs, bp.prepayments ?? [], bp.startDate, bp.endDate);
  return t.difference < 0 ? (
    <span className="text-bad">Nachzahlung: {formatCurrency(Math.abs(t.difference))}</span>
  ) : (
    <span className="text-ok">Erstattung: {formatCurrency(t.difference)}</span>
  );
}

/** Table with the five latest billing periods. */
export function RecentBillings({ billingPeriods }: { billingPeriods: BillingPeriodWithProperty[] }) {
  return (
    <section>
      <h2 className="section-title mb-3">Letzte Abrechnungen</h2>
      {billingPeriods.length === 0 ? (
        <EmptyState
          title="Noch keine Abrechnungen vorhanden."
          action={
            <Link href="/billing" className="text-sm text-accentstrong hover:underline">
              Erste Abrechnung erstellen
            </Link>
          }
        />
      ) : (
        <div className="overflow-x-auto border border-line bg-layer">
          <table className="table-plain">
            <thead>
              <tr>
                <th>Objekt</th>
                <th>Zeitraum</th>
                <th>Status</th>
                <th>Ergebnis</th>
              </tr>
            </thead>
            <tbody>
              {billingPeriods.slice(0, 5).map((bp) => {
                const status = getBillingStatus(bp);
                return (
                  <tr key={bp.id}>
                    <td>
                      <Link href={`/billing/${bp.id}`} className="text-accentstrong hover:underline">
                        {bp.property?.street}, {bp.property?.city}
                      </Link>
                    </td>
                    <td className="num">
                      {formatDate(bp.startDate)} – {formatDate(bp.endDate)}
                    </td>
                    <td>
                      <Tag tone={billingStatusTone(status.label)}>{status.label}</Tag>
                    </td>
                    <td className="num">
                      <Result bp={bp} />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
