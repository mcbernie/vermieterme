import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EMPTY_LANDLORD, landlordErrors, sameLandlord, toLandlordForm } from "@/features/settings/landlord";
import { LandlordForm } from "@/features/settings/landlord-form";
import { SESSION_EXPIRED } from "@/lib/api-client";
import { json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

afterEach(() => vi.unstubAllGlobals());

const VALID = { ...EMPTY_LANDLORD, name: "Max", street: "Weg 1", zip: "10115", city: "Berlin" };
const submit = () => fireEvent.click(screen.getByRole("button", { name: "Speichern" }));

describe("landlord helpers", () => {
  it("maps a missing record and null fields to empty strings", () => {
    expect(toLandlordForm(null)).toEqual(EMPTY_LANDLORD);
    expect(toLandlordForm({ id: "l1", name: "Max", phone: null })).toEqual({ ...EMPTY_LANDLORD, id: "l1", name: "Max" });
  });

  it("requires the address; an invalid IBAN is no blocking error", () => {
    expect(Object.keys(landlordErrors(EMPTY_LANDLORD)).sort()).toEqual(["city", "name", "street", "zip"]);
    expect(landlordErrors(VALID)).toEqual({});
    expect(landlordErrors({ ...VALID, iban: "DE00 1234" })).toEqual({});
  });
});

describe("sameLandlord", () => {
  it("treats a differently formatted IBAN as the same value", () => {
    expect(sameLandlord({ ...VALID, iban: "de89370400440532013000" }, { ...VALID, iban: "DE89 3704 0044 0532 0130 00" })).toBe(
      true,
    );
  });

  it("still reports a different IBAN or field (counter-check)", () => {
    expect(sameLandlord({ ...VALID, iban: "DE89370400440532013000" }, { ...VALID, iban: "DE89370400440532013001" })).toBe(false);
    expect(sameLandlord(VALID, { ...VALID, city: "Hamburg" })).toBe(false);
  });
});

describe("LandlordForm", () => {
  it("shows the field errors on submit and sends nothing; the button stays enabled", async () => {
    const { calls } = mockFetch({});
    renderWithUi(<LandlordForm initial={EMPTY_LANDLORD} />);
    expect(screen.getByRole("button", { name: "Speichern" })).toBeEnabled();
    submit();
    expect(await screen.findByText("Bitte einen Namen angeben.")).toBeInTheDocument();
    expect(screen.getByLabelText(/Straße/)).toHaveAttribute("aria-invalid", "true");
    expect(calls).toHaveLength(0);
  });

  it("shows an invalid IBAN only as a hint and still saves", async () => {
    const { calls } = mockFetch({ "POST /api/settings": json({ ...VALID, id: "l1" }) });
    renderWithUi(<LandlordForm initial={{ ...VALID, iban: "DE00" }} />);
    expect(screen.getByText("Ungültige IBAN")).toBeInTheDocument();
    submit();
    expect(await screen.findByText("Gespeichert")).toBeInTheDocument();
    expect(calls).toHaveLength(1);
    expect(calls[0]?.body).toMatchObject({ iban: "DE00" });
  });

  it("saves the form once on a double submit and confirms it", async () => {
    let release: (r: Response) => void = () => {};
    const { calls } = mockFetch({ "POST /api/settings": () => new Promise<Response>((r) => (release = r)) });
    renderWithUi(<LandlordForm initial={VALID} />);
    submit();
    await waitFor(() => expect(screen.getByRole("button", { name: /Speichern/ })).toBeDisabled());
    // The disabled button cannot be clicked; a second form submit (e.g. Enter) must not send again.
    fireEvent.submit(screen.getByRole("form", { name: "Vermieter-Informationen" }));
    release(json({ ...VALID, id: "l1" }));
    expect(await screen.findByText("Gespeichert")).toBeInTheDocument();
    expect(calls).toHaveLength(1);
    expect(calls[0]?.body).toEqual(VALID);
  });

  it("does not confirm the save when the form was changed meanwhile", async () => {
    let release: (r: Response) => void = () => {};
    mockFetch({ "POST /api/settings": () => new Promise<Response>((r) => (release = r)) });
    renderWithUi(<LandlordForm initial={VALID} />);
    submit();
    fireEvent.change(screen.getByLabelText(/Stadt/), { target: { value: "Bremen" } });
    release(json({ ...VALID, id: "l1" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Speichern" })).toBeEnabled());
    expect(screen.queryByText("Gespeichert")).not.toBeInTheDocument();
  });

  it("reports an expired session visibly", async () => {
    mockFetch({ "POST /api/settings": loginPage() });
    renderWithUi(<LandlordForm initial={VALID} />);
    submit();
    expect(await screen.findByText(SESSION_EXPIRED)).toBeInTheDocument();
    expect(screen.queryByText("Gespeichert")).not.toBeInTheDocument();
  });

  it("formats the IBAN when leaving the field", () => {
    mockFetch({});
    renderWithUi(<LandlordForm initial={VALID} />);
    const iban = screen.getByLabelText("IBAN");
    fireEvent.change(iban, { target: { value: "de89370400440532013000" } });
    fireEvent.blur(iban);
    expect(iban).toHaveValue("DE89 3704 0044 0532 0130 00");
  });
});
