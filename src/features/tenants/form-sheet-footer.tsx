import { Button, Spinner } from "@zweiund40gmbh/ui";

interface Props {
  /** Id of the form the save button submits. */
  formId: string;
  saveLabel: string;
  onCancel: () => void;
  /** Save running: both buttons are disabled, the save button shows a spinner. */
  busy: boolean;
}

/**
 * Footer of a form panel, shaped like the library's `SheetFooter` (buttons fill the width).
 * Unlike `SheetFooter` (0.10.0), it also disables „Abbrechen“ while saving, as `RentAdjustModal` does.
 */
export function FormSheetFooter({ formId, saveLabel, onCancel, busy }: Props) {
  return (
    <>
      <Button type="button" variant="secondary" className="h-12 flex-1 justify-start" onClick={onCancel} disabled={busy}>
        Abbrechen
      </Button>
      <Button
        type="submit"
        form={formId}
        variant="primary"
        className="h-12 flex-1 justify-start"
        disabled={busy}
        aria-busy={busy || undefined}
      >
        {busy && <Spinner size={14} label="Wird gespeichert …" />}
        {saveLabel}
      </Button>
    </>
  );
}
