"use client";

import { Button, Field, InlineNotice, PasswordInput, TextInput } from "@zweiund40gmbh/ui";
import { AuthBrand } from "./auth-brand";
import { useLogin } from "./use-login";

/** Sign-in panel: e-mail and password (plus Apple when configured). Validation runs on submit. */
export function LoginForm() {
  const { fieldErrors, error, loading, hasApple, handleSubmit, signInWithApple } = useLogin();

  return (
    <main className="flex min-h-screen items-center justify-center bg-bg px-4">
      <div className="panel grid w-full max-w-sm gap-6">
        <AuthBrand />
        <form noValidate onSubmit={handleSubmit} className="grid gap-4">
          <Field label="E-Mail" error={fieldErrors.email}>
            <TextInput name="email" type="email" autoComplete="email" />
          </Field>
          <Field label="Passwort" error={fieldErrors.password}>
            <PasswordInput name="password" autoComplete="current-password" />
          </Field>
          {error && <InlineNotice tone="bad">{error}</InlineNotice>}
          <Button type="submit" variant="primary" disabled={loading}>
            {loading ? "Anmelden..." : "Anmelden"}
          </Button>
        </form>
        {hasApple && (
          <div className="grid gap-3">
            <p className="label-caps text-center text-ink3">oder</p>
            <Button type="button" onClick={signInWithApple} disabled={loading}>
              Mit Apple anmelden
            </Button>
          </div>
        )}
      </div>
    </main>
  );
}
