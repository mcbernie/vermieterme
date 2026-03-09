export default function ImpressumPage() {
  return (
    <div className="mx-auto max-w-2xl px-4 py-12 sm:px-6 lg:px-8">
      <h1 className="mb-8 text-2xl font-bold text-zinc-900">Impressum</h1>

      <div className="space-y-8 text-sm text-zinc-700">
        <section>
          <h2 className="mb-2 text-lg font-semibold text-zinc-900">
            Angaben gemäß § 5 TMG
          </h2>
          <p>
            VermieterMe ist eine Open-Source-Software zur Verwaltung von
            Mietobjekten und Erstellung von Betriebskostenabrechnungen.
          </p>
          <p className="mt-2">
            Die Verantwortung für den Betrieb dieser Instanz liegt beim
            jeweiligen Betreiber. Bitte konfigurieren Sie Ihre eigenen
            Impressums-Angaben entsprechend der gesetzlichen Anforderungen.
          </p>
        </section>

        <section>
          <h2 className="mb-2 text-lg font-semibold text-zinc-900">
            Entwicklung
          </h2>
          <p>
            Konzept, Design und Entwicklung:
          </p>
          <p className="mt-1 font-medium text-zinc-900">Nicolas Wilms</p>
          <p className="mt-2">
            Quellcode verfügbar auf{" "}
            <a
              href="https://github.com/nicowilms/vermieterme"
              className="text-red-700 underline hover:text-red-800"
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub
            </a>
            {" "}unter der MIT-Lizenz.
          </p>
        </section>

        <section>
          <h2 className="mb-2 text-lg font-semibold text-zinc-900">
            Haftungsausschluss
          </h2>
          <p>
            Diese Software wird „wie besehen" ohne jegliche Gewährleistung
            bereitgestellt. Die Nutzung erfolgt auf eigene Verantwortung. Der
            Entwickler übernimmt keine Haftung für die Richtigkeit der
            erstellten Abrechnungen.
          </p>
        </section>

        <section>
          <h2 className="mb-2 text-lg font-semibold text-zinc-900">
            Datenschutz
          </h2>
          <p>
            VermieterMe ist eine Self-Hosted-Anwendung. Alle Daten verbleiben
            auf dem Server des Betreibers. Es werden keine Daten an Dritte
            übermittelt. Es werden keine Cookies zu Tracking-Zwecken
            eingesetzt — lediglich ein Session-Cookie für die Authentifizierung.
          </p>
        </section>
      </div>
    </div>
  );
}
