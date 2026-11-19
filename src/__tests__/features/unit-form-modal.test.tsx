import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UnitFormModal } from "@/features/properties/unit-form-modal";

function fill() {
  fireEvent.change(screen.getByLabelText(/Bezeichnung/), { target: { value: "W1" } });
  fireEvent.change(screen.getByLabelText(/Stockwerk/), { target: { value: "EG" } });
}

describe("UnitFormModal", () => {
  it("saves only once on a double submit", async () => {
    let resolve: (ok: boolean) => void = () => {};
    const onSave = vi.fn(() => new Promise<boolean>((r) => (resolve = r)));
    const onClose = vi.fn();
    render(<UnitFormModal onSave={onSave} onClose={onClose} />);
    fill();
    const form = document.getElementById("unit-form") as HTMLFormElement;
    fireEvent.submit(form);
    fireEvent.submit(form);
    expect(onSave).toHaveBeenCalledTimes(1);
    resolve(true);
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("shows an error and stays open when saving fails", async () => {
    const onClose = vi.fn();
    render(<UnitFormModal onSave={async () => false} onClose={onClose} />);
    fill();
    fireEvent.submit(document.getElementById("unit-form") as HTMLFormElement);
    expect(await screen.findByText(/Speichern fehlgeschlagen/)).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
