import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { RentChangeFormModal } from "@/features/rent-changes/rent-change-form-modal";
import type { RentChangeWithUnit } from "@/types";

afterEach(cleanup);

const editing = {
  id: "a",
  unitId: "u1",
  type: "rent",
  amount: 500,
  effectiveDate: "2026-01-01T00:00:00.000Z",
  reason: "Index",
} as unknown as RentChangeWithUnit;

function setup(onSubmit = vi.fn().mockResolvedValue(true), onClose = vi.fn(), edit: RentChangeWithUnit | null = editing) {
  render(
    <UiProvider locale="de-DE">
      <RentChangeFormModal
        open
        editing={edit}
        unitOptions={[{ value: "u1", label: "Weg 1, Bremen – EG" }]}
        onClose={onClose}
        onSubmit={onSubmit}
      />
    </UiProvider>
  );
  const date = screen.getByLabelText(/Gültig ab/) as HTMLInputElement;
  const form = date.closest("form") as HTMLFormElement;
  const submit = () =>
    act(async () => {
      fireEvent.submit(form);
    });
  return { onSubmit, onClose, date, form, submit };
}

describe("RentChangeFormModal", () => {
  it("saves the edited rent change in euros and closes", async () => {
    const { onSubmit, onClose, submit } = setup();
    await submit();
    expect(onSubmit).toHaveBeenCalledWith({
      unitId: "u1",
      type: "rent",
      amount: 500,
      effectiveDate: "2026-01-01",
      reason: "Index",
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("blocks saving and shows an error for an unreadable date", async () => {
    const { onSubmit, date, submit } = setup();
    fireEvent.change(date, { target: { value: "31.02.2026" } });
    fireEvent.blur(date);
    await submit();
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein gültiges Datum eingeben.")).toBeInTheDocument();
  });

  it("asks for date, amount and unit when creating an empty entry", async () => {
    const { onSubmit, submit } = setup(undefined, undefined, null);
    await submit();
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein Datum angeben.")).toBeInTheDocument();
    expect(screen.getByText("Bitte einen Betrag angeben.")).toBeInTheDocument();
    expect(screen.getByText("Bitte eine Wohnung wählen.")).toBeInTheDocument();
  });

  it("stays open and shows a message when saving fails", async () => {
    const { onClose, submit } = setup(vi.fn().mockResolvedValue(false));
    await submit();
    expect(screen.getByText("Das Speichern ist fehlgeschlagen.")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
