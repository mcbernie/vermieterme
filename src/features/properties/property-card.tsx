import { Button } from "@zweiund40gmbh/ui";
import type { PropertyWithCount, PropertyWithUnits, RentChange, UnitWithTenants } from "@/types";
import { UnitTable } from "./unit-table";

interface Props {
  property: PropertyWithCount;
  expanded: boolean;
  detail: PropertyWithUnits | null;
  rentChanges: RentChange[];
  onToggle: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onAddUnit: () => void;
  onEditUnit: (unit: UnitWithTenants) => void;
  onDeleteUnit: (unit: UnitWithTenants) => void;
}

/** One property as a panel: summary row plus its units when expanded. */
export function PropertyCard(p: Props) {
  const { property, expanded, detail } = p;
  return (
    <div className="panel !p-0">
      <div className="flex flex-wrap items-center justify-between gap-3 p-4">
        <button
          type="button"
          aria-expanded={expanded}
          onClick={p.onToggle}
          className="flex-1 cursor-pointer text-left"
        >
          <div className="section-title">{property.street}</div>
          <div className="text-sm text-ink2">
            {property.zip} {property.city}
          </div>
        </button>
        <div className="text-right">
          <div className="text-sm text-ink2">{property._count?.units || 0} Wohnungen</div>
          <div className="num text-xs text-ink3">{property.totalShares} Anteile</div>
        </div>
        <div className="flex gap-1">
          <Button size="sm" variant="secondary" onClick={p.onEdit}>
            Bearbeiten
          </Button>
          <Button size="sm" variant="danger" onClick={p.onDelete}>
            Löschen
          </Button>
        </div>
      </div>

      {expanded && detail && (
        <div className="border-t border-line p-4">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="label-caps">Wohnungen</h2>
            <Button size="sm" variant="secondary" onClick={p.onAddUnit}>
              Wohnung hinzufügen
            </Button>
          </div>
          {detail.units.length === 0 ? (
            <p className="text-sm text-ink2">Noch keine Wohnungen angelegt.</p>
          ) : (
            <UnitTable
              units={detail.units}
              rentChanges={p.rentChanges}
              onEdit={p.onEditUnit}
              onDelete={p.onDeleteUnit}
            />
          )}
        </div>
      )}
    </div>
  );
}
