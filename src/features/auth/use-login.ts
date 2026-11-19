"use client";

import { getProviders, signIn, useSession } from "next-auth/react";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { safeCallbackUrl } from "./safe-callback-url";
import { validateLogin, type LoginErrors } from "./login-validation";

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
    const result = await signIn("credentials", { email, password, redirect: false });
    if (result?.error) {
      setError("Ungültige Anmeldedaten");
      setLoading(false);
      busy.current = false;
    } else {
      router.push(callbackUrl);
      router.refresh();
    }
  }

  /** Starts the Apple sign-in. */
  function signInWithApple() {
    void signIn("apple", { callbackUrl });
  }

  return { fieldErrors, error, loading, hasApple, handleSubmit, signInWithApple };
}
