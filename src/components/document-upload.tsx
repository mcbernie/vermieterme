"use client";

import { useState, useEffect, useCallback } from "react";
import { Button, FileDropzone, InlineNotice, formatBytes, useConfirm, useFormat } from "@zweiund40gmbh/ui";
import { apiLoad, apiSend, isArray } from "@/lib/api-client";
import type { Document } from "@/types";

interface DocumentUploadProps {
  billingPeriodId?: string;
  tenantId?: string;
  category?: string;
  label?: string;
  accept?: string;
}

/** Lists the documents of a billing period or tenant and uploads new ones through a dropzone. */
export function DocumentUpload({
  billingPeriodId,
  tenantId,
  category = "other",
  label = "Dokument hochladen",
  accept = ".pdf,.jpg,.jpeg,.png,.webp",
}: DocumentUploadProps) {
  const confirm = useConfirm();
  const format = useFormat();
  const [documents, setDocuments] = useState<Document[]>([]);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);

  const fetchDocuments = useCallback(async () => {
    const params = new URLSearchParams();
    if (billingPeriodId) params.set("billingPeriodId", billingPeriodId);
    if (tenantId) params.set("tenantId", tenantId);

    const docs = await apiLoad<Document[]>(`/api/documents?${params}`, { accept: isArray });
    if (docs) setDocuments(category ? docs.filter((d) => d.category === category) : docs);
  }, [billingPeriodId, tenantId, category]);

  useEffect(() => {
    fetchDocuments();
  }, [fetchDocuments]);

  async function handleUpload(files: File[]) {
    const file = files[0];
    if (!file) return;

    setUploading(true);
    setUploadError(null);
    const formData = new FormData();
    formData.append("file", file);
    formData.append("category", category);
    if (billingPeriodId) formData.append("billingPeriodId", billingPeriodId);
    if (tenantId) formData.append("tenantId", tenantId);
    try {
      if (await apiSend("/api/documents", { method: "POST", form: formData })) {
        await fetchDocuments();
      } else {
        setUploadError("Das Dokument konnte nicht hochgeladen werden.");
      }
    } finally {
      setUploading(false);
    }
  }

  async function handleDelete(id: string) {
    const ok = await confirm({
      title: "Dokument entfernen",
      message: "Möchten Sie dieses Dokument wirklich löschen?",
      confirmLabel: "Löschen",
      danger: true,
    });
    if (!ok) return;
    if (await apiSend(`/api/documents/${id}`, { method: "DELETE" })) await fetchDocuments();
  }

  return (
    <div className="grid gap-3">
      {documents.length > 0 && (
        <ul className="border border-line bg-layer">
          {documents.map((doc) => (
            <li key={doc.id} className="flex items-center justify-between gap-3 border-b border-line px-3 py-2 last:border-b-0">
              <a
                href={`/api/documents/${doc.id}/file`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex min-w-0 items-baseline gap-2 text-sm text-accentstrong hover:underline"
              >
                <span className="truncate">{doc.originalName}</span>
                <span className="num shrink-0 text-xs text-ink3">{formatBytes(doc.size, format.number)}</span>
              </a>
              <Button size="sm" variant="danger" onClick={() => handleDelete(doc.id)}>
                Entfernen
              </Button>
            </li>
          ))}
        </ul>
      )}

      <FileDropzone
        accept={accept}
        disabled={uploading}
        onFiles={handleUpload}
        onReject={() => setUploadError(null)}
        hint={`Erlaubt: ${accept} · eine Datei je Auswahl`}
        label={uploading ? "Lade hoch..." : label}
      />
      {uploadError && <InlineNotice tone="bad">{uploadError}</InlineNotice>}
    </div>
  );
}
