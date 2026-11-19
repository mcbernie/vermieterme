"use client";

import { useId, useMemo, useState, type FormEvent } from "react";
import { InlineNotice, Sheet, SheetFooter, type DateInputProblem } from "@zweiund40gmbh/ui";
import type { PropertyWithUnits, Tenant } from "@/types";
import { TenantContactFields } from "./tenant-contact-fields";
import {
  EMPTY_TENANT_FORM,
  tenantToForm,
  validateTenantForm,
  type TenantDateField,
  type TenantDateProblems,
  type TenantFormValues,
} from "./tenant-form-values";
import { unitOptions } from "./tenant-helpers";
import { TenantLeaseFields } from "./tenant-lease-fields";
import { TenantPersonFields } from "./tenant-person-fields";
import type { MutationError } from "./use-tenants";

interface Props {
  /** Tenant to edit; undefined creates a new one. */
  tenant?: Tenant;
  properties: readonly PropertyWithUnits[];
  /** Saves the values; resolves to an error text or null on success. */
  onSave: (values: TenantFormValues) => Promise<MutationError>;
  onClose: () => void;
}

/**
 * Side panel to create or edit a tenant. Validates on submit and shows errors at the fields;
 * a failed save keeps the panel open with the server error. Mount it only while it should be open.
 */
export function TenantFormSheet({ tenant, properties, onSave, onClose }: Props) {
  const formId = useId();
  const [form, setForm] = useState<TenantFormValues>(() => (tenant ? tenantToForm(tenant) : EMPTY_TENANT_FORM));
  const [problems, setProblems] = useState<TenantDateProblems>({});
  const [attempted, setAttempted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const units = useMemo(() => unitOptions(properties), [properties]);

  const errors = attempted ? validateTenantForm(form, problems) : {};
  // Unreadable dates are flagged right away, not only after the first submit.
  const shownErrors = { ...errors, ...dateProblemErrors(form, problems) };

  /** Sets one field of the form. */
  function set<K extends keyof TenantFormValues>(key: K, value: TenantFormValues[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
    // The index date input unmounts with another lease type; its last problem must not linger.
    if (key === "leaseType") onDateProblem("indexReferenceDate", null);
  }

  /** Stores the problem a date input reports for its current text. */
  function onDateProblem(field: TenantDateField, problem: DateInputProblem | null) {
    setProblems((prev) => (prev[field] === problem ? prev : { ...prev, [field]: problem }));
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAttempted(true);
    setSaveError(null);
    if (Object.keys(validateTenantForm(form, problems)).length > 0) return;
    setBusy(true);
    const error = await onSave(form);
    setBusy(false);
    if (error) setSaveError(error);
    else onClose();
  }

  const fieldProps = { form, errors: shownErrors, set, onDateProblem };
  return (
    <Sheet
      title={tenant ? "Mieter bearbeiten" : "Neuen Mieter anlegen"}
      wide
      onClose={onClose}
      footer={
        <SheetFooter onClose={onClose} formId={formId} busy={busy} saveLabel={tenant ? "Aktualisieren" : "Speichern"} />
      }
    >
      <form id={formId} noValidate onSubmit={submit} className="grid gap-6 p-6">
        {saveError && <InlineNotice tone="bad">{saveError}</InlineNotice>}
        <TenantPersonFields {...fieldProps} units={units} />
        <TenantLeaseFields {...fieldProps} />
        <TenantContactFields {...fieldProps} />
      </form>
    </Sheet>
  );
}

/** Errors of date fields whose text is currently unreadable (shown before any submit). */
function dateProblemErrors(form: TenantFormValues, problems: TenantDateProblems) {
  const all = validateTenantForm(form, problems);
  const out: Partial<Record<TenantDateField, string>> = {};
  for (const field of Object.keys(problems) as TenantDateField[]) {
    if (problems[field] && all[field]) out[field] = all[field];
  }
  return out;
}
