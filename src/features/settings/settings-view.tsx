"use client";

import { useState } from "react";
import { BackupSection } from "./backup-section";
import { CostCategoriesSection } from "./cost-categories-section";
import { loadLandlord } from "./landlord";
import { LandlordForm } from "./landlord-form";
import { LoadGate, SettingsSection } from "./section";
import { useLoad } from "./use-load";
import { UsersSection } from "./users-section";

/** Landlord and bank details: loads them and shows the form only when loading succeeded. */
function LandlordSection() {
  const { data, loading, error, retry } = useLoad(loadLandlord);
  return (
    <SettingsSection title="Vermieter-Informationen">
      <LoadGate loading={loading} error={error} onRetry={retry}>
        {data && <LandlordForm initial={data} />}
      </LoadGate>
    </SettingsSection>
  );
}

/**
 * General settings: landlord, cost categories, users and backup. After an import the data sections are
 * mounted again, which reloads them.
 */
export function SettingsView() {
  const [generation, setGeneration] = useState(0);
  return (
    <>
      <LandlordSection key={`landlord-${generation}`} />
      <CostCategoriesSection key={`categories-${generation}`} />
      <UsersSection key={`users-${generation}`} />
      <BackupSection onImported={() => setGeneration((g) => g + 1)} />
    </>
  );
}
