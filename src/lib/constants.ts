import type { SelectOption } from "@zweiund40gmbh/ui";

export const SALUTATIONS: SelectOption<string>[] = [
  { value: "Herr", label: "Herr" },
  { value: "Frau", label: "Frau" },
  { value: "Herr und Frau", label: "Herr und Frau" },
  { value: "Firma", label: "Firma" },
];

export const SALUTATIONS_SECONDARY: SelectOption<string>[] = [
  { value: "", label: "---" },
  { value: "Herr", label: "Herr" },
  { value: "Frau", label: "Frau" },
];

export const DISTRIBUTION_KEYS = ["MEA", "laut Bescheid", "siehe Anlage"];

export const DISTRIBUTION_KEY_OPTIONS: SelectOption<string>[] = DISTRIBUTION_KEYS.map(
  (key) => ({ value: key, label: key })
);

export const PASSWORD_MIN_LENGTH = 8;
