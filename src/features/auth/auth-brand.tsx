import { PRODUCT_NAME } from "@/lib/brand";

/** Brand mark as in the app header: accent square and product name. */
export function AuthBrand() {
  return (
    <div className="flex items-center gap-2 text-lg font-semibold text-ink">
      <span aria-hidden="true" className="inline-block h-4 w-4" style={{ background: "var(--accent)" }} />
      {PRODUCT_NAME}
    </div>
  );
}
