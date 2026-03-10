# VermieterMe

Self-hosted Webanwendung zur Verwaltung von Mietobjekten und Erstellung von jährlichen Betriebskostenabrechnungen als PDF.

## Features

- **Objektverwaltung** — Immobilien mit beliebig vielen Wohneinheiten und Miteigentumsanteilen (MEA) anlegen
- **Mieterverwaltung** — Mieter mit Ein-/Auszugsdatum erfassen, auch Zweitmietende möglich
- **Betriebskostenabrechnung** — Abrechnungszeiträume anlegen, Kosten nach Kategorien erfassen, Vorauszahlungen verwalten
- **PDF-Export** — Fertige Betriebskostenabrechnungen als PDF generieren (via @react-pdf/renderer)
- **Kostenkategorien** — Frei konfigurierbare Kategorien mit Verteilerschlüsseln (MEA, nach Anlage, laut Bescheid)
- **Vermieterprofil** — Kontaktdaten und Bankverbindung für die PDF-Abrechnung
- **Authentifizierung** — Login mit E-Mail/Passwort, optionale Apple-ID-Anmeldung
- **Statusübersicht** — Abrechnungen nach Objekt gruppiert mit Dreistufigem Status (Offen → In Bearbeitung → Abgeschlossen)
- **VPI-Indexmiete** — VPI-Werte automatisch von der Bundesbank laden, Mietanpassungshinweise auf dem Dashboard mit Ein-Klick-Anpassung
- **Mietverträge** — Vertragsarten (Standard, Indexmiete, Staffelmiete) mit VPI-Tracking für Indexmieten
- **Dokumenten-Upload** — Mietverträge (PDF) pro Mieter und Belege pro Abrechnung hochladen (PDF, JPEG, PNG, WebP, max. 10 MB)
- **Miete & NK anpassen** — Kaltmiete und NK-Vorauszahlung direkt bei den Mietern anpassen mit Gültigkeitsdatum und Verlauf
- **IBAN-Validierung** — ISO 13616 Prüfung für Vermieter- und Mieter-Bankverbindung
- **Vorjahresübernahme** — Automatische Vorschläge zur Übernahme von Abrechnungsdaten aus dem Vorjahr

## Tech Stack

