import { Button, InlineNotice } from "@zweiund40gmbh/ui";
import { formatDate } from "@/lib/format";
import type { PreviousYearSuggestion } from "./billing-list-utils";

interface Props {
  suggestions: PreviousYearSuggestion[];
  /** Property id whose suggestion is being created, `null` when idle. */
  creatingFor: string | null;
  onCreate: (s: PreviousYearSuggestion) => void;
}

/** Hints for properties without a previous-year period, each with a one-click create action. */
export function BillingSuggestions({ suggestions, creatingFor, onCreate }: Props) {
  if (suggestions.length === 0) return null;
  return (
    <div className="grid gap-2">
      {suggestions.map((s) => (
        <InlineNotice
          key={s.property.id}
          tone="info"
          actions={
            <Button size="sm" variant="primary" disabled={creatingFor !== null} onClick={() => onCreate(s)}>
              Abrechnung {s.previousYear} erstellen
            </Button>
          }
        >
          <span className="font-medium">
            {s.property.street}, {s.property.city}
          </span>
          {" — "}
          Noch keine Abrechnung für {s.previousYear} vorhanden. Daten aus der letzten Abrechnung (
          {formatDate(s.sourcePeriod.startDate)} – {formatDate(s.sourcePeriod.endDate)}) übernehmen?
        </InlineNotice>
      ))}
    </div>
  );
}
