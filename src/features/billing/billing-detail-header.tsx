import { Button, PageHeader, SaveStatus, type AutosaveStatus } from "@zweiund40gmbh/ui";
import { getDaysInPeriod, getMonthsInPeriod } from "@/lib/billing";
import { formatDate } from "@/lib/format";
import type { BillingPeriodDetail } from "@/types";
import { BillingDates } from "./billing-dates";

interface Props {
  id: string;
  period: BillingPeriodDetail;
  saving: boolean;
  /** Save indicator next to the save button; errors are shown as page notice instead. */
  saveStatus: AutosaveStatus;
  lastSavedAt: Date | null;
  onSave: () => void;
  onDate: (field: "sentDate" | "paidDate", value: string) => void;
}

/** Page header of a billing period: period facts, save and PDF actions, and sent/paid dates. */
export function BillingDetailHeader({ id, period, saving, saveStatus, lastSavedAt, onSave, onDate }: Props) {
  const { property } = period;
  const months = getMonthsInPeriod(period.startDate, period.endDate);
  const days = getDaysInPeriod(period.startDate, period.endDate);
  return (
    <div className="grid gap-4">
      <PageHeader
        backHref="/billing"
        crumbs={[{ label: "Abrechnungen", href: "/billing" }, { label: `${property.street}, ${property.city}` }]}
        title={`Abrechnung: ${property.street}, ${property.city}`}
        description={
          <>
            <span className="num">
              Zeitraum: {formatDate(period.startDate)} – {formatDate(period.endDate)} ({months} Monate, {days} Tage)
            </span>
            {period.billingDate && (
              <span className="num block">Abrechnungsdatum: {formatDate(period.billingDate)}</span>
            )}
          </>
        }
        actions={
          <div className="flex items-center gap-3">
            <SaveStatus status={saveStatus} lastSavedAt={lastSavedAt} />
            <Button variant="primary" onClick={onSave} disabled={saving}>
              {saving ? "Speichern..." : "Speichern"}
            </Button>
            <a
              href={`/api/billing-periods/${id}/pdf`}
              target="_blank"
              rel="noopener noreferrer"
              className="zui-button"
              data-variant="secondary"
              data-size="md"
            >
              PDF erstellen
            </a>
          </div>
        }
      />
      <BillingDates sentDate={period.sentDate} paidDate={period.paidDate} onChange={onDate} />
    </div>
  );
}
