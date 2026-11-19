import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { TenantAccessToken } from "@/components/tenant-access-token";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

/** Stubs `fetch` with answers per HTTP method. */
function stubFetch(answers: Record<string, { status: number; body: unknown }>) {
  const fetchMock = vi.fn((_url: string, init?: RequestInit) => {
    const a = answers[init?.method ?? "GET"] ?? { status: 500, body: {} };
    return Promise.resolve(new Response(JSON.stringify(a.body), { status: a.status }));
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function setup() {
  render(
    <UiProvider locale="de-DE">
      <ConfirmProvider>
        <TenantAccessToken tenantId="t1" tenantName="Erika Muster" onClose={() => undefined} />
      </ConfirmProvider>
    </UiProvider>,
  );
}

describe("TenantAccessToken", () => {
  it("shows the existing code", async () => {
    stubFetch({ GET: { status: 200, body: { hasToken: true, token: "ABC123" } } });
    setup();
    expect(await screen.findByText("ABC123")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Zugang widerrufen" })).toBeInTheDocument();
  });

  it("generates a code when there is none", async () => {
    const fetchMock = stubFetch({
      GET: { status: 200, body: { hasToken: false, token: null } },
      POST: { status: 201, body: { token: "XYZ789" } },
    });
    setup();
    fireEvent.click(await screen.findByRole("button", { name: "Einladungscode generieren" }));
    expect(await screen.findByText("XYZ789")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith("/api/tenants/t1/access-token", { method: "POST" });
  });

  it("revokes the code after confirmation", async () => {
    const fetchMock = stubFetch({
      GET: { status: 200, body: { hasToken: true, token: "ABC123" } },
      DELETE: { status: 200, body: { success: true } },
    });
    setup();
    fireEvent.click(await screen.findByRole("button", { name: "Zugang widerrufen" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Widerrufen" }));
    expect(await screen.findByRole("button", { name: "Einladungscode generieren" })).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith("/api/tenants/t1/access-token", { method: "DELETE" });
  });

  it("shows a failed request", async () => {
    stubFetch({ GET: { status: 500, body: { error: "Internal Server Error" } } });
    setup();
    expect(await screen.findByText("Die Anfrage ist fehlgeschlagen (Status 500).")).toBeInTheDocument();
  });
});
