"use client";

import { Button, Tag } from "@zweiund40gmbh/ui";
import { getMonthsInPeriod } from "@/lib/billing";
import { formatCurrency } from "@/lib/format";
import type { BillingPeriodDetail } from "@/types";
import {
  currentTenant,
  isPrepaymentUnreviewed,
  tenantName,
  unitPrepayment,
  type PrepaymentValues,
} from "./billing-calc";
import { MoneyCell } from "./money-cell";
import type { AmountProblem } from "./use-billing-save";

interface Props {
  period: BillingPeriodDetail;
  prepayments: PrepaymentValues;
  onChange: (unitId: string, euros: number | null) => void;
  onProblem: (key: string, problem: AmountProblem) => void;
  onConfirm: (unitId: string) => void;
}

/** Monthly prepayment per unit and its total over the period. */
export function PrepaymentTable({ period, prepayments, onChange, onProblem, onConfirm }: Props) {
  const units = period.property.units || [];
  const months = getMonthsInPeriod(period.startDate, period.endDate);
  if (units.length === 0) {
    return <p className="panel text-sm text-ink2">Keine Wohnungen für dieses Objekt vorhanden.</p>;
  }
  return (
    <div className="overflow-x-auto border border-line bg-layer">
      <table className="table-plain">
        <thead>
          <tr>
            <th>Wohnung</th>
            <th>Mieter</th>
            <th className="text-right">Anteile</th>
            <th className="text-right">Monatlich</th>
            <th className="text-right">Gesamt ({months} Mon.)</th>
          </tr>
        </thead>
        <tbody>
          {units.map((unit) => {
            const unreviewed = isPrepaymentUnreviewed(period, unit.id);
            return (
              <tr key={unit.id}>
                <td>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-ink">{unit.name}</span>
                    {unreviewed && (
                      <>
                        <Tag tone="warn">Ungeprüft</Tag>
                        <Button size="sm" onClick={() => onConfirm(unit.id)}>
                          Bestätigen
                        </Button>
                      </>
                    )}
                  </div>
                </td>
                <td className="text-ink2">{tenantName(currentTenant(unit))}</td>
                <td className="num text-right text-ink2">
                  {unit.shares} / {period.property.totalShares}
                </td>
                <td className="text-right">
                  <MoneyCell
                    label={`Monatliche Vorauszahlung ${unit.name}`}
                    value={prepayments[unit.id] ?? null}
                    onChange={(v) => onChange(unit.id, v)}
                    onProblemChange={(problem) => onProblem(`prepayment-${unit.id}`, problem)}
                  />
                </td>
                <td className="num text-right text-ink">
                  {formatCurrency(unitPrepayment(prepayments, unit.id, period))}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
