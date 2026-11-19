import { Tag } from "@zweiund40gmbh/ui";
import type { Tenant } from "@/types";

/** Status tags of a tenant: active or moved out, plus index or graduated lease. */
export function TenantTags({ tenant }: { tenant: Pick<Tenant, "moveOutDate" | "leaseType"> }) {
  return (
    <span className="inline-flex flex-wrap gap-1">
      {tenant.moveOutDate ? (
        <Tag>Ausgezogen</Tag>
      ) : (
        <Tag tone="ok" dot>
          Aktiv
        </Tag>
      )}
      {tenant.leaseType === "index" && <Tag tone="info">Indexmiete</Tag>}
      {tenant.leaseType === "staffel" && <Tag tone="accent">Staffelmiete</Tag>}
    </span>
  );
}
