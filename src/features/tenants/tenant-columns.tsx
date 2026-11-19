import { Button, type ColumnDef } from "@zweiund40gmbh/ui";
import { formatCurrency, formatDate } from "@/lib/format";
import type { RentChange, TenantWithUnit } from "@/types";
import { latestRentChange, propertyLabel, secondTenantName, tenantName } from "./tenant-helpers";
import { TenantTags } from "./tenant-tags";

interface ColumnOptions {
  rentChanges: readonly RentChange[];
  onEdit: (tenant: TenantWithUnit) => void;
  onDelete: (tenant: TenantWithUnit) => void;
}

/** Latest amount of a unit and type, or null. */
function latestAmount(rentChanges: readonly RentChange[], unitId: string, type: RentChange["type"]) {
  return latestRentChange(rentChanges, unitId, type)?.amount ?? null;
}

/** Cell of an amount: currency or a dash. */
function AmountCell({ amount }: { amount: number | null }) {
  return amount === null ? <span className="text-ink3">–</span> : <span className="num">{formatCurrency(amount)}</span>;
}

/** Columns of the tenant table. Build them with `useMemo` so the table keeps its state. */
export function tenantColumns({ rentChanges, onEdit, onDelete }: ColumnOptions): ColumnDef<TenantWithUnit>[] {
  return [
    {
      id: "name",
      header: "Mieter",
      primary: true,
      // Search and export see both names as displayed; sorting goes by last name.
      get: (t) => [tenantName(t), secondTenantName(t)].filter(Boolean).join(" / "),
      sortValue: (t) => `${t.lastName} ${t.firstName}`,
      cell: (t) => {
        const second = secondTenantName(t);
        return (
          <div>
            <div>{tenantName(t)}</div>
            {second && <div className="text-xs text-ink3">{second}</div>}
          </div>
        );
      },
    },
    {
      id: "property",
      header: "Objekt",
      get: (t) => propertyLabel(t),
    },
    { id: "unit", header: "Wohnung", get: (t) => t.unit?.name ?? "" },
    {
      id: "status",
      header: "Status",
      get: (t) => (t.moveOutDate ? "Ausgezogen" : "Aktiv"),
      cell: (t) => <TenantTags tenant={t} />,
    },
    {
      id: "moveIn",
      header: "Einzug",
      get: (t) => formatDate(t.moveInDate),
      sortValue: (t) => new Date(t.moveInDate).getTime(),
    },
    {
      id: "moveOut",
      header: "Auszug",
      hidden: true,
      get: (t) => (t.moveOutDate ? formatDate(t.moveOutDate) : ""),
      sortValue: (t) => (t.moveOutDate ? new Date(t.moveOutDate).getTime() : 0),
    },
    { id: "phone", header: "Telefon", hidden: true, get: (t) => t.phone ?? "" },
    { id: "email", header: "E-Mail", hidden: true, get: (t) => t.email ?? "" },
    {
      id: "rent",
      header: "Kaltmiete",
      align: "end",
      get: (t) => latestAmount(rentChanges, t.unitId, "rent"),
      cell: (t) => <AmountCell amount={latestAmount(rentChanges, t.unitId, "rent")} />,
    },
    {
      id: "prepayment",
      header: "NK-Vorauszahlung",
      align: "end",
      get: (t) => latestAmount(rentChanges, t.unitId, "prepayment"),
      cell: (t) => <AmountCell amount={latestAmount(rentChanges, t.unitId, "prepayment")} />,
    },
    {
      id: "actions",
      header: "",
      align: "end",
      cell: (t) => (
        <div className="flex justify-end gap-1">
          <Button size="sm" variant="secondary" onClick={() => onEdit(t)}>
            Bearbeiten
          </Button>
          <Button size="sm" variant="danger" onClick={() => onDelete(t)}>
            Löschen
          </Button>
        </div>
      ),
    },
  ];
}
