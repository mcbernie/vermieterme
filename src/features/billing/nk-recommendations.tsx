"use client";

import { useRef, useState } from "react";
import { Button, Tag } from "@zweiund40gmbh/ui";
import { RentChangeDialog, type RentChangeDialogValue } from "@/components/rent-change-dialog";
import { dayAfter } from "@/lib/billing";
import { formatCurrency } from "@/lib/format";
import { tenantName, type NkRecommendation } from "./billing-calc";
import { sendJson } from "./billing-http";

/** Default reason of a prepayment adjustment after a back payment. */
export function nkReason(shortfall: number): string {
  return `NK-Anpassung nach Abrechnung (Nachzahlung ${shortfall.toFixed(2)} €)`;
}

/**
 * Creates the prepayment rent change for a recommendation. Throws with a German message on failure,
 * so the dialog stays open and shows it.
 */
export async function createNkAdjustment(rec: NkRecommendation, value: RentChangeDialogValue): Promise<void> {
  const result = await sendJson(
    "/api/rent-changes",
    "POST",
    {
      unitId: rec.unit.id,
      type: "prepayment",
      amount: value.amount,
      effectiveDate: value.effectiveDate,
      reason: value.reason ?? nkReason(rec.result),
    },
    "Die Anpassung konnte nicht angelegt werden.",
  );
  if (!result.ok) throw new Error(result.error ?? "Die Anpassung konnte nicht angelegt werden.");
}

interface Props {
  recommendations: NkRecommendation[];
  /** Last day of the billing period; the adjustment starts the day after. */
  periodEnd: string;
}

/** Suggested higher prepayments for units with a back payment, each creatable as rent change. */
export function NkRecommendations({ recommendations, periodEnd }: Props) {
  const [target, setTarget] = useState<NkRecommendation | null>(null);
  const [applied, setApplied] = useState<ReadonlySet<string>>(new Set());
  const inflight = useRef<Promise<void> | null>(null);

  /** Creates the adjustment once; a second submit while one runs waits for the same request. */
  function apply(rec: NkRecommendation, value: RentChangeDialogValue): Promise<void> {
    if (inflight.current) return inflight.current;
    const run = createNkAdjustment(rec, value)
      .then(() => setApplied((prev) => new Set(prev).add(rec.unit.id)))
      .finally(() => {
        inflight.current = null;
      });
    inflight.current = run;
    return run;
  }

  if (recommendations.length === 0) return null;
  return (
    <section className="grid gap-3">
      <h2 className="section-title">Empfohlene NK-Vorauszahlungs-Anpassungen</h2>
      <div className="overflow-x-auto border border-line bg-layer">
        <table className="table-plain">
          <thead>
            <tr>
              <th>Mieter</th>
              <th>Wohnung</th>
              <th className="text-right">Nachzahlung</th>
              <th className="text-right">NK-Vorauszahlung / Monat</th>
              <th className="text-right">Aktion</th>
            </tr>
          </thead>
          <tbody>
            {recommendations.map((rec) => (
              <tr key={rec.unit.id}>
                <td className="text-ink">{tenantName(rec.tenant)}</td>
                <td className="text-ink2">{rec.unit.name}</td>
                <td className="num text-right text-bad">{formatCurrency(rec.result)}</td>
                <td className="num text-right">
                  {formatCurrency(rec.monthly)} → <span className="font-medium text-ink">{formatCurrency(rec.suggested)}</span>
                </td>
                <td className="text-right">
                  {applied.has(rec.unit.id) ? (
                    <Tag tone="ok">Anpassung angelegt</Tag>
                  ) : (
                    <Button size="sm" variant="primary" onClick={() => setTarget(rec)}>
                      Anpassung anlegen
                    </Button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <RentChangeDialog
        open={target !== null}
        onOpenChange={(open) => !open && setTarget(null)}
        onSubmit={(value) => (target ? apply(target, value) : undefined)}
        title="NK-Vorauszahlung anpassen"
        description={
          target
            ? `${tenantName(target.tenant)} (${target.unit.name}): aktuell ${formatCurrency(target.monthly)}/Monat, Vorschlag ${formatCurrency(target.suggested)}/Monat nach Nachzahlung von ${formatCurrency(target.result)}. Betrag bei Bedarf anpassen.`
            : ""
        }
        defaultAmount={target?.suggested ?? 0}
        defaultEffectiveDate={dayAfter(periodEnd)}
        defaultReason={target ? nkReason(target.result) : ""}
        amountLabel="Neue NK-Vorauszahlung (EUR / Monat)"
      />
    </section>
  );
}