- [Next.js 16](https://nextjs.org/) (App Router) mit React 19
- [TypeScript](https://www.typescriptlang.org/)
- [Tailwind CSS v4](https://tailwindcss.com/)
- [Prisma](https://www.prisma.io/) ORM mit SQLite
- [NextAuth v5](https://authjs.dev/) (Auth.js)
- [@react-pdf/renderer](https://react-pdf.org/) für PDF-Generierung
- [Vitest](https://vitest.dev/) für Tests

## Quickstart mit Docker

```bash
docker run -d \
  -p 3000:3000 \
  -v vermieterme-data:/app/data \
  -e AUTH_SECRET="$(openssl rand -base64 32)" \
  -e AUTH_TRUST_HOST=true \
  -e ADMIN_EMAIL="admin@vermieterme.local" \
  -e ADMIN_PASSWORD="changeme123" \
  mcbernie/vermieterme:latest
```

Danach unter [http://localhost:3000](http://localhost:3000) erreichbar. Datenbank und Seed werden automatisch beim ersten Start erstellt.

### Docker Compose

```yaml
services:
  vermieterme:
    image: mcbernie/vermieterme:latest
    ports:
      - "3000:3000"
    volumes:
      - vermieterme-data:/app/data
    environment:
      - AUTH_SECRET=dein-geheimer-schluessel
      - AUTH_TRUST_HOST=true
      - ADMIN_EMAIL=admin@vermieterme.local
      - ADMIN_PASSWORD=changeme123

volumes:
  vermieterme-data:
```

## Voraussetzungen (ohne Docker)

- Node.js 22+
- npm

## Installation

```bash
# Repository klonen
git clone <repo-url>
cd vermieterme

# Abhängigkeiten installieren
npm install

# Umgebungsvariablen konfigurieren
cp .env.example .env
```

### Umgebungsvariablen (.env)

| Variable | Beschreibung | Standard |
|---|---|---|
| `DATABASE_URL` | SQLite-Datenbankpfad | `file:./dev.db` |
| `AUTH_SECRET` | Geheimer Schlüssel für NextAuth (generieren mit `npx auth secret`) | — |
| `AUTH_TRUST_HOST` | Auf `true` setzen für Self-Hosting | `true` |
| `ADMIN_EMAIL` | E-Mail für den Admin-Account | `admin@vermieterme.local` |
| `ADMIN_PASSWORD` | Passwort für den Admin-Account | `changeme123` |
| `AUTH_APPLE_ID` | *(optional)* Apple OAuth Client ID | — |
| `AUTH_APPLE_SECRET` | *(optional)* Apple OAuth Client Secret | — |

### Datenbank einrichten

```bash
# Datenbank-Schema anwenden
npm run db:push

# Beispieldaten laden (optional)
npm run db:seed
```

Die Seed-Daten enthalten ein Beispielobjekt (Hamburger Str. 277, Bremen) mit einer Wohneinheit, einem Mieter, neun Kostenkategorien und zwei Abrechnungszeiträumen (2023 + 2024).

### Starten

```bash
# Entwicklungsserver
npm run dev

# Produktions-Build
npm run build
npm start
```

Die Anwendung ist unter [http://localhost:3000](http://localhost:3000) erreichbar. Beim ersten Start mit den in `.env` konfigurierten Admin-Zugangsdaten anmelden.

> **Wichtig:** Das Admin-Passwort nach dem ersten Login im Profil-Bereich ändern.

## Verfügbare Scripts

| Script | Beschreibung |
|---|---|
| `npm run dev` | Entwicklungsserver starten |
| `npm run build` | Produktions-Build erstellen |
| `npm start` | Produktionsserver starten |
| `npm test` | Tests ausführen |
| `npm run test:watch` | Tests im Watch-Modus |
| `npm run lint` | ESLint ausführen |
| `npm run db:push` | Prisma-Schema auf Datenbank anwenden |
| `npm run db:seed` | Beispieldaten in Datenbank laden |
| `npm run db:studio` | Prisma Studio öffnen (Datenbank-GUI) |

## Projektstruktur

```
vermieterme/
├── prisma/
│   ├── schema.prisma          # Datenbank-Schema
│   ├── seed.ts                # Beispieldaten
│   └── dev.db                 # SQLite-Datenbank (generiert)
├── data/
│   ├── dev.db                 # SQLite-Datenbank (generiert)
│   └── uploads/               # Hochgeladene Dokumente
├── public/
│   ├── favicon.svg            # App-Icon
│   └── vermieterme-icon.svg   # Logo (512x512)
├── src/
│   ├── __tests__/             # Vitest-Tests
│   │   ├── api/               # API-Route-Tests
│   │   └── lib/               # Utility-Tests
│   ├── app/
│   │   ├── api/               # API-Routen (REST)
│   │   │   ├── billing-periods/
│   │   │   ├── documents/
│   │   │   ├── rent-changes/
│   │   │   ├── properties/
│   │   │   ├── tenants/
│   │   │   └── vpi/
│   │   ├── billing/           # Abrechnungen
│   │   ├── login/             # Login-Seite
│   │   ├── profile/           # Profilverwaltung
│   │   ├── properties/        # Objektverwaltung
│   │   ├── settings/          # Einstellungen, VPI, PDF-Vorlage
│   │   ├── tenants/           # Mieterverwaltung
│   │   └── page.tsx           # Dashboard
│   ├── components/
│   │   ├── document-upload.tsx # Wiederverwendbare Upload-Komponente
│   │   ├── nav.tsx            # Navigation
│   │   └── ui/               # UI-Komponenten
│   └── lib/
│       ├── auth.ts            # NextAuth-Konfiguration
│       ├── billing.ts         # Abrechnungs-Berechnungen
│       ├── format.ts          # Formatierungs-Hilfsfunktionen
│       ├── iban.ts            # IBAN-Validierung (ISO 13616)
│       ├── prisma.ts          # Prisma-Client
│       └── vpi.ts             # VPI-Parsing (Bundesbank API)
├── vitest.config.mts          # Test-Konfiguration
├── Dockerfile                 # Multi-Stage Docker Build
└── .github/workflows/
    ├── ci.yml                 # Lint, Test, Build
    └── docker.yml             # Docker Hub Publish
```

## Datenmodell

- **Property** — Immobilie mit Adresse und Gesamtanteilen
- **Unit** — Wohneinheit mit Etage und MEA-Anteil
- **Tenant** — Mieter (mit optionalem Zweitmieter) und Ein-/Auszugsdatum
- **BillingPeriod** — Abrechnungszeitraum pro Objekt
- **Cost** — Kosten pro Kategorie und Abrechnungszeitraum
- **CostCategory** — Kostenkategorie mit Verteilerschlüssel
- **Prepayment** — Monatliche Vorauszahlungen pro Einheit und Zeitraum
- **RentChange** — Miet- und Vorauszahlungsänderungen mit Gültigkeitsdatum
- **VpiEntry** — Verbraucherpreisindex-Werte nach Basisjahr (2010, 2015, 2020)
- **Document** — Hochgeladene Dokumente (Mietverträge, Belege) mit Zuordnung zu Mietern oder Abrechnungen
- **LandlordInfo** — Vermieter-Kontaktdaten und Bankverbindung
- **PdfTemplate** — Anpassbare Vorlage für PDF-Abrechnungen

## Backup & Dokumente

Unter **Einstellungen → Backup** kann die gesamte Datenbank als JSON exportiert und importiert werden. Das Backup enthält alle Datenbank-Einträge inklusive der Dokument-Metadaten.

**Wichtig:** Die hochgeladenen Dateien (Mietverträge, Belege) liegen im Verzeichnis `data/uploads/` und sind **nicht** im JSON-Backup enthalten. Für ein vollständiges Backup muss dieser Ordner zusätzlich gesichert werden:

```bash
# Vollständiges Backup
cp data/uploads/ /pfad/zum/backup/uploads/ -r
# JSON-Backup über die Web-Oberfläche exportieren
```

Bei Docker wird `data/` über ein Volume gemountet — ein Volume-Backup sichert sowohl die Datenbank als auch die Uploads.

## Tests

```bash
npm test
```

65 Tests in 9 Testdateien:
- `format.test.ts` — Währungs- und Datumsformatierung (9 Tests)
- `utils.test.ts` — CSS-Klassen-Utility (5 Tests)
- `iban.test.ts` — IBAN-Validierung und Formatierung (14 Tests)
- `vpi.test.ts` — VPI-Parsing der Bundesbank-API (8 Tests)
- `properties.test.ts` — Objekt-API (3 Tests)
- `tenants.test.ts` — Mieter-API (5 Tests)
- `billing-periods.test.ts` — Abrechnungs-API inkl. Validierungen (7 Tests)
- `users.test.ts` — Benutzer-API (7 Tests)
- `profile.test.ts` — Profil-API mit Authentifizierung (7 Tests)

## CI/CD

GitHub Actions Workflows:

- **CI** (`ci.yml`) — Lint, Test und Build bei jedem Push/PR auf `main`
- **Docker** (`docker.yml`) — Baut Multi-Arch Docker Image (amd64 + arm64) und pusht zu Docker Hub bei Push auf `main` oder bei Tags (`v*`)

### Docker Hub Setup

Im GitHub Repository unter *Settings → Secrets and variables → Actions* folgende Secrets anlegen:

| Secret | Beschreibung |
|---|---|
| `DOCKERHUB_USERNAME` | Docker Hub Benutzername |
| `DOCKERHUB_TOKEN` | Docker Hub Access Token |

## Autor

Entwickelt von **Nicolas Wilms** in Bremen.

## Lizenz

MIT — siehe [LICENSE](LICENSE).
