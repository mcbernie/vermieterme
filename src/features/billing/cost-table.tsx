"use client";

import Link from "next/link";
import { Button, Icon, IconButton, Select, Tag } from "@zweiund40gmbh/ui";
import { calculateMEAAmount } from "@/lib/billing";
import { DISTRIBUTION_KEYS } from "@/lib/constants";
import { formatCurrency } from "@/lib/format";
import type { BillingPeriodDetail, CostCategory } from "@/types";
import { DEFAULT_COST_VALUE, effectiveKey, isCostUnreviewed, type CostFormValue, type CostValues } from "./billing-calc";
import { MoneyCell } from "./money-cell";
import type { AmountProblem } from "./use-billing-save";

const DEFAULT_KEY = "__default__";

/** Callbacks of the cost table. */
export interface CostTableActions {
  onAmount: (categoryId: string, field: "totalAmount" | "unitAmount", euros: number | null) => void;
  onProblem: (key: string, problem: AmountProblem) => void;
  onToggle: (categoryId: string, enabled: boolean) => void;
  onOverride: (categoryId: string, override: string | null) => void;
  onConfirm: (categoryId: string) => void;
}

interface RowProps extends CostTableActions {
  category: CostCategory;
  value: CostFormValue;
  period: BillingPeriodDetail;
}

/** Per-unit MEA shares of a total, e.g. "EG: 120,00 €, OG: 80,00 €". */
function meaText(total: number, period: BillingPeriodDetail): string {
  return period.property.units
    .map((u) => `${u.name}: ${formatCurrency(calculateMEAAmount(total, u.shares, period.property.totalShares))}`)
    .join(", ");
}

/** One cost category with visibility switch, distribution key and amounts. */
function CostRow({ category, value, period, ...on }: RowProps) {
  const enabled = value.enabled !== false;
  const key = effectiveKey(value, category);
  const needsUnitAmount = key === "laut Bescheid" || key === "siehe Anlage";
  const overridden = value.distributionKeyOverride !== null && value.distributionKeyOverride !== undefined;
  const unreviewed = enabled && isCostUnreviewed(period, category.id);
  const options = [
    { value: DEFAULT_KEY, label: `Standard (${category.distributionKey})` },
    ...DISTRIBUTION_KEYS.map((k) => ({ value: k, label: k })),
  ];
  const showMea = key === "MEA" && value.totalAmount !== null && period.property.units.length > 0;

  return (
    <tr className={enabled ? undefined : "text-ink3"}>
      <td className="w-10">
        <IconButton
          size="sm"
          label={enabled ? "In dieser Abrechnung ausblenden" : "In dieser Abrechnung einblenden"}
          onClick={() => on.onToggle(category.id, !enabled)}
        >
          <Icon name={enabled ? "eye" : "eyeOff"} />
        </IconButton>
      </td>
      <td>
        <div className="flex flex-wrap items-center gap-2">
          <span className={enabled ? "text-ink" : "line-through"}>{category.name}</span>
          {unreviewed && (
            <>
              <Tag tone="warn">Ungeprüft</Tag>
              <Button size="sm" onClick={() => on.onConfirm(category.id)}>
                Bestätigen
              </Button>
            </>
          )}
        </div>
      </td>
      <td className="w-56">
        <Select
          size="sm"
          aria-label={`Verteilerschlüssel ${category.name}`}
          options={options}
          value={overridden ? value.distributionKeyOverride : DEFAULT_KEY}
          onChange={(v) => on.onOverride(category.id, v === DEFAULT_KEY ? null : v)}
        />
        {overridden && <p className="mt-1 text-xs text-warn">Abweichend (Standard: {category.distributionKey})</p>}
      </td>
      <td className="text-right">
        <MoneyCell
          label={`Gesamtbetrag ${category.name}`}
          value={value.totalAmount}
          disabled={!enabled}
          onChange={(v) => on.onAmount(category.id, "totalAmount", v)}
          onProblemChange={(problem) => on.onProblem(`cost-total-${category.id}`, problem)}
        />
      </td>
      <td className="num text-right">
        {needsUnitAmount ? (
          <MoneyCell
            label={`Anteil Wohnung ${category.name}`}
            value={value.unitAmount}
            disabled={!enabled}
            onChange={(v) => on.onAmount(category.id, "unitAmount", v)}
            onProblemChange={(problem) => on.onProblem(`cost-unit-${category.id}`, problem)}
          />
        ) : key === "MEA" ? (
          <span className="text-ink2">{showMea ? meaText(value.totalAmount ?? 0, period) : "Automatisch (MEA)"}</span>
        ) : (
          <span className="text-ink3">–</span>
        )}
      </td>
    </tr>
  );
}

interface Props extends CostTableActions {
  period: BillingPeriodDetail;
  categories: CostCategory[];
  costs: CostValues;
}

/** Cost categories of the period with amounts per category and distribution key. */
export function CostTable({ period, categories, costs, ...actions }: Props) {
  if (categories.length === 0) {
    return (
      <p className="panel text-sm text-ink2">
        Keine Kostenarten definiert. Bitte legen Sie zuerst Kostenarten in den{" "}
        <Link href="/settings" className="text-accentstrong hover:underline">
          Einstellungen
        </Link>{" "}
        an.
      </p>
    );
  }
  return (
    <div className="overflow-x-auto border border-line bg-layer">
      <table className="table-plain">
        <thead>
          <tr>
            <th>
              <span className="sr-only">Aktiv</span>
            </th>
            <th>Kostenart</th>
            <th>Verteilerschlüssel</th>
            <th className="text-right">Gesamtbetrag</th>
            <th className="text-right">Anteil Wohnung</th>
          </tr>
        </thead>
        <tbody>
          {categories.map((category) => (
            <CostRow
              key={category.id}
              category={category}
              value={costs[category.id] ?? DEFAULT_COST_VALUE}
              period={period}
              {...actions}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}
