# VermieterMe

Self-hosted Webanwendung zur Verwaltung von Mietobjekten und Erstellung von jährlichen **Betriebskostenabrechnungen** als PDF.

Entwickelt für private Vermieter in Deutschland, die ihre Nebenkostenabrechnungen einfach und lokal verwalten wollen — ohne Cloud, ohne Abo.

## Features

- Objekte mit beliebig vielen Wohneinheiten und Miteigentumsanteilen (MEA)
- Mieterverwaltung mit Ein-/Auszugsdatum und optionalem Zweitmieter
- Betriebskostenabrechnung mit konfigurierbaren Kostenkategorien und Verteilerschlüsseln
- Automatische Berechnung von Nachzahlungen und Erstattungen
- PDF-Export der fertigen Abrechnung
- Login mit E-Mail/Passwort, optional Apple-ID
- SQLite-Datenbank — keine externe DB nötig

## Quickstart

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

Danach erreichbar unter **http://localhost:3000**

## Docker Compose

```yaml
services:
  vermieterme:
    image: mcbernie/vermieterme:latest
    ports:
      - "3000:3000"
    volumes:
      - vermieterme-data:/app/data
    environment:
      - AUTH_SECRET=ein-langer-geheimer-schluessel
      - AUTH_TRUST_HOST=true
      - ADMIN_EMAIL=admin@vermieterme.local
      - ADMIN_PASSWORD=changeme123

volumes:
  vermieterme-data:
```

## Umgebungsvariablen

| Variable | Beschreibung | Pflicht |
|---|---|---|
| `AUTH_SECRET` | Geheimer Schlüssel für Session-Verschlüsselung | Ja |
| `AUTH_TRUST_HOST` | Auf `true` setzen (nötig für Self-Hosting) | Ja |
| `ADMIN_EMAIL` | E-Mail für den Admin-Account | Ja |
| `ADMIN_PASSWORD` | Passwort für den Admin-Account | Ja |
| `DATABASE_URL` | SQLite-Pfad (Standard: `file:/app/data/vermieterme.db`) | Nein |
| `AUTH_APPLE_ID` | Apple OAuth Client ID | Nein |
| `AUTH_APPLE_SECRET` | Apple OAuth Client Secret | Nein |

> **Wichtig:** Das Standard-Passwort nach dem ersten Login im Profil-Bereich ändern.

## Daten & Backup

Die SQLite-Datenbank liegt im Volume unter `/app/data/vermieterme.db`. Für ein Backup genügt eine Kopie dieser Datei:

```bash
docker cp <container>:/app/data/vermieterme.db ./backup.db
```

## Architektur

- **Next.js 16** (App Router) mit React 19 und TypeScript
- **Tailwind CSS v4** für das UI
- **Prisma ORM** mit SQLite
- **NextAuth v5** für Authentifizierung
- **@react-pdf/renderer** für PDF-Generierung

Multi-Arch Image: `linux/amd64` und `linux/arm64` (Raspberry Pi, Apple Silicon, etc.)

## Links

- [GitHub Repository](https://github.com/mcbernie/vermieterme)
- [Lizenz: MIT](https://github.com/mcbernie/vermieterme/blob/main/LICENSE)

---

Entwickelt von **Nicolas Wilms** in Bremen.
