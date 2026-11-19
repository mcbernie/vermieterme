import { Button } from "@zweiund40gmbh/ui";
import { formatCurrency } from "@/lib/format";
import type { RentChange, UnitWithTenants } from "@/types";
import { getLatestRent } from "./latest-rent";

interface Props {
  units: UnitWithTenants[];
  rentChanges: RentChange[];
  onEdit: (unit: UnitWithTenants) => void;
  onDelete: (unit: UnitWithTenants) => void;
}

/** Static table of the units of one property with current tenant and latest rent. */
export function UnitTable({ units, rentChanges, onEdit, onDelete }: Props) {
  return (
    <div className="overflow-x-auto">
      <table className="table-plain">
        <thead>
          <tr>
            <th>Bezeichnung</th>
            <th>Stockwerk</th>
            <th className="text-right">Anteile</th>
            <th>Aktueller Mieter</th>
            <th className="text-right">Miete</th>
            <th className="text-right">Aktionen</th>
          </tr>
        </thead>
        <tbody>
          {units.map((unit) => {
            const tenant = unit.tenants.find((t) => !t.moveOutDate);
            const { rent, prepayment } = getLatestRent(rentChanges, unit.id);
            return (
              <tr key={unit.id}>
                <td className="text-ink">{unit.name}</td>
                <td className="text-ink2">{unit.floor}</td>
                <td className="num text-right text-ink2">{unit.shares}</td>
                <td className="text-ink2">{tenant ? `${tenant.firstName} ${tenant.lastName}` : "Kein Mieter"}</td>
                <td className="num text-right text-ink2">
                  {rent == null && prepayment == null ? (
                    <span className="text-ink3">—</span>
                  ) : (
                    <>
                      {rent != null && <div>{formatCurrency(rent)}</div>}
                      {prepayment != null && <div className="text-xs text-ink3">+ {formatCurrency(prepayment)} NK</div>}
                    </>
                  )}
                </td>
                <td className="text-right">
                  <div className="flex justify-end gap-1">
                    <Button size="sm" variant="secondary" onClick={() => onEdit(unit)}>
                      Bearbeiten
                    </Button>
                    <Button size="sm" variant="danger" onClick={() => onDelete(unit)}>
                      Löschen
                    </Button>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
