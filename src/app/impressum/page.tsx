import type { ReactNode } from "react";

/** One titled block of the imprint. */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-2">
      <h2 className="section-title">{title}</h2>
      {children}
    </section>
  );
}

/** Public imprint page (outside the app frame, so it brings its own narrow column). */
export default function ImpressumPage() {
  return (
    <main className="min-h-screen bg-bg">
      <div className="mx-auto grid max-w-2xl gap-8 px-4 py-12 text-sm text-ink2">
        <h1 className="page-title">Impressum</h1>

        <Section title="Angaben gemäß § 5 TMG">
          <p>
            VermieterMe ist eine Open-Source-Software zur Verwaltung von Mietobjekten und Erstellung von
            Betriebskostenabrechnungen.
          </p>
          <p>
            Die Verantwortung für den Betrieb dieser Instanz liegt beim jeweiligen Betreiber. Bitte
            konfigurieren Sie Ihre eigenen Impressums-Angaben entsprechend der gesetzlichen Anforderungen.
          </p>
        </Section>

        <Section title="Entwicklung">
          <p>Konzept, Design und Entwicklung:</p>
          <p className="font-medium text-ink">Nicolas Wilms</p>
          <p>
            Quellcode verfügbar auf{" "}
            <a
              href="https://github.com/nicowilms/vermieterme"
              className="text-accentstrong hover:underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub
            </a>{" "}
            unter der MIT-Lizenz.
          </p>
        </Section>

        <Section title="Haftungsausschluss">
          <p>
            Diese Software wird {"„wie besehen“"} ohne jegliche Gewährleistung bereitgestellt. Die
            Nutzung erfolgt auf eigene Verantwortung. Der Entwickler übernimmt keine Haftung für die
            Richtigkeit der erstellten Abrechnungen.
          </p>
        </Section>

        <Section title="Datenschutz">
          <p>
            VermieterMe ist eine Self-Hosted-Anwendung. Alle Daten verbleiben auf dem Server des Betreibers.
            Es werden keine Daten an Dritte übermittelt. Es werden keine Cookies zu Tracking-Zwecken
            eingesetzt — lediglich ein Session-Cookie für die Authentifizierung.
          </p>
        </Section>
      </div>
    </main>
  );
}
