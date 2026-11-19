import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { passwordErrors } from "@/features/profile/password";
import { PasswordForm } from "@/features/profile/password-form";
import { ProfileView } from "@/features/profile/profile-view";
import { SESSION_EXPIRED } from "@/features/settings/request";
import { fakeResponse, json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

const updateSession = vi.fn(async () => null);
vi.mock("next-auth/react", () => ({ useSession: () => ({ update: updateSession }) }));

const SECRET = "streng-geheim-42";
const field = (label: RegExp) => screen.getByLabelText(label, { selector: "input" });

/** Fills the password form. */
function fill(current: string, next: string, repeat = next) {
  fireEvent.change(field(/^Aktuelles Passwort/), { target: { value: current } });
  fireEvent.change(field(/^Neues Passwort\s*\*?$/), { target: { value: next } });
  fireEvent.change(field(/^Neues Passwort bestätigen/), { target: { value: repeat } });
}

const submit = () => fireEvent.click(screen.getByRole("button", { name: "Passwort ändern" }));

let consoleSpies: ReturnType<typeof vi.spyOn>[] = [];
beforeEach(() => {
  updateSession.mockClear();
  consoleSpies = [vi.spyOn(console, "error"), vi.spyOn(console, "log"), vi.spyOn(console, "warn")];
});
afterEach(() => {
  // Passwords must never reach the console.
  for (const spy of consoleSpies) expect(JSON.stringify(spy.mock.calls)).not.toContain(SECRET);
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("passwordErrors", () => {
  it("applies the old rules", () => {
    expect(passwordErrors({ currentPassword: "", newPassword: "kurz", confirmPassword: "anders" })).toEqual({
      currentPassword: "Bitte das aktuelle Passwort angeben.",
      newPassword: "Passwort muss mindestens 8 Zeichen lang sein",
      confirmPassword: "Passwörter stimmen nicht überein",
    });
    expect(passwordErrors({ currentPassword: "a", newPassword: SECRET, confirmPassword: SECRET })).toEqual({});
  });
});

describe("PasswordForm", () => {
  it("sends nothing when the repetition differs", async () => {
    const { calls } = mockFetch({});
    renderWithUi(<PasswordForm />);
    fill("alt-passwort", SECRET, "etwas-anderes");
    submit();
    expect(await screen.findByText("Passwörter stimmen nicht überein")).toBeInTheDocument();
    expect(calls).toHaveLength(0);
  });

  it("sends current and new password once and clears the fields on success", async () => {
    const { calls } = mockFetch({ "PUT /api/profile": json({ id: "u1" }) });
    renderWithUi(<PasswordForm />);
    fill("alt-passwort", SECRET);
    submit();
    expect(await screen.findByText("Passwort geändert")).toBeInTheDocument();
    expect(calls).toEqual([
      { method: "PUT", url: "/api/profile", body: { currentPassword: "alt-passwort", newPassword: SECRET } },
    ]);
    expect(field(/^Aktuelles Passwort/)).toHaveValue("");
    expect(field(/^Neues Passwort bestätigen/)).toHaveValue("");
  });

  it("shows the API error and keeps the input", async () => {
    mockFetch({ "PUT /api/profile": json({ error: "Aktuelles Passwort ist falsch" }, 400) });
    renderWithUi(<PasswordForm />);
    fill("falsch", SECRET);
    submit();
    expect(await screen.findByText("Aktuelles Passwort ist falsch")).toBeInTheDocument();
    expect(field(/^Aktuelles Passwort/)).toHaveValue("falsch");
    expect(screen.getByText("Aktuelles Passwort ist falsch").textContent).not.toContain(SECRET);
  });

  it("reports an expired session", async () => {
    mockFetch({ "PUT /api/profile": loginPage() });
    renderWithUi(<PasswordForm />);
    fill("alt-passwort", SECRET);
    submit();
    expect(await screen.findByText(SESSION_EXPIRED)).toBeInTheDocument();
    expect(screen.queryByText("Passwort geändert")).not.toBeInTheDocument();
  });
});

describe("ProfileView", () => {
  it("saves the name, updates the session and reloads the profile", async () => {
    const { calls } = mockFetch({
      "GET /api/profile": json({ id: "u1", name: "Anna", email: "anna@beispiel.de" }),
      "PUT /api/profile": json({ id: "u1", name: "Anna B." }),
    });
    renderWithUi(<ProfileView />);
    const name = await screen.findByLabelText("Name");
    expect(name).toHaveValue("Anna");
    fireEvent.change(name, { target: { value: "Anna B." } });
    fireEvent.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByText("Name gespeichert")).toBeInTheDocument();
    expect(updateSession).toHaveBeenCalledWith({ name: "Anna B." });
    await waitFor(() => expect(calls.filter((c) => c.method === "GET")).toHaveLength(2));
    expect(calls.find((c) => c.method === "PUT")?.body).toEqual({ name: "Anna B." });
  });

  it("shows a load error instead of empty forms", async () => {
    mockFetch({ "GET /api/profile": fakeResponse("Server Error", { status: 500, contentType: "text/plain" }) });
    renderWithUi(<ProfileView />);
    expect(await screen.findByText("Daten konnten nicht geladen werden.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Passwort ändern" })).not.toBeInTheDocument();
  });
});
