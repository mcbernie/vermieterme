import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { DocumentUpload } from "@/components/document-upload";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const json = (data: unknown, ok = true) => ({ ok, json: async () => data }) as Response;

function mount() {
  const { container } = render(
    <UiProvider locale="de-DE">
      <ConfirmProvider>
        <DocumentUpload tenantId="t1" />
      </ConfirmProvider>
    </UiProvider>
  );
  const input = container.querySelector('input[type="file"]') as HTMLInputElement;
  const upload = () =>
    act(async () => {
      const file = new File(["x"], "a.pdf", { type: "application/pdf" });
      fireEvent.change(input, { target: { files: [file] } });
    });
  return { upload };
}

describe("DocumentUpload", () => {
  it("reloads the list after a successful upload", async () => {
    const fetchMock = vi.fn(async (_url: string, init?: RequestInit) =>
      init?.method === "POST" ? json({}) : json([])
    );
    vi.stubGlobal("fetch", fetchMock);
    const { upload } = mount();
    await upload();
    const gets = fetchMock.mock.calls.filter((c) => (c[1]?.method ?? "GET") === "GET");
    expect(gets.length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText("Das Dokument konnte nicht hochgeladen werden.")).toBeNull();
  });

  it("shows a message when the server rejects the upload", async () => {
    vi.stubGlobal("fetch", vi.fn(async (_url: string, init?: RequestInit) => (init?.method === "POST" ? json({}, false) : json([]))));
    const { upload } = mount();
    await upload();
    expect(screen.getByText("Das Dokument konnte nicht hochgeladen werden.")).toBeInTheDocument();
  });

  it("shows a message on a network error", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_url: string, init?: RequestInit) => {
        if (init?.method === "POST") throw new Error("offline");
        return json([]);
      })
    );
    const { upload } = mount();
    await upload();
    expect(screen.getByText("Das Dokument konnte nicht hochgeladen werden.")).toBeInTheDocument();
  });
});
