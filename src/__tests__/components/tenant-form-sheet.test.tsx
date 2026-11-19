import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { TenantFormSheet } from "@/features/tenants/tenant-form-sheet";
import { PROPERTY, TENANT } from "../features/tenant-fixtures";

afterEach(cleanup);

/** Renders the sheet; `edit` preloads the sample tenant. */
function setup({ edit = false, onSave = vi.fn().mockResolvedValue(null), onClose = vi.fn() } = {}) {
  render(
    <UiProvider locale="de-DE">
      <TenantFormSheet
        tenant={edit ? TENANT : undefined}
        properties={[PROPERTY]}
        onSave={onSave}
        onClose={onClose}
      />
    </UiProvider>,
  );
  const moveIn = screen.getByLabelText(/Einzugsdatum/) as HTMLInputElement;
  const form = moveIn.closest("form") as HTMLFormElement;
  return { onSave, onClose, moveIn, form };
}

describe("TenantFormSheet", () => {
  it("shows the field errors of an empty form on submit and does not save", () => {
    const { onSave, form } = setup();
    fireEvent.submit(form);
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte eine Wohnung wählen.")).toBeInTheDocument();
    expect(screen.getByText("Bitte einen Vornamen angeben.")).toBeInTheDocument();
    expect(screen.getByText("Bitte einen Nachnamen angeben.")).toBeInTheDocument();
    expect(screen.getByText("Bitte ein Datum angeben.")).toBeInTheDocument();
  });

  it("keeps the save button enabled while the form is incomplete", () => {
    setup();
    expect(screen.getByRole("button", { name: "Speichern" })).toBeEnabled();
  });

  it("saves the edited values and closes", async () => {
    const { onSave, onClose, form } = setup({ edit: true });
    // The first "Vorname" belongs to the main tenant, the second to the optional second tenant.
    fireEvent.change(screen.getAllByLabelText(/^Vorname/)[0], { target: { value: "Erika Maria" } });
    await act(async () => {
      fireEvent.submit(form);
    });
    expect(onClose).toHaveBeenCalled();
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onSave.mock.calls[0][0]).toMatchObject({
      unitId: "u1",
      firstName: "Erika Maria",
      moveInDate: "2022-04-01",
      leaseType: "index",
    });
  });

  it("blocks saving and shows an error for an unreadable move-in date", () => {
    const { onSave, moveIn, form } = setup({ edit: true });
    fireEvent.change(moveIn, { target: { value: "31.02.2026" } });
    fireEvent.blur(moveIn);
    expect(screen.getByText("Bitte ein gültiges Datum eingeben.")).toBeInTheDocument();
    fireEvent.submit(form);
    expect(onSave).not.toHaveBeenCalled();
  });

  it("rejects an invalid IBAN at the field", () => {
    const { onSave, form } = setup({ edit: true });
    fireEvent.change(screen.getByLabelText("IBAN"), { target: { value: "DE00 1234" } });
    fireEvent.submit(form);
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByText("Ungültige IBAN")).toBeInTheDocument();
  });

  it("keeps the sheet open and shows the server error when saving fails", async () => {
    const onSave = vi.fn().mockResolvedValue("Die Anfrage ist fehlgeschlagen (Status 500).");
    const { onClose, form } = setup({ edit: true, onSave });
    fireEvent.submit(form);
    expect(await screen.findByText("Die Anfrage ist fehlgeschlagen (Status 500).")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("shows the index fields only for index leases", () => {
    setup({ edit: true });
    expect(screen.getByLabelText("VPI Basisjahr")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("radio", { name: "Standard" }));
    expect(screen.queryByLabelText("VPI Basisjahr")).not.toBeInTheDocument();
  });
});
