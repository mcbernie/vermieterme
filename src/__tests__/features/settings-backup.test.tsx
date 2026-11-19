import "@testing-library/jest-dom/vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { backupFileName, downloadBackup, importBackup } from "@/features/settings/backup";
import { BackupSection } from "@/features/settings/backup-section";
import { SESSION_EXPIRED } from "@/lib/api-client";
import { fakeResponse, json, loginPage, mockFetch, renderWithUi } from "./settings-helpers";

let clicked: string[] = [];

beforeEach(() => {
  clicked = [];
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
    clicked.push(this.download);
  });
  vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:x"), revokeObjectURL: vi.fn() }));
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const backupResponse = () =>
  fakeResponse({ version: 1 }, { headers: { "Content-Disposition": 'attachment; filename="vermieterme-backup-2026-01-02.json"' } });

/** Picks a file in the hidden input of the backup section. */
function pickFile(content: string) {
  const file = new File([content], "backup.json", { type: "application/json" });
  fireEvent.change(screen.getByTestId("backup-file"), { target: { files: [file] } });
}

describe("backup helpers", () => {
  it("takes the file name from the header, otherwise the date", () => {
    expect(backupFileName('attachment; filename="a.json"', new Date("2026-03-04T10:00:00Z"))).toBe("a.json");
    expect(backupFileName(null, new Date("2026-03-04T10:00:00Z"))).toBe("vermieterme-backup-2026-03-04.json");
  });

  it("downloads the backup under the server's file name", async () => {
    mockFetch({ "GET /api/backup": backupResponse() });
    await downloadBackup();
    expect(clicked).toEqual(["vermieterme-backup-2026-01-02.json"]);
  });

  it("does not download the login page after the session expired", async () => {
    mockFetch({ "GET /api/backup": loginPage() });
    await expect(downloadBackup()).rejects.toThrow(SESSION_EXPIRED);
    expect(clicked).toEqual([]);
  });

  it("keeps kind and status of a failed export answer behind the export text", async () => {
    mockFetch({ "GET /api/backup": json({ error: "x" }, 500) });
    await expect(downloadBackup()).rejects.toMatchObject({ message: "Export fehlgeschlagen", kind: "status", status: 500 });
  });

  it("rejects a file without JSON before sending anything", async () => {
    const { calls } = mockFetch({});
    const error = importBackup(new File(["kein json"], "x.json"));
    await expect(error).rejects.toMatchObject({ message: "Import fehlgeschlagen", kind: "unexpected", status: 0 });
    expect(calls).toHaveLength(0);
  });
});

describe("BackupSection", () => {
  it("reports a successful export and a failed one", async () => {
    mockFetch({ "GET /api/backup": backupResponse() });
    renderWithUi(<BackupSection onImported={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Daten exportieren" }));
    expect(await screen.findByText("Export erfolgreich")).toBeInTheDocument();

    mockFetch({ "GET /api/backup": json({ error: "x" }, 500) });
    fireEvent.click(screen.getByRole("button", { name: "Daten exportieren" }));
    expect(await screen.findByText("Export fehlgeschlagen")).toBeInTheDocument();
  });

  it("imports only after the confirmation and then reloads", async () => {
    const onImported = vi.fn();
    const { calls } = mockFetch({ "POST /api/backup": json({ success: true }) });
    renderWithUi(<BackupSection onImported={onImported} />);
    pickFile('{"version":1}');
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("unwiderruflich");
    expect(calls).toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: "Importieren" }));
    expect(await screen.findByText(/Import erfolgreich/)).toBeInTheDocument();
    expect(calls).toEqual([{ method: "POST", url: "/api/backup", body: { version: 1 } }]);
    expect(onImported).toHaveBeenCalledTimes(1);
  });

  it("sends nothing when the confirmation is cancelled", async () => {
    const { calls } = mockFetch({});
    renderWithUi(<BackupSection onImported={vi.fn()} />);
    pickFile('{"version":1}');
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "Abbrechen" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(calls).toHaveLength(0);
  });

  it("shows the API error of a failed import", async () => {
    const onImported = vi.fn();
    mockFetch({ "POST /api/backup": json({ error: "Ungültiges Backup-Format" }, 400) });
    renderWithUi(<BackupSection onImported={onImported} />);
    pickFile('{"version":2}');
    fireEvent.click(await screen.findByRole("button", { name: "Importieren" }));
    expect(await screen.findByText("Ungültiges Backup-Format")).toBeInTheDocument();
    expect(onImported).not.toHaveBeenCalled();
  });
});
