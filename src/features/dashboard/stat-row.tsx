import Link from "next/link";
import { StatTile } from "@zweiund40gmbh/ui";

interface StatRowProps {
  properties: number;
  units: number;
  billings: number;
}

/** Link below a tile figure. */
function More({ href, children }: { href: string; children: string }) {
  return (
    <Link href={href} className="text-accentstrong hover:underline">
      {children}
    </Link>
  );
}

/** Three key figures: properties, units and billings with a link to the matching list. */
export function StatRow({ properties, units, billings }: StatRowProps) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <StatTile label="Anzahl Objekte" value={properties} hint={<More href="/properties">Alle anzeigen</More>} />
      <StatTile label="Anzahl Wohnungen" value={units} hint={<More href="/properties">Objekte verwalten</More>} />
      <StatTile label="Aktuelle Abrechnungen" value={billings} hint={<More href="/billing">Alle anzeigen</More>} />
    </div>
  );
}
