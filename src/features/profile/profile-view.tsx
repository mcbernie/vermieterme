"use client";

import { PageHeader } from "@zweiund40gmbh/ui";
import { useSession } from "next-auth/react";
import { requestJson } from "@/features/settings/request";
import { LoadGate, SettingsSection } from "@/features/settings/section";
import { LOAD_FAILED, useLoad } from "@/features/settings/use-load";
import type { AppUser } from "@/types";
import { NameForm } from "./name-form";
import { PasswordForm } from "./password-form";

/** Loads the signed-in user's profile. */
const loadProfile = (signal: AbortSignal) => requestJson<AppUser>("/api/profile", LOAD_FAILED, { signal });

/** Own profile: display name and password change. */
export function ProfileView() {
  const { update: updateSession } = useSession();
  const { data: profile, loading, error, refreshError, retry, refresh } = useLoad(loadProfile);

  /** After a name change: update the session (header shows the name) and reload the profile. */
  async function nameSaved(name: string) {
    try {
      await updateSession({ name });
    } catch {
      // The name is stored; the header catches up on the next sign-in.
    }
    await refresh();
  }

  return (
    <>
      <PageHeader title="Mein Profil" description={profile ? profile.email : undefined} />
      <LoadGate loading={loading} error={error} refreshError={refreshError} onRetry={retry}>
        {profile && (
          <div className="grid max-w-3xl gap-8">
            <SettingsSection title="Persönliche Daten">
              <NameForm profile={profile} onSaved={nameSaved} />
            </SettingsSection>
            <SettingsSection title="Passwort ändern">
              <PasswordForm />
            </SettingsSection>
          </div>
        )}
      </LoadGate>
    </>
  );
}
