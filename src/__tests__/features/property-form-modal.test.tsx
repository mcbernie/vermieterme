import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PropertyFormModal } from "@/features/properties/property-form-modal";

function fill() {
  fireEvent.change(screen.getByLabelText(/Straße/), { target: { value: "Weg 1" } });
  fireEvent.change(screen.getByLabelText(/PLZ/), { target: { value: "10115" } });
  fireEvent.change(screen.getByLabelText(/Stadt/), { target: { value: "Berlin" } });
}

const submit = () => fireEvent.submit(document.getElementById("property-form") as HTMLFormElement);

describe("PropertyFormModal", () => {
  it("submits the entered values with the default shares and closes", async () => {
    const onSave = vi.fn(async () => true);
    const onClose = vi.fn();
    render(<PropertyFormModal onSave={onSave} onClose={onClose} />);
    fill();
    submit();
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(onSave).toHaveBeenCalledWith({ street: "Weg 1", zip: "10115", city: "Berlin", totalShares: 100 });
  });

  it("prefills the values when editing", () => {
    const property = { id: "p", street: "Alt 2", zip: "1", city: "Köln", totalShares: 50 };
    render(<PropertyFormModal property={property} onSave={async () => true} onClose={vi.fn()} />);
    expect(screen.getByLabelText(/Straße/)).toHaveValue("Alt 2");
    expect(screen.getByLabelText(/Gesamtanteile/)).toHaveValue(50);
  });

  it("saves only once on a double submit", async () => {
    let resolve: (ok: boolean) => void = () => {};
    const onSave = vi.fn(() => new Promise<boolean>((r) => (resolve = r)));
    const onClose = vi.fn();
    render(<PropertyFormModal onSave={onSave} onClose={onClose} />);
    fill();
    submit();
    submit();
    expect(onSave).toHaveBeenCalledTimes(1);
    resolve(true);
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("shows an error, stays open and allows a retry when saving fails or throws", async () => {
    const onSave = vi.fn().mockRejectedValueOnce(new Error("boom")).mockResolvedValueOnce(false);
    const onClose = vi.fn();
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(<PropertyFormModal onSave={onSave} onClose={onClose} />);
    fill();
    submit();
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    submit();
    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(2));
    expect(await screen.findByText(/Speichern fehlgeschlagen/)).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
