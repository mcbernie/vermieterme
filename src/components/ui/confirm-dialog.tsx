"use client";

import { AlertTriangleIcon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: "danger" | "default";
}

export function ConfirmDialog({
  open,
  onOpenChange,
  onConfirm,
  title,
  description,
  confirmLabel = "Löschen",
  cancelLabel = "Abbrechen",
  variant = "danger",
}: ConfirmDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <div className="flex flex-col items-center gap-3 pt-2">
            {variant === "danger" && (
              <div className="flex h-12 w-12 items-center justify-center rounded-full bg-red-100">
                <AlertTriangleIcon className="h-6 w-6 text-red-600" />
              </div>
            )}
            <DialogTitle className="text-center text-base font-semibold">
              {title}
            </DialogTitle>
            {description && (
              <DialogDescription className="text-center">
                {description}
              </DialogDescription>
            )}
          </div>
        </DialogHeader>
        <DialogFooter>
          <button
            type="button"
            onClick={() => onOpenChange(false)}
            className="rounded-lg border border-zinc-300 bg-white px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={() => {
              onConfirm();
              onOpenChange(false);
            }}
            className={
              variant === "danger"
                ? "rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700"
                : "rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800"
            }
          >
            {confirmLabel}
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
