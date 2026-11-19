import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CostCategoriesSection, nextSortOrder } from "@/features/settings/cost-categories-section";
import { CostCategoryModal } from "@/features/settings/cost-category-modal";
import { requestJson, SESSION_EXPIRED } from "@/features/settings/request";
import { newUserErrors, UserFormModal } from "@/features/settings/user-form-modal";
import { UsersSection } from "@/features/settings/users-section";
import { json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

afterEach(() => vi.unstubAllGlobals());

const CATEGORY = { id: "c1", name: "Grundsteuer", distributionKey: "laut Bescheid", sortOrder: 3 };
const USER = { id: "u1", name: "Anna", email: "anna@beispiel.de" };

describe("cost categories", () => {
  it("proposes the position after the last category", () => {
    expect(nextSortOrder([])).toBe(1);
    expect(nextSortOrder([CATEGORY, { ...CATEGORY, id: "c2", sortOrder: 7 }])).toBe(8);
  });

  it("requires a name and keeps the dialog open with the error of a failed save", async () => {
    const onSave = vi.fn().mockRejectedValueOnce(new Error("intern"));
    const onClose = vi.fn();
    renderWithUi(<CostCategoryModal nextSortOrder={4} onSave={onSave} onClose={onClose} />);
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText("Bitte einen Namen angeben.")).toBeInTheDocument();
    expect(onSave).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText(/Name/), { target: { value: "Müll" } });
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText("Fehler beim Speichern")).toBeInTheDocument();
    expect(onSave).toHaveBeenCalledWith({ name: "Müll", distributionKey: "MEA", sortOrder: 4 });
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("lists categories and updates one through the dialog", async () => {
    const { calls } = mockFetch({
      "GET /api/cost-categories": json([CATEGORY]),
      "PUT /api/cost-categories/c1": json(CATEGORY),
    });
    renderWithUi(<CostCategoriesSection />);
    expect(await screen.findByText("Grundsteuer")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Bearbeiten" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByLabelText(/Name/)).toHaveValue("Grundsteuer");
    fireEvent.click(within(dialog).getByRole("button", { name: "Speichern" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(calls.find((c) => c.method === "PUT")?.body).toEqual({
      name: "Grundsteuer",
      distributionKey: "laut Bescheid",
      sortOrder: 3,
    });
    expect(calls.filter((c) => c.method === "GET")).toHaveLength(2);
  });

  it("shows a load error with a retry instead of an empty list", async () => {
    mockFetch({ "GET /api/cost-categories": loginPage() });
    renderWithUi(<CostCategoriesSection />);
    expect(await screen.findByText(SESSION_EXPIRED)).toBeInTheDocument();
    expect(screen.queryByText(/Noch keine Kostenarten/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Neue Kostenart" })).toBeDisabled();
  });
});

describe("users", () => {
  it("checks e-mail and password length", () => {
    expect(newUserErrors({ name: "", email: "", password: "kurz" })).toEqual({
      email: "Bitte eine E-Mail-Adresse angeben.",
      password: "Passwort muss mindestens 8 Zeichen lang sein",
    });
    expect(newUserErrors({ name: "", email: "a@b.de", password: "12345678" })).toEqual({});
  });

  it("shows the API message of a refused user and stays open", async () => {
    const onClose = vi.fn();
    mockFetch({ "POST /api/users": json({ error: "Ein Benutzer mit dieser E-Mail existiert bereits" }, 409) });
    renderWithUi(<UsersCreate onClose={onClose} />);
    fireEvent.change(screen.getByLabelText(/E-Mail/), { target: { value: "anna@beispiel.de" } });
    fireEvent.change(screen.getByLabelText(/^Passwort/, { selector: "input" }), { target: { value: "geheim123" } });
    fireEvent.click(screen.getByRole("button", { name: "Erstellen" }));
    expect(await screen.findByText("Ein Benutzer mit dieser E-Mail existiert bereits")).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("shows a failed delete visibly", async () => {
    mockFetch({
      "GET /api/users": json([USER]),
      "DELETE /api/users/u1": json({ error: "Sie können sich nicht selbst löschen" }, 400),
    });
    renderWithUi(<UsersSection />);
    fireEvent.click(await screen.findByRole("button", { name: "Löschen" }));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Löschen" }));
    expect(await screen.findByText("Sie können sich nicht selbst löschen")).toBeInTheDocument();
  });
});

/** The user dialog wired to the real API call of the section. */
function UsersCreate({ onClose }: { onClose: () => void }) {
  return (
    <UserFormModal
      onCreate={async (values) => {
        await requestJson("/api/users", "Fehler beim Erstellen", { method: "POST", json: values });
      }}
      onClose={onClose}
    />
  );
}
