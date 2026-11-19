import { apiFetch, apiJson, RequestError, SessionExpiredError } from "@/lib/api-client";

/** Error text of a failed export. */
export const EXPORT_FAILED = "Export fehlgeschlagen";
/** Error text of a failed import without an API message. */
export const IMPORT_FAILED = "Import fehlgeschlagen";

/**
 * File name of a backup: the one from `Content-Disposition`, otherwise `vermieterme-backup-<date>.json`.
 * @param disposition Value of the `Content-Disposition` header, if any.
 * @param today Date used for the fallback name.
 */
export function backupFileName(disposition: string | null, today: Date): string {
  return disposition?.match(/filename="(.+)"/)?.[1] || `vermieterme-backup-${today.toISOString().slice(0, 10)}.json`;
}

/** Offers a blob as a file download through a temporary link. */
function saveBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/**
 * Downloads all data as a JSON file.
 * @throws RequestError when the export fails or the session expired.
 */
export async function downloadBackup(): Promise<void> {
  let res: Response;
  try {
    res = await apiFetch("/api/backup");
  } catch (error) {
    // A failed export always reads „Export fehlgeschlagen“, as before; only an expired session differs.
    if (error instanceof SessionExpiredError) throw error;
    throw new RequestError(EXPORT_FAILED);
  }
  try {
    saveBlob(await res.blob(), backupFileName(res.headers.get("Content-Disposition"), new Date()));
  } catch {
    throw new RequestError(EXPORT_FAILED);
  }
}

/**
 * Replaces all data with the content of a backup file.
 * @throws RequestError when the file is no JSON, the import fails or the session expired.
 */
export async function importBackup(file: File): Promise<void> {
  let data: unknown;
  try {
    data = JSON.parse(await file.text());
  } catch {
    throw new RequestError(IMPORT_FAILED);
  }
  await apiJson("/api/backup", { fallback: IMPORT_FAILED, method: "POST", json: data });
}
