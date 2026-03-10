"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type { Document } from "@/types";

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

interface DocumentUploadProps {
  billingPeriodId?: string;
  tenantId?: string;
  category?: string;
  label?: string;
  accept?: string;
}

export function DocumentUpload({
  billingPeriodId,
  tenantId,
  category = "other",
  label = "Dokument hochladen",
  accept = ".pdf,.jpg,.jpeg,.png,.webp",
}: DocumentUploadProps) {
  const [documents, setDocuments] = useState<Document[]>([]);
  const [uploading, setUploading] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const fetchDocuments = useCallback(async () => {
    const params = new URLSearchParams();
    if (billingPeriodId) params.set("billingPeriodId", billingPeriodId);
    if (tenantId) params.set("tenantId", tenantId);

    try {
      const res = await fetch(`/api/documents?${params}`);
      if (res.ok) {
        const docs = await res.json();
        setDocuments(
          category
            ? docs.filter((d: Document) => d.category === category)
            : docs
        );
      }
    } catch (error) {
      console.error("Failed to fetch documents:", error);
    }
  }, [billingPeriodId, tenantId, category]);

  useEffect(() => {
    fetchDocuments();
  }, [fetchDocuments]);

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;

    setUploading(true);
    try {
      const formData = new FormData();
      formData.append("file", file);
      formData.append("category", category);
      if (billingPeriodId) formData.append("billingPeriodId", billingPeriodId);
      if (tenantId) formData.append("tenantId", tenantId);

      const res = await fetch("/api/documents", {
        method: "POST",
        body: formData,
      });

      if (res.ok) {
        await fetchDocuments();
      }
    } catch (error) {
      console.error("Upload failed:", error);
    } finally {
      setUploading(false);
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    }
  }

  async function handleDelete(id: string) {
    setDeleteTarget(null);
    try {
      const res = await fetch(`/api/documents/${id}`, { method: "DELETE" });
      if (res.ok) {
        await fetchDocuments();
      }
    } catch (error) {
      console.error("Delete failed:", error);
    }
  }

  return (
    <div>
      {documents.length > 0 && (
        <div className="mb-3 space-y-2">
          {documents.map((doc) => (
            <div
              key={doc.id}
              className="flex items-center justify-between rounded-lg border border-zinc-200 bg-zinc-50 px-3 py-2"
            >
              <a
                href={`/api/documents/${doc.id}/file`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-sm text-zinc-700 hover:text-zinc-900"
              >
                <span className="text-zinc-400">
                  {doc.mimeType === "application/pdf" ? (
                    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
                    </svg>
                  ) : (
                    <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" d="m2.25 15.75 5.159-5.159a2.25 2.25 0 0 1 3.182 0l5.159 5.159m-1.5-1.5 1.409-1.409a2.25 2.25 0 0 1 3.182 0l2.909 2.909M3.75 21h16.5A2.25 2.25 0 0 0 22.5 18.75V5.25A2.25 2.25 0 0 0 20.25 3H3.75A2.25 2.25 0 0 0 1.5 5.25v13.5A2.25 2.25 0 0 0 3.75 21Z" />
                    </svg>
                  )}
                </span>
                <span className="max-w-[200px] truncate">{doc.originalName}</span>
                <span className="text-xs text-zinc-400">
                  ({formatFileSize(doc.size)})
                </span>
              </a>
              <button
                onClick={() => setDeleteTarget(doc.id)}
                className="rounded border border-red-200 px-2 py-0.5 text-xs font-medium text-red-600 hover:bg-red-50"
              >
                Entfernen
              </button>
            </div>
          ))}
        </div>
      )}

      <label className="inline-flex cursor-pointer items-center gap-2 rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50">
        <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" />
        </svg>
        {uploading ? "Lade hoch..." : label}
        <input
          ref={fileInputRef}
          type="file"
          accept={accept}
          onChange={handleUpload}
          disabled={uploading}
          className="hidden"
        />
      </label>

      <ConfirmDialog
        open={deleteTarget !== null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        onConfirm={() => {
          if (deleteTarget) handleDelete(deleteTarget);
        }}
        title="Dokument entfernen"
        description="Möchten Sie dieses Dokument wirklich löschen?"
      />
    </div>
  );
}
