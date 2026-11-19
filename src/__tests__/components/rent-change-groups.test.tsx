import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { RentChangeGroupList } from "@/features/rent-changes/rent-change-groups";
import { groupRentChanges } from "@/features/rent-changes/rent-change-utils";
import type { RentChangeWithUnit } from "@/types";

afterEach(cleanup);

const changes = [
  {
    id: "a",
    unitId: "u1",
    type: "rent",
    amount: 500,
    effectiveDate: "2025-01-01",
    reason: "Index",
    unit: { name: "EG", property: { street: "Weg 1", city: "Bremen" } },
  },
  {
    id: "b",
    unitId: "u1",
    type: "prepayment",
    amount: 80,
    effectiveDate: "2026-01-01",
    reason: null,
    unit: { name: "EG", property: { street: "Weg 1", city: "Bremen" } },
  },
] as unknown as RentChangeWithUnit[];

describe("RentChangeGroupList", () => {
  it("renders a table per property with type tags, newest first", () => {
    render(
      <UiProvider locale="de-DE">
        <RentChangeGroupList groups={groupRentChanges(changes)} onEdit={() => undefined} onDelete={() => undefined} />
      </UiProvider>
    );
    expect(screen.getByRole("heading", { name: "Weg 1, Bremen" })).toBeInTheDocument();
    const rows = screen.getAllByRole("row").slice(1);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Vorauszahlung NK");
    expect(rows[1]).toHaveTextContent("Kaltmiete");
    expect(rows[1]).toHaveTextContent("Index");
  });

  it("calls the handlers with the clicked change", () => {
    const onEdit = vi.fn();
    const onDelete = vi.fn();
    render(
      <UiProvider locale="de-DE">
        <RentChangeGroupList groups={groupRentChanges(changes)} onEdit={onEdit} onDelete={onDelete} />
      </UiProvider>
    );
    fireEvent.click(screen.getAllByRole("button", { name: "Bearbeiten" })[0]);
    fireEvent.click(screen.getAllByRole("button", { name: "Löschen" })[1]);
    expect(onEdit.mock.calls[0][0].id).toBe("b");
    expect(onDelete.mock.calls[0][0].id).toBe("a");
  });
});
