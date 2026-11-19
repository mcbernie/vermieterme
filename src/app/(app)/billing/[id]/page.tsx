"use client";

import { useParams } from "next/navigation";
import { BillingDetailView } from "@/features/billing/billing-detail-view";

/** Billing period detail page; the view is remounted per id so no state leaks between periods. */
export default function BillingDetailPage() {
  const { id } = useParams<{ id: string }>();
  return <BillingDetailView key={id} id={id} />;
}
