import Link from "next/link";
import { Button, Tag } from "@zweiund40gmbh/ui";
import { billingStatusTone } from "@/features/dashboard/billing-tone";
import { getBillingStatus, getUnreviewedCount } from "@/lib/billing";
import { formatCurrency, formatDate } from "@/lib/format";
import type { BillingPeriodWithProperty } from "@/types";
import { listTotals, type PropertyGroup } from "./billing-list-utils";

const DASH = <span className="text-ink3">–</span>;

/** Formats an optional date, a dash when missing. */
function optionalDate(value: string | null) {
  return value ? formatDate(value) : DASH;
}

/** Back payment (bad) or refund (ok) of a listed period; `difference` is prepayments minus costs. */
function Balance({ difference }: { difference: number }) {
  return difference < 0 ? (
    <span className="text-bad">Nachzahlung: {formatCurrency(Math.abs(difference))}</span>
  ) : (
    <span className="text-ok">Erstattung: {formatCurrency(Math.abs(difference))}</span>
  );
}

/** One table row per billing period. */
function PeriodRow({ bp, onDelete }: { bp: BillingPeriodWithProperty; onDelete: (id: string) => void }) {
  const status = getBillingStatus(bp);
  const totals = listTotals(bp);
  const unreviewed = getUnreviewedCount(bp.costs ?? [], bp.prepayments ?? []);
  return (
    <tr>
      <td className="num whitespace-nowrap">
        <Link href={`/billing/${bp.id}`} className="text-accentstrong hover:underline">
          {formatDate(bp.startDate)} – {formatDate(bp.endDate)}
        </Link>
      </td>
      <td>
        <div className="flex flex-wrap gap-1">
          <Tag tone={billingStatusTone(status.label)}>{status.label}</Tag>
          {unreviewed > 0 && <Tag tone="warn">{unreviewed} Positionen ungeprüft</Tag>}
        </div>
      </td>
      <td className="num">{optionalDate(bp.billingDate)}</td>
      <td className="num">{optionalDate(bp.sentDate)}</td>
      <td className="num">{optionalDate(bp.paidDate)}</td>
      <td className="num text-right">{bp._count && bp._count.costs > 0 ? bp._count.costs : DASH}</td>
      <td className="num text-right">{totals ? formatCurrency(totals.totalCosts) : DASH}</td>
      <td className="num text-right">{totals ? formatCurrency(totals.totalUnitCosts) : DASH}</td>
      <td className="num text-right">{totals ? formatCurrency(totals.totalPrepayment) : DASH}</td>
      <td className="num text-right whitespace-nowrap">{totals ? <Balance difference={totals.difference} /> : DASH}</td>
      <td className="text-right">
        <div className="flex justify-end gap-1">
          <Link href={`/billing/${bp.id}`} className="zui-button" data-variant="secondary" data-size="sm">
            Bearbeiten
          </Link>
          <Button size="sm" variant="danger" onClick={() => onDelete(bp.id)}>
            Löschen
          </Button>
        </div>
      </td>
    </tr>
  );
}

/** Billing periods of one property as a plain table under the property's address. */
export function BillingPeriodTable({ group, onDelete }: { group: PropertyGroup; onDelete: (id: string) => void }) {
  return (
    <section className="grid gap-2">
      <h2 className="section-title">
        {group.property.street}{" "}
        <span className="font-normal text-ink3">
          {group.property.zip} {group.property.city}
        </span>
      </h2>
      <div className="overflow-x-auto border border-line bg-layer">
        <table className="table-plain">
          <thead>
            <tr>
              <th>Zeitraum</th>
              <th>Status</th>
              <th>Abrechnungsdatum</th>
              <th>Versendet</th>
              <th>Bezahlt</th>
              <th className="text-right">Kostenarten</th>
              <th className="text-right">Kosten gesamt</th>
              <th className="text-right">Ihr Anteil</th>
              <th className="text-right">Vorauszahlungen</th>
              <th className="text-right">Ergebnis</th>
              <th className="text-right">Aktionen</th>
            </tr>
          </thead>
          <tbody>
            {group.periods.map((bp) => (
              <PeriodRow key={bp.id} bp={bp} onDelete={onDelete} />
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
