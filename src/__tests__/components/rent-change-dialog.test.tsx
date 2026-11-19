import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { RentChangeDialog } from "@/components/rent-change-dialog";

afterEach(cleanup);

function setup(onSubmit = vi.fn(), onOpenChange = vi.fn()) {
  render(
    <UiProvider locale="de-DE">
      <RentChangeDialog
        open
        onOpenChange={onOpenChange}
        onSubmit={onSubmit}
        title="Anpassung"
        defaultAmount={500}
        defaultEffectiveDate="2026-01-01"
      />
    </UiProvider>
  );
  const date = screen.getByLabelText(/Gültig ab/) as HTMLInputElement;
  const money = screen.getByLabelText(/Neuer Betrag/) as HTMLInputElement;
  const form = date.closest("form") as HTMLFormElement;
  return { onSubmit, onOpenChange, date, money, form };
}

describe("RentChangeDialog", () => {
  it("submits the seeded values in euros", async () => {
    const { onSubmit, form } = setup();
    await act(async () => {
      fireEvent.submit(form);
    });
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith({ amount: 500, effectiveDate: "2026-01-01", reason: null });
  });

  it("blocks submitting and shows an error for an unreadable date", () => {
    const { onSubmit, date, form } = setup();
    fireEvent.change(date, { target: { value: "31.02.2026" } });
    fireEvent.blur(date);
    fireEvent.submit(form);
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein gültiges Datum eingeben.")).toBeInTheDocument();
  });

  it("shows an error for an empty date after a submit attempt", () => {
    const { onSubmit, date, form } = setup();
    fireEvent.change(date, { target: { value: "" } });
    fireEvent.blur(date);
    fireEvent.submit(form);
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein Datum angeben.")).toBeInTheDocument();
  });

  it("submits the amount entered before Enter", async () => {
    const { onSubmit, money, form } = setup();
    fireEvent.change(money, { target: { value: "650,50" } });
    fireEvent.keyDown(money, { key: "Enter" });
    await act(async () => {
      fireEvent.submit(form);
    });
    expect(onSubmit).toHaveBeenCalled();
    expect(onSubmit.mock.calls[0][0].amount).toBe(650.5);
  });

  it("stays open with the inputs and shows the message when onSubmit fails", async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error("Speichern nicht möglich."));
    const { onOpenChange, money, form } = setup(onSubmit);
    fireEvent.change(money, { target: { value: "650,50" } });
    fireEvent.blur(money);
    await act(async () => {
      fireEvent.submit(form);
    });
    expect(await screen.findByText("Speichern nicht möglich.")).toBeInTheDocument();
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
    expect(money.value).toContain("650");
  });
});
