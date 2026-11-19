import { EmptyState } from "@zweiund40gmbh/ui";
import Link from "next/link";

/** 404 page: states that the page does not exist and links back to the dashboard. */
export default function NotFound() {
  return (
    <main className="flex min-h-[60vh] items-center justify-center px-4">
      <EmptyState
        icon={null}
        title="Seite nicht gefunden"
        description="Die gesuchte Seite existiert nicht oder wurde verschoben."
        action={
          <Link href="/" className="text-sm text-accentstrong hover:underline">
            Zurück zum Dashboard
          </Link>
        }
      />
    </main>
  );
}
