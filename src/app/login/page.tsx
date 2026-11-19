import { Suspense } from "react";
import { LoginForm } from "@/features/auth/login-form";

/** Login page; the form reads the search params, hence the Suspense boundary. */
export default function LoginPage() {
  return (
    <Suspense>
      <LoginForm />
    </Suspense>
  );
}
