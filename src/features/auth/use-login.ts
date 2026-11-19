"use client";

import { getProviders, signIn, useSession } from "next-auth/react";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { safeCallbackUrl } from "./safe-callback-url";
import { validateLogin, type LoginErrors } from "./login-validation";

const UNAVAILABLE = "Anmeldung derzeit nicht möglich. Bitte versuchen Sie es erneut.";

/** State and handlers of the login form: credentials sign-in, redirect and the optional Apple provider. */
export function useLogin() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { status } = useSession();
  const callbackUrl = safeCallbackUrl(searchParams.get("callbackUrl"));
  const [fieldErrors, setFieldErrors] = useState<LoginErrors>({});
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [hasApple, setHasApple] = useState(false);
  // State updates are asynchronous, so a ref blocks a second submit in the same tick.
  const busy = useRef(false);

  /** Re-enables the form after a failed attempt. */
  function release() {
    busy.current = false;
    setLoading(false);
  }

  useEffect(() => {
    if (status === "authenticated") router.push(callbackUrl);
  }, [status, callbackUrl, router]);

  useEffect(() => {
    getProviders().then((providers) => {
      if (providers?.apple) setHasApple(true);
    });
  }, []);

  /** Validates, then signs in with the credentials provider; a failure shows a notice, success redirects. */
  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (busy.current) return;
    const formData = new FormData(e.currentTarget);
    const email = String(formData.get("email") ?? "");
    const password = String(formData.get("password") ?? "");
    const errors = validateLogin(email, password);
    setFieldErrors(errors);
    setError("");
    if (errors.email || errors.password) return;

    busy.current = true;
    setLoading(true);
    let signedIn = false;
    try {
      const result = await signIn("credentials", { email, password, redirect: false });
      if (result?.error) setError("Ungültige Anmeldedaten");
      else signedIn = true;
    } catch {
      setError(UNAVAILABLE);
    } finally {
      // After a success the lock stays until the redirect has happened.
      if (!signedIn) release();
    }
    if (signedIn) {
      router.push(callbackUrl);
      router.refresh();
    }
  }

  /** Starts the Apple sign-in (locked against double clicks, like the credentials form). */
  async function signInWithApple() {
    if (busy.current) return;
    busy.current = true;
    setLoading(true);
    setError("");
    try {
      await signIn("apple", { callbackUrl });
    } catch {
      setError(UNAVAILABLE);
      release();
    }
  }

  return { fieldErrors, error, loading, hasApple, handleSubmit, signInWithApple };
}
