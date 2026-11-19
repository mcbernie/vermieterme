import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LoginForm } from "@/features/auth/login-form";

const push = vi.fn();
const refresh = vi.fn();
const signIn = vi.fn();
const getProviders = vi.fn();
let search = "";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push, refresh }),
  useSearchParams: () => new URLSearchParams(search),
}));
vi.mock("next-auth/react", () => ({
  signIn: (...args: unknown[]) => signIn(...args),
  getProviders: () => getProviders(),
  useSession: () => ({ status: "unauthenticated" }),
}));

/** Renders the form and waits for the provider lookup, so no state update lands after the test. */
async function renderForm() {
  render(<LoginForm />);
  await waitFor(() => expect(getProviders).toHaveBeenCalled());
}

function fill(email: string, password: string) {
  fireEvent.change(screen.getByLabelText(/E-Mail/), { target: { value: email } });
  fireEvent.change(screen.getByLabelText("Passwort"), { target: { value: password } });
}

const submit = () => fireEvent.click(screen.getByRole("button", { name: "Anmelden" }));

describe("LoginForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    search = "";
    getProviders.mockResolvedValue({ credentials: {} });
    signIn.mockResolvedValue({ error: null });
  });

  it("signs in with the entered credentials and redirects to the callback url", async () => {
    search = "callbackUrl=%2Fbilling";
    await renderForm();
    fill("a@b.de", "geheim");
    submit();
    await waitFor(() => expect(push).toHaveBeenCalledWith("/billing"));
    expect(signIn).toHaveBeenCalledWith("credentials", { email: "a@b.de", password: "geheim", redirect: false });
    expect(refresh).toHaveBeenCalled();
  });

  it("ignores a foreign callback url after sign-in", async () => {
    search = "callbackUrl=https%3A%2F%2Fx.de";
    await renderForm();
    fill("a@b.de", "geheim");
    submit();
    await waitFor(() => expect(push).toHaveBeenCalledWith("/"));
  });

  it("shows the field errors on submit and does not sign in", async () => {
    await renderForm();
    submit();
    expect(await screen.findByText("Bitte geben Sie Ihre E-Mail-Adresse ein.")).toBeInTheDocument();
    expect(screen.getByText("Bitte geben Sie Ihr Passwort ein.")).toBeInTheDocument();
    expect(signIn).not.toHaveBeenCalled();
  });

  it("shows a notice and re-enables the button when the credentials are wrong", async () => {
    signIn.mockResolvedValue({ error: "CredentialsSignin" });
    await renderForm();
    fill("a@b.de", "falsch");
    submit();
    expect(await screen.findByText("Ungültige Anmeldedaten")).toBeInTheDocument();
    expect(push).not.toHaveBeenCalled();
    await waitFor(() => expect(screen.getByRole("button", { name: "Anmelden" })).toBeEnabled());
  });

  it("signs in only once on a double submit", async () => {
    let resolve: (value: { error: null }) => void = () => {};
    signIn.mockReturnValue(new Promise((r) => (resolve = r)));
    await renderForm();
    fill("a@b.de", "geheim");
    const form = screen.getByRole("button", { name: "Anmelden" }).closest("form") as HTMLFormElement;
    fireEvent.submit(form);
    fireEvent.submit(form);
    expect(signIn).toHaveBeenCalledTimes(1);
    resolve({ error: null });
    await waitFor(() => expect(push).toHaveBeenCalledTimes(1));
  });

  it("offers Apple only when the provider is configured", async () => {
    getProviders.mockResolvedValue({ credentials: {}, apple: {} });
    await renderForm();
    fireEvent.click(await screen.findByRole("button", { name: "Mit Apple anmelden" }));
    expect(signIn).toHaveBeenCalledWith("apple", { callbackUrl: "/" });
  });

  it("hides Apple without the provider", async () => {
    await renderForm();
    expect(screen.queryByRole("button", { name: "Mit Apple anmelden" })).not.toBeInTheDocument();
  });
});
