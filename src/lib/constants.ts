import type { ComboboxOption } from "@/components/ui/combobox";

export const SALUTATIONS: ComboboxOption[] = [
  { value: "Herr", label: "Herr" },
  { value: "Frau", label: "Frau" },
  { value: "Herr und Frau", label: "Herr und Frau" },
  { value: "Firma", label: "Firma" },
];

export const SALUTATIONS_SECONDARY: ComboboxOption[] = [
  { value: "", label: "---" },
  { value: "Herr", label: "Herr" },
  { value: "Frau", label: "Frau" },
];

export const DISTRIBUTION_KEYS = ["MEA", "laut Bescheid", "siehe Anlage"];

export const DISTRIBUTION_KEY_OPTIONS: ComboboxOption[] = DISTRIBUTION_KEYS.map(
  (key) => ({ value: key, label: key })
);

export const PASSWORD_MIN_LENGTH = 8;
