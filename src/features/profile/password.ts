import { PASSWORD_MIN_LENGTH } from "@/lib/constants";

/** Values of the change-password form. */
export interface PasswordValues {
  currentPassword: string;
  newPassword: string;
  confirmPassword: string;
}

/** Field errors of the change-password form. */
export type PasswordErrors = Partial<Record<keyof PasswordValues, string>>;

/**
 * Checks the change-password form with the same rules as before: current password given, new password long
 * enough and repeated identically. The server checks the current password itself.
 * @returns Errors by field; empty when the form can be sent.
 */
export function passwordErrors(values: PasswordValues): PasswordErrors {
  const errors: PasswordErrors = {};
  if (!values.currentPassword) errors.currentPassword = "Bitte das aktuelle Passwort angeben.";
  if (values.newPassword.length < PASSWORD_MIN_LENGTH) {
    errors.newPassword = `Passwort muss mindestens ${PASSWORD_MIN_LENGTH} Zeichen lang sein`;
  }
  if (values.newPassword !== values.confirmPassword) errors.confirmPassword = "Passwörter stimmen nicht überein";
  return errors;
}
