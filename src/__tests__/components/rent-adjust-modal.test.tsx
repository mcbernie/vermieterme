import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { isUnreadableAmount, RentAdjustModal } from "@/features/tenants/rent-adjust-modal";

afterEach(cleanup);

function setup(onSave = vi.fn().mockResolvedValue(null), onClose = vi.fn()) {
  render(
    <UiProvider locale="de-DE">
      <RentAdjustModal unitId="u1" tenantName="Frau Erika Muster" defaultType="prepayment" onSave={onSave} onClose={onClose} />
    </UiProvider>,
  );
  const money = screen.getByLabelText(/Neuer Betrag/) as HTMLInputElement;
  const date = screen.getByLabelText(/Gültig ab/) as HTMLInputElement;
  const form = money.closest("form") as HTMLFormElement;
  return { onSave, onClose, money, date, form };
}

/** Types an amount and commits it like the user would (Enter). */
function typeAmount(input: HTMLInputElement, text: string) {
  fireEvent.focus(input);
  fireEvent.change(input, { target: { value: text } });
  fireEvent.keyDown(input, { key: "Enter" });
}

describe("RentAdjustModal", () => {
  it("requires an amount", () => {
    const { onSave, form } = setup();
    fireEvent.submit(form);
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte einen Betrag angeben.")).toBeInTheDocument();
  });

  it("saves the amount in euros with the chosen type and closes", async () => {
    const { onSave, onClose, money, date, form } = setup();
    fireEvent.click(screen.getByRole("radio", { name: "Kaltmiete" }));
    typeAmount(money, "650,50");
    fireEvent.change(date, { target: { value: "01.05.2026" } });
    fireEvent.blur(date);
    await act(async () => {
      fireEvent.submit(form);
    });
    expect(onClose).toHaveBeenCalled();
    expect(onSave).toHaveBeenCalledWith({
      unitId: "u1",
      type: "rent",
      amount: 650.5,
      effectiveDate: "2026-05-01",
      reason: null,
    });
  });

  it("blocks saving for an unreadable date", () => {
    const { onSave, money, date, form } = setup();
    typeAmount(money, "10");
    fireEvent.change(date, { target: { value: "31.02.2026" } });
    fireEvent.blur(date);
    fireEvent.submit(form);
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein gültiges Datum eingeben.")).toBeInTheDocument();
  });

  it("blocks saving when the amount text became unreadable after a valid amount", () => {
    const { onSave, money, form } = setup();
    typeAmount(money, "10");
    typeAmount(money, "zehn");
    fireEvent.submit(form);
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte einen gültigen Betrag eingeben.")).toBeInTheDocument();
  });

  it("shows the server error and stays open", async () => {
    const { money, form, onClose } = setup(vi.fn().mockResolvedValue("Nicht angemeldet"));
    typeAmount(money, "10");
    fireEvent.submit(form);
    expect(await screen.findByText("Nicht angemeldet")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("isUnreadableAmount", () => {
  it("accepts empty and readable amounts", () => {
    expect(isUnreadableAmount("")).toBe(false);
    expect(isUnreadableAmount("1.234,50 €")).toBe(false);
    expect(isUnreadableAmount("12")).toBe(false);
    expect(isUnreadableAmount("abc")).toBe(true);
  });
});
