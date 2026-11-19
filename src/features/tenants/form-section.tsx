import type { ReactNode } from "react";
import type { DateInputProblem } from "@zweiund40gmbh/ui";
import type { TenantDateField, TenantFormErrors, TenantFormValues } from "./tenant-form-values";

/** Props shared by the field groups of the tenant form. */
export interface TenantFieldsProps {
  form: TenantFormValues;
  /** Errors shown after a submit attempt. */
  errors: TenantFormErrors;
  /** Sets one field. */
  set: <K extends keyof TenantFormValues>(key: K, value: TenantFormValues[K]) => void;
  /** Reports whether a date input currently rejects its text. */
  onDateProblem: (field: TenantDateField, problem: DateInputProblem | null) => void;
}

/** Titled group of form fields, separated from the previous group by a line. */
export function FormSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-4 border-t border-line pt-4 first:border-t-0 first:pt-0">
      <h3 className="label-caps">{title}</h3>
      {children}
    </section>
  );
}
