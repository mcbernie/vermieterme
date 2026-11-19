import { Button, Tag } from "@zweiund40gmbh/ui";
import { formatCurrency, formatDate } from "@/lib/format";
import type { RentChangeWithUnit } from "@/types";
import { TYPE_LABELS, type RentChangeGroups } from "./rent-change-utils";

interface RentChangeGroupsProps {
  groups: RentChangeGroups;
  onEdit: (change: RentChangeWithUnit) => void;
  onDelete: (change: RentChangeWithUnit) => void;
}

/** One table per property; rows are the rent changes, grouped by unit. */
export function RentChangeGroupList({ groups, onEdit, onDelete }: RentChangeGroupsProps) {
  return (
    <div className="grid gap-6">
      {Object.entries(groups).map(([propertyName, units]) => (
        <section key={propertyName}>
          <h2 className="section-title mb-3">{propertyName}</h2>
          <div className="overflow-x-auto border border-line bg-layer">
            <table className="table-plain">
              <thead>
                <tr>
                  <th>Wohnung</th>
                  <th>Typ</th>
                  <th className="text-right">Betrag</th>
                  <th>Gültig ab</th>
                  <th>Grund</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {Object.entries(units).flatMap(([unitName, changes]) =>
                  changes.map((change) => (
                    <tr key={change.id}>
                      <td>{unitName}</td>
                      <td>
                        <Tag tone={change.type === "rent" ? "info" : "neutral"}>{TYPE_LABELS[change.type]}</Tag>
                      </td>
                      <td className="num text-right text-ink">{formatCurrency(change.amount)}</td>
                      <td className="num">{formatDate(change.effectiveDate)}</td>
                      <td className="text-ink2">{change.reason ?? ""}</td>
                      <td>
                        <div className="flex justify-end gap-1">
                          <Button size="sm" onClick={() => onEdit(change)}>
                            Bearbeiten
                          </Button>
                          <Button size="sm" variant="danger" onClick={() => onDelete(change)}>
                            Löschen
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>
      ))}
    </div>
  );
}
