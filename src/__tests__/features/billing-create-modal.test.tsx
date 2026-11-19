import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { UiProvider } from "@zweiund40gmbh/ui";
import { BillingCreateModal } from "@/features/billing/billing-create-modal";
import type { WriteResult } from "@/features/billing/billing-http";
import type { BillingPeriodWithProperty, Property } from "@/types";

vi.mock("@zweiund40gmbh/ui", async (orig) => ({
  ...(await orig<typeof import("@zweiund40gmbh/ui")>()),
  Select: (await import("../select-stub")).SelectStub,
}));

afterEach(cleanup);

const property: Property = { id: "p1", street: "Weg 1", zip: "1", city: "Bremen", totalShares: 100 };
const previous: BillingPeriodWithProperty = {
  id: "bp1",
  propertyId: "p1",
  startDate: "2024-01-01T00:00:00.000Z",
  endDate: "2024-12-31T00:00:00.000Z",
  billingDate: null,
  sentDate: null,
  paidDate: null,
  copiedFromId: null,
  property,
};

function setup(onCreate = vi.fn(async (): Promise<WriteResult> => ({ ok: true, error: null }))) {
  const onClose = vi.fn();
  render(
    <UiProvider locale="de-DE">
      <BillingCreateModal properties={[property]} periods={[previous]} onCreate={onCreate} onClose={onClose} />
    </UiProvider>,
  );
  const form = document.getElementById("billing-create-form") as HTMLFormElement;
  const submit = () =>
    act(async () => {
      fireEvent.submit(form);
    });
  return { onCreate, onClose, submit };
}

/** Picks an option of a (stubbed) `Select`. */
async function choose(option: string) {
  await act(async () => {
    fireEvent.click(screen.getByRole("option", { name: option }));
  });
}

/** Types a date into a `DateInput` and leaves the field. */
function typeDate(label: RegExp, text: string) {
  const input = screen.getByLabelText(label);
  fireEvent.change(input, { target: { value: text } });
  fireEvent.blur(input);
}

describe("BillingCreateModal", () => {
  it("asks for property and dates instead of submitting an empty form", async () => {
    const { onCreate, submit } = setup();
    await submit();
    expect(onCreate).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein Objekt wählen.")).toBeInTheDocument();
    expect(screen.getAllByText("Bitte ein Datum angeben.")).toHaveLength(2);
  });

  it("does not submit an unreadable date and shows the error at the field", async () => {
    const { onCreate, submit } = setup();
    await choose("Weg 1, Bremen");
    typeDate(/Beginn/, "31.02.2025");
    typeDate(/Ende/, "31.12.2025");
    await submit();
    expect(onCreate).not.toHaveBeenCalled();
    expect(screen.getByText("Bitte ein gültiges Datum eingeben.")).toBeInTheDocument();
  });

  it("prefills the dates from the copied period and creates once on a double submit", async () => {
    let resolve: (r: WriteResult) => void = () => {};
    const onCreate = vi.fn(() => new Promise<WriteResult>((r) => (resolve = r)));
    const { onClose, submit } = setup(onCreate);
    await choose("Weg 1, Bremen");
    await choose("1.1.2024 – 31.12.2024");
    await submit();
    await submit();
    expect(onCreate).toHaveBeenCalledTimes(1);
    expect(onCreate).toHaveBeenCalledWith({
      propertyId: "p1",
      startDate: "2025-01-01",
      endDate: "2025-12-31",
      billingDate: null,
      copyFromId: "bp1",
    });
    await act(async () => resolve({ ok: true, error: null }));
    expect(onClose).toHaveBeenCalled();
  });

  it("shows the server message and stays open when creating fails", async () => {
    const onCreate = vi.fn(async (): Promise<WriteResult> => ({ ok: false, error: "Zeitraum überschneidet sich" }));
    const { onClose, submit } = setup(onCreate);
    await choose("Weg 1, Bremen");
    typeDate(/Beginn/, "01.01.2025");
    typeDate(/Ende/, "31.12.2025");
    await submit();
    expect(onCreate).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Zeitraum überschneidet sich")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
