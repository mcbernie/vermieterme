/** Field errors of the login form (a key is only present when the field is invalid). */
export interface LoginErrors {
  email?: string;
  password?: string;
}

const EMAIL_SHAPE = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;

/**
 * Checks the submitted login values. Returns an error text per invalid field,
 * an empty object when everything is fine.
 */
export function validateLogin(email: string, password: string): LoginErrors {
  const errors: LoginErrors = {};
  const trimmed = email.trim();
  if (!trimmed) errors.email = "Bitte geben Sie Ihre E-Mail-Adresse ein.";
  else if (!EMAIL_SHAPE.test(trimmed)) errors.email = "Das ist keine gültige E-Mail-Adresse.";
  if (!password) errors.password = "Bitte geben Sie Ihr Passwort ein.";
  return errors;
}
