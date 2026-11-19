import { InlineNotice, Progress } from "@zweiund40gmbh/ui";

interface Props {
  total: number;
  reviewed: number;
  allReviewed: boolean;
}

/** How many positions of the period are reviewed; hidden while there are none. */
export function ReviewProgress({ total, reviewed, allReviewed }: Props) {
  if (total === 0) return null;
  const text = `${reviewed} von ${total} Positionen geprüft`;
  return (
    <InlineNotice tone={allReviewed ? "ok" : "warn"} role="status" title={text}>
      <div className="grid gap-2">
        {allReviewed && <span>Alle geprüft</span>}
        <Progress value={reviewed} max={total} label="Prüfstand" valueText={text} tone={allReviewed ? "ok" : "warn"} size="sm" />
      </div>
    </InlineNotice>
  );
}
