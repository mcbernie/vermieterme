# Plan – Phase 2: eigenständiges Rust-Backend

Stand: 2026-11-20. Vertrag der Schnittstelle: [`api-contract.md`](api-contract.md).

## Ziel

Die Next.js-API-Routen, Prisma/SQLite und NextAuth werden durch ein eigenständiges Backend in Rust ersetzt
(axum + sqlx, REST/JSON). Ein Docker-Image liefert Backend, statisch exportierte Web-Oberfläche und – bis Phase 3 –
die PDF-Erzeugung. Die Datenbank ist per `DATABASE_URL` zwischen SQLite und MySQL/MariaDB umstellbar. Das Datenmodell
ist von Anfang an mandantenfähig (#44). Die Mieter-App (`apps/mieter-app`) läuft ohne App-Update weiter.

Nicht Teil von Phase 2: PDF in Rust (Phase 3), OIDC-Anmeldung (nur der Schnitt wird vorbereitet), E-Mail-Versand.

## Architektur

```
backend/                      Cargo-Workspace (Cargo.lock, rust-toolchain.toml, clippy.toml, deny.toml)
  crates/domain/              vermieterme-domain: reine Fachlogik ohne IO
                              Cent-Beträge, Datumsarithmetik, Abrechnung (Summen, MEA, Monate),
                              Verteilerschlüssel, VPI-Parser und Anpassungsvorschlag, IBAN-Prüfung
  crates/db/                  vermieterme-db: Pool, Dialekt, Migrationen, Repositories mit Scope-Parameter,
                              Import der Prisma-Datenbank (legacy_import)
  crates/api/                 vermieterme-api: axum-Router, Handler, Auth, Rechte-Mapping, Fehlerform,
                              Konfiguration (from_lookup), PDF-Brücke; Binary `vermieterme`
  migrations/sqlite/          SQL-Migrationen je Dialekt, gleiche Nummern und Inhalte
  migrations/mysql/
pdf-renderer/                 Node-Skript (Übergang bis Phase 3): JSON auf stdin -> PDF auf stdout
src/, apps/mieter-app/        Web-Oberfläche (statischer Export) und Mieter-App wie bisher
```

Binary `vermieterme` mit Unterbefehlen: `serve` (Standard), `import-legacy [--dry-run]`, `healthcheck`.

**Begriffe:** *Kunde* = Mandant = `account` (Tabelle, Code, API). *Mieter* = `tenant` (wie heute, wegen
`/api/tenant-app`). Das Wort „tenant“ wird nie für Mandanten benutzt.

**Scope:** Jede Repository-Funktion mit Kundenbezug nimmt `&AccountScope` (Kunde aus der Sitzung, nie aus der
Anfrage); Mieter-Zugriffe nehmen `&TenantScope` (Kunde, Mieter, Einheit, Objekt – serverseitig aus dem Mieter
ermittelt). Alle fachlichen Tabellen tragen `account_id`; jede Abfrage filtert darauf. Fremde IDs → `404`.
Rechte-Mapping (Rollen `admin`, `member`, Mieter) an einer Stelle: `backend/crates/api/src/auth/permissions.rs`.

## Datenbank-Strategie

- **sqlx mit `Any`-Treiber**, Abfragen als Laufzeit-SQL im gemeinsamen Teilumfang (Platzhalter `?` gilt für SQLite
  und MySQL). Wenige abweichende Stellen (Upsert: `ON CONFLICT` vs. `ON DUPLICATE KEY`) über ein `Dialect`-Enum
  in `vermieterme-db` – nur dort.
- **Migrationen je Dialekt** (`migrations/sqlite`, `migrations/mysql`), gleiche Nummern, gleiche Tabellen und
  Spalten; CI prüft per Test, dass beide Schemata dieselben Tabellen/Spalten haben.
- **Portable Typen:** IDs `VARCHAR(36)`/`TEXT` (Bestands-cuids bleiben, neue IDs UUID v7 als Text); Beträge `BIGINT`
  in Cent; Kalenderdaten `CHAR(10)` `YYYY-MM-DD`; Zeitpunkte `BIGINT` Unix-Millisekunden UTC; Wahrheitswerte
  `INTEGER`/`SMALLINT` 0/1, gelesen als `i64` (`TINYINT`/`bool` sind über `sqlx::Any` nicht lesbar); JSON (PDF-Vorlage) als `TEXT`. Unique-Spalten in MySQL als `VARCHAR` mit Länge.
- **Optimistische Sperre:** `version BIGINT` in `cost` und `prepayment` (#67); Teil-Updates als ein `UPDATE` mit nur
  den gesendeten Feldern (#25).
- Integrationstests: je Test eigene Datenbank. SQLite braucht keinen Dienst und läuft **immer** (temporäre Datei je
  Test, kein Überspringen). MariaDB über `TEST_MYSQL_URL`; Tests heißen `mysql_*` und geben ohne Variable sichtbar
  `[skip]` aus, das Rezept `test-mysql` bricht ohne Variable ab.

Begründung: Ein Codepfad für beide Datenbanken, keine zweite Implementierung je Repository. Preis: keine
`query!`-Prüfung zur Übersetzungszeit – ausgeglichen durch Integrationstests gegen beide Dialekte in CI.
Der Typumfang von `Any` (bool, i64, f64, String, Bytes) ist laut sqlx-0.8-Doku ausreichend – **ungeprüft**, wird in
P2 als Erstes per Spike bestätigt; sonst Rückfall auf Dialekt-Enum für die betroffenen Abfragen.

## Auth

- **Web (Vermieter):** E-Mail + Passwort, argon2id. Bestehende bcrypt-Hashes werden beim nächsten erfolgreichen
  Login auf argon2id umgestellt. Sitzungs-Cookie `vm_session` (`HttpOnly`, `Secure`, `SameSite=Lax`, Pfad `/`),
  256 bit Zufall, in der DB nur SHA-256-Hash; Leerlauf- und Höchstdauer; Rotation beim Login; alle Sitzungen enden
  bei Passwortänderung, Sperre des Benutzers oder des Kunden.
- **CSRF:** Mutierende Anfragen (`POST/PUT/PATCH/DELETE`) unter `/api/` außer `/api/tenant-app/**` brauchen den
  Header `X-CSRF-Guard: 1` und, falls vorhanden, ein `Origin` gleich dem eigenen. Zusammen mit `SameSite=Lax`.
- **Rate-Limits** (im Speicher, eine Instanz): Login je IP und je E-Mail, Einladungscode je IP; Sperre nach
  Fehlversuchen mit gleicher, generischer Antwort und gleicher Antwortzeit.
- **Admin-Bootstrap:** Gibt es keinen Admin und sind `ADMIN_EMAIL`/`ADMIN_PASSWORD` gesetzt, wird beim Start ein
  Admin angelegt. Danach wird das Umgebungs-Passwort nie mehr als Passwort akzeptiert (#69).
- **Kunde wechseln:** Admin setzt in seiner Sitzung `acting_account_id`; jeder Wechsel und jede Änderung im fremden
  Kunden landet im Audit-Log; `GET /api/auth/session` meldet `impersonating`, die Oberfläche zeigt eine Hinweisleiste.
- **Mieter-App:** JWT HS256 mit `AUTH_SECRET`, Claims `{tenantId, unitId}`, `iat`, `exp` 30 Tage – wie heute, damit
  vorhandene App-Anmeldungen gültig bleiben. Neu: bei jeder Anfrage Prüfung in der DB (Mieter existiert, Kunde nicht
  gesperrt, `iat` nach `app_tokens_revoked_at` des Mieters). Einladungscodes: 6 Zeichen (App begrenzt die Eingabe,
  `apps/mieter-app/app/login.tsx:136`), gespeichert als HMAC-SHA256 (Schlüssel per HKDF aus `AUTH_SECRET`), einmalig,
  mit Ablauf, Sperre nach Fehlversuchen; Klartext nur einmal in der Antwort beim Erzeugen.
- **OIDC-Schnitt:** Tabelle `user_identity(provider, subject, user_id)`; jeder Anmeldeweg endet in
  `create_session(user_id)`. Später `z40-auth` (crates.io, 0.5.0, OIDC-Relying-Party) – Eignung **ungeprüft**.

## PDF während Phase 2

Optionen: (A) Node-Dienst als zweiter Prozess im Container; (B) Rust startet je PDF ein Node-Skript
(JSON auf stdin, PDF auf stdout); (C) PDF in Rust vorziehen (Phase 3); (D) PDF im Browser erzeugen – scheidet aus,
weil die Mieter-App ein Server-PDF lädt.

**Empfehlung B:** Rust berechnet alle Werte (eine Quelle der Wahrheit, `vermieterme-domain`) und übergibt sie fertig;
`pdf-renderer/` enthält nur noch das bestehende React-PDF-Dokument ohne Rechenlogik, ohne DB, ohne Netzwerk.
Zeitlimit und Parallelitätsgrenze im Rust-Prozess; Fehler → `500 {"error":"PDF-Generierung fehlgeschlagen"}`.
Kein zusätzlicher Port, kein zweiter Dauerprozess; Phase 3 tauscht nur die Brücke aus. Preis: Node-Laufzeit im Image.

## Datenmigration aus der Prisma-SQLite

Werkzeug: `vermieterme import-legacy` (Modul `legacy_import` in `vermieterme-db`). Liest die alte Datei nur lesend,
schreibt in die neue Datenbank (SQLite oder MySQL), in einer Transaktion je Lauf; die Altdatei bleibt unverändert
(Rückweg auf 1.x möglich). Beim ersten Start mit leerer neuer DB und vorhandener Altdatei läuft er automatisch
(Entscheidung 27). `--dry-run` schreibt nur den Bericht.

- Alle Daten gehen an den ersten Kunden; IDs bleiben erhalten (App-JWTs, Links).
- **Beträge Float → Cent:** kürzeste Dezimaldarstellung des f64 (Rust `Display`), dann kaufmännisch auf 2 Stellen
  (halb von null weg). VPI-Werte und `indexReferenceValue` → Zehntel als Ganzzahl.
- **Zeitpunkte:** Prisma speichert in SQLite Unix-Millisekunden (geprüft an `prisma/dev.db`). Kalenderdaten → UTC-Datum;
  Werte, die nicht auf 00:00 UTC liegen, kommen in den Bericht.
- **Verteilerschlüssel:** bekannte Texte (`MEA`, `laut Bescheid`, `siehe Anlage`, Groß-/Kleinschreibung egal) → Codes;
  unbekannte → Abbruch mit Bericht.
- **Einladungscodes:** Klartext → HMAC; Benutzer: bcrypt-Hash übernommen; Upload-Dateien nach `UPLOAD_DIR/{accountId}/`.
- **Prüfprotokoll** (`data/migration-report-<Zeit>.json`, ohne Namen/Adressen/IBAN): Zeilen je Tabelle alt/neu,
  Summen je Betragsspalte (Euro alt, Cent neu, Differenz), gerundete Werte mit ID, Abrechnungssummen je Zeitraum und
  Einheit alt vs. neu, fehlende Upload-Dateien. Abweichung > 0 Cent außerhalb der dokumentierten Rundung → Abbruch.

## Frontend: statischer Export

- `next.config.ts`: `output: "export"` im Produktions-Build; in der Entwicklung Weiterleitung `/api/*` an das Backend
  (Umsetzbarkeit per Phase-abhängiger Konfiguration **ungeprüft**).
- Fällt weg: `src/app/api/**`, NextAuth (`src/lib/auth.ts`, `src/proxy.ts`, `session-provider.tsx`, `next-auth/react`),
  Prisma (`prisma/`, `src/lib/prisma.ts`), `api-utils.ts`, `tenant-auth.ts`, `token.ts`, `password.ts`,
  `src/__tests__/api/*` (nach Portierung, Entscheidung 24). Server Components mit DB-Zugriff gibt es keine
  (alle Seiten sind Client-Seiten; geprüft per grep).
- Neu: Sitzungs-Hook über `GET /api/auth/session`, Login/Logout über die neuen Endpunkte, `X-CSRF-Guard` im
  API-Client, 401 → Login. Beträge: API in Cent, Umrechnung an genau einer Stelle im API-Client.
- Dynamische Route `billing/[id]` → `billing/detail?id=…` (Export kennt keine unbekannten IDs).
- Das Backend liefert `STATIC_DIR` aus (gleicher Origin, kein CORS); unbekannte Pfade außerhalb `/api` → `404.html`.

## Docker und Betrieb

Mehrstufig: Rust-Build (musl, statisch), Node-Build (statischer Export, PDF-Renderer-Bundle), Laufzeit
`node:22-alpine` mit Binary, `web/`, `pdf-renderer/`. Nicht-root (UID 1001), `VOLUME /app/data`, Port 3000,
Healthcheck `/api/health`. Einstiegspunkt `vermieterme serve` (Migrationen, ggf. Import, dann Start).
Image-Version 2.0; die Beispiel-`docker-compose.yml` nutzt heute das Tag `1.0`, Watchtower zieht 2.0 also nicht
ungefragt. Doku: `docs/deployment.md`, `docs/operations.md` (Konfigurationsreferenz aus `config.rs` erzeugt oder
per Test gegen den Code geprüft).

## CI

Öffentliches Repo → kein `fast-runner`. Parallele Jobs: `rust-lint` (fmt, clippy `-D warnings`), `rust-test-sqlite`,
`rust-test-mysql` (MariaDB-Dienst mit tmpfs), `cargo-deny`, Web-Jobs wie heute. `Swatinem/rust-cache` mit
`shared-key` und `save-if` nur auf `main`. Docker-Build mit Pfadfilter auch für `push`. Alle Jobs rufen `just`-Rezepte.

## Entscheidungen

1. **Mandanten (#44)** von Anfang an: Tabelle `account`, mehrere Benutzer je Kunde mit gleichen Rechten, Admin
   wechselt in Kunden mit Audit-Log und Hinweisleiste, Bestand → erster Kunde, Vermieterdaten, PDF-Vorlage und
   Kostenarten je Kunde. VPI-Werte bleiben global (öffentliche Statistik). Begründung: Nachrüsten wäre ein zweiter Umbau.
2. **Scope-Parameter** an jeder Datenzugriffsfunktion, fremde IDs → 404 (security-baseline, #44).
3. **Beträge in Cent (i64)** in DB und Domain (#70). Web-API liefert Cent (Felder mit Endung `Cents`), damit
   Rundungsfehler im Client ausgeschlossen sind und jede Verwechslung sofort auffällt. Mieter-App bekommt weiter
   Euro als JSON-Zahl, Umrechnung `cents / 100` nur in den Mieter-App-Handlern. Präzisiert #70 („API liefert Euro“
   gilt nur noch für die Mieter-App) – #70 entsprechend nachtragen.
4. **Vorauszahlung nach Kalendermonaten** (#70), überall dieselbe Domain-Funktion (Abrechnung, Mieter-App, PDF).
   Jeder berührte Kalendermonat zählt voll (Entscheidung 17); die Regel ist als `MonthRule` in `vermieterme-domain`
   austauschbar.
5. **Einheitenanteil:** Bei `mea` und `per_unit` wird der Anteil immer aus dem Gesamtbetrag berechnet, je Position in
   Cent kaufmännisch gerundet (auch im PDF, heute dort `0`, `src/app/api/billing-periods/[id]/pdf/route.tsx`); ein
   gespeicherter Einheitenbetrag wird dort ignoriert (#81). Gespeicherte Beträge je Einheit gelten nur bei
   `per_notice` und `per_attachment`. Deaktivierte Kosten zählen nirgends (#69). Rundung überall kaufmännisch
   (halbe Cent bzw. Basispunkte weg von der Null).
6. **Apple-Login entfällt** (#70); Login mit E-Mail/Passwort, OIDC später.
7. **Mieter-App-Vertrag eingefroren:** Pfade, Methoden, Felder, `Authorization: Bearer`, JWT-Format. Abweichungen nur
   als Sicherheitsfix, je Endpunkt begründet in `api-contract.md`.
8. **Bestehende App-Logins bleiben gültig** (gleiches Secret, gleiche Claims); neu ist der Widerruf über
   `app_tokens_revoked_at`.
9. **Einladungscodes** als HMAC, einmalig, mit Ablauf, Sperre nach Fehlversuchen (#70); Länge bleibt 6 (App-Eingabe).
10. **sqlx `Any` + Migrationen je Dialekt** (siehe Datenbank-Strategie).
11. **PDF über Node-Subprozess** bis Phase 3 (Option B).
12. **Web-API-Pfade bleiben unter `/api/…`** wie heute, damit die Oberfläche nur Felder, nicht Adressen umstellt.
13. **Ein Prozess, Rate-Limits im Speicher**: genügt für eine Instanz; mehrere Instanzen sind kein Ziel.
14. **Web-PDF je Einheit:** `GET /api/billing-periods/{id}/pdf?unitId=` (heute nur erste Einheit mit Mieter).
15. **Datumsformat:** Kalenderdaten intern `YYYY-MM-DD`; die Mieter-App erhält weiter `YYYY-MM-DDT00:00:00.000Z`.
16. **Paketmanager bleibt pnpm** (Projektregel AGENTS.md geht der Sprachregel „yarn 4“ vor).

## Entschieden am 2026-11-20 (vorher offene Fragen)

17. **Angebrochene Monate:** Jeder berührte Kalendermonat zählt voll (wie heute in `src/lib/billing.ts`).
    `MonthRule` bleibt austauschbar.
18. **`?token=` in PDF-/Datei-Links der App** (#69): wird nicht angenommen (Token in URL verstößt gegen die
    Sicherheits-Grundlinie). Heutiges Verhalten zur Laufzeit prüfen; Download per Header kommt mit einem App-Update.
19. **Nicht eingelöste Bestands-Einladungscodes:** gehasht übernehmen, Ablauf 30 Tage nach der Migration.
20. **Dokumente ohne Zuordnung** (#68): in der Mieter-App unsichtbar (404). Dokumente eines anderen Mieters im selben
    Zeitraum (#71) ebenso.
21. **Admin im Bestand:** Benutzer mit `ADMIN_EMAIL` wird Admin und Mitglied des ersten Kunden, alle übrigen Mitglieder.
22. **Passwort zurücksetzen ohne E-Mail:** einmaliger Link (24 h), vom Admin bzw. Kundenbenutzer erzeugt und angezeigt;
    kein SMTP in Phase 2.
23. **Neue Dependencies freigegeben** (Rust: axum, tokio, tower-http, sqlx, serde, serde_json, thiserror, anyhow,
    tracing, tracing-subscriber, argon2, bcrypt, jsonwebtoken, hmac, sha2, hkdf, rand, time, uuid, reqwest (rustls),
    infer; Node: esbuild für das Renderer-Bundle). `z40-fundament-http` nur, wenn es zur axum-Version passt (P0 prüft).
24. **Route-Tests `src/__tests__/api/*`** werden entfernt, sobald ihre Fälle als Rust-Integrationstests portiert sind.
25. **Backup:** Export nur Format 2 (Cent, je Kunde); Import von Format 1 und 2 in den aktuellen Kunden.
26. **#2:** Spalte `unit.kind` und Schlüssel `per_unit` (alle Einheiten des Objekts) in Phase 2, Ausnahmen später.
27. **Automatische Migration** beim ersten Start von 2.0: ja, Altdatei unverändert, Abbruch mit deutlicher Meldung
    bei Prüffehler.
28. **IBAN-Verschlüsselung:** nicht in Phase 2 (Schutz über Volume/Host), als Issue vorgemerkt.
29. **Einheitenbetrag je Einheit** (#80): Beträge „laut Bescheid“/„siehe Anlage“ werden je Einheit und Kostenposition
    gespeichert (Tabelle `cost_unit_amount`), nicht mehr einmal je Kostenposition. Altbestand: der bisherige Betrag
    wird für jede Einheit des Objekts übernommen (P9). Web-API und Vertrag: P2/P5 ergänzen das Feld je Einheit.
30. **Ein Kunde je Benutzer:** Mitgliedschaft als `app_user.account_id` (Admins ohne festen Kunden wechseln per
    Audit). Mehrere Kunden je Benutzer sind kein Ziel; bei Bedarf später eine Mitgliedschaftstabelle.
31. **Sperre gegen Durchprobieren von Einladungscodes** über Rate-Limit je IP und global im Speicher plus Ablauf und
    Einmaligkeit; kein Fehlversuchszähler je Code (ein Angreifer kennt den Code nicht, ein Zähler je Code schützt nicht).
32. **Gültigkeit von Links:** Einladung eines Web-Benutzers 7 Tage, Passwort-Reset 24 h; Token nur im URL-Fragment
    (`/accept-invite#token=…`, `/reset-password#token=…`), nie im Query-String. P12 legt diese Seiten an.
33. **Neuinstallation:** Der Bootstrap-Admin hat keinen Kunden, bis er im Admin-Bereich einen anlegt; die Oberfläche
    (P12) führt ihn dorthin. Beim Upgrade legt der Import (P9) den ersten Kunden an.
34. **CSP der Oberfläche:** Die API-Schicht setzt eine CSP ohne `unsafe-inline` für Skripte (`style-src` erlaubt Inline-Styles). Die Auslieferung des statischen
    Exports (P12) setzt einen eigenen CSP-Header mit Hashes der Inline-Skripte.
35. **Fehlerprotokoll:** Datenbankfehler werden nur mit Fehlerart und SQLSTATE/Code geloggt, nie mit der
    Treibermeldung (sie kann Feldwerte enthalten, z. B. bei doppelten Schlüsseln).
36. **Mieter umziehen:** Wechselt ein Mieter die Einheit, werden seine Mieter-App-Zugänge widerrufen
    (`app_tokens_revoked_at`, P4); zusätzlich lehnt die Mieter-App-API Tokens ab, deren `unitId` nicht mehr zur
    Einheit des Mieters passt (P7). Grund: Das Token trägt die Einheit, nach Neuvermietung gehören deren Daten
    einem anderen Mieter (#107).
37. **Einheit umhängen:** Eine Einheit mit Vorauszahlungen oder Einheitenbeträgen in Abrechnungen kann nicht in ein
    anderes Objekt verschoben werden (409), damit abgeschlossene Abrechnungen unverändert bleiben.
38. **Listen der Mieter und Mietänderungen** liefern wie bisher `unit{id,name,floor,shares,property{id,street,zip,city}}`
    eingebettet, damit die Oberfläche unverändert bleibt (#108). VPI-Vorschläge rechnen mit dem UTC-Datum
    (Abweichung höchstens ein Tag, für Monatswerte ohne Belang).
39. **Einheitenbetrag ohne Gesamtbetrag:** Bei `per_notice`/`per_attachment` zählt eine aktive Position mit Betrag
    der Einheit auch ohne Gesamtbetrag (z. B. Grundsteuer je Wohnung, #80); bei `mea`/`per_unit` braucht sie den
    Gesamtbetrag.
40. **Überlappung von Zeiträumen:** Datumsangaben sind inklusiv; ein gemeinsamer Grenztag gilt als Überlappung
    (sonst zählte der Grenzmonat nach Entscheidung 17 in beiden Zeiträumen). Geprüft beim Anlegen und Ändern; der
    Import (P9) übernimmt Altbestand unverändert und meldet solche Fälle im Prüfprotokoll.
41. **Isolationsstufe:** Abrechnungsänderungen (und Einheiten/Objekt-Löschen) laufen unter MySQL mit
    `READ COMMITTED`, damit keine Gap-Sperren über Objekt- und Kundengrenzen entstehen (#116); andere Pfade bleiben
    `REPEATABLE READ`. Betrieb: MySQL/MariaDB mit `binlog_format=ROW` (Standard) – Hinweis in `docs/operations.md` (P13).
42. **Umhängen löscht unwirksame Einheitenbeträge:** Wird eine Einheit ohne wirksame Abrechnungsdaten in ein anderes
    Objekt verschoben, löscht dieselbe Transaktion ihre bei `mea`/`per_unit` gespeicherten (unwirksamen)
    Einheitenbeträge; ergänzt Entscheidung 5 („bleiben gespeichert“ gilt nur ohne Umhängen).
43. **Dokumente:** Gehören Zeitraum und Mieter eines Dokuments zu verschiedenen Objekten → 400. Der Import (P9)
    benennt Altdateien in sichere Namen (`{uuid}.{ext}` im Kundenordner) um, damit Download und Aufräumen greifen;
    Dateien mit unbekanntem Typ werden als `application/octet-stream` übernommen und im Prüfprotokoll gemeldet.
44. **Speicherkontingent:** Je Kunde höchstens `UPLOAD_QUOTA_BYTES` (Standard 1 GiB) an Dokumenten und höchstens
    60 Uploads je Stunde; darüber 413 `quota_exceeded` bzw. 429 (#124).
45. **Zeitlimits:** Jede Anfrage hat ein Zeitlimit (`REQUEST_TIMEOUT_SECS`, Standard 30 s; Upload-Route 300 s).
    Schutz gegen langsame Verbindungen (Slowloris) übernimmt der vorgeschaltete Proxy (Betriebsdoku, P13).

## Arbeitspakete

Ein Umsetzer zur Zeit, Reihenfolge = Nummer. Je Paket ein eigener Reviewer,
höchstens 3 Korrekturschleifen. Qualitätstor je Paket: `just check`.

| ID | Ziel | erlaubte Pfade | Abh. | Modell | Akzeptanz | Issues |
|---|---|---|---|---|---|---|
| P0 | Workspace-Gerüst, `justfile`, CI-Jobs, Router-Slots | `backend/` (Cargo.toml, Lock, Toolchain, Lints, deny, leere Crates, Slots in `api/src/lib.rs` und `api/src/routes/mod.rs`), `justfile`, `.github/workflows/ci.yml`, `scripts/` (Zeilenprüfung), `.dockerignore` | – | sonnet | `just check` grün; leere Crates mit Doku; alle Workspace-Dependencies nach Entscheidung 23 eingetragen | – |
| P1 | Domain: Cent, Datum, Abrechnung, MEA, Monate (`MonthRule`), Schlüssel inkl. `per_unit`, VPI, IBAN | `backend/crates/domain/` | P0 | opus | Fälle aus `billing-calc.test.ts`, `vpi.test.ts`, `iban.test.ts` portiert; Rundungstests; keine IO-Abhängigkeit | #70, #69, #2 |
| P2 | Schema beider Dialekte, Pool, `Dialect`, Scope-Typen, Repos Kunde/Benutzer/Sitzung/Audit; `Any`-Spike | `backend/crates/db/`, `backend/migrations/` | P1 | opus | Migration 0001 vollständig für alle Tabellen; Schema-Gleichheitstest; Tests SQLite + MariaDB | #44 |
| P3 | Auth, Sitzungen, CSRF, Rate-Limit, Rechte-Mapping, Fehlerform, Konfiguration, Benutzer/Profil/Admin/Audit, Health | `backend/crates/api/src/{auth,config,error,admin,users,health}*` | P2 | opus | Login/Logout/Session; bcrypt→argon2id; Sitzungsende bei PW-Änderung; 404 für fremde Kunden; Audit bei Wechsel | #44, #70, #69 |
| P4 | Stammdaten: Objekte, Einheiten, Mieter, Mietänderungen, Kostenarten, Vermieter, PDF-Vorlage, VPI (+Abruf, Vorschläge) | `backend/crates/api/src/masterdata/`, `backend/crates/db/src/repo/masterdata*` | P3 | sonnet | Je Endpunkt Test inkl. fremder ID → 404 und Validierung → 400 | #69, #2 |
| P5 | Abrechnung: Zeiträume, Kopie, Kosten, Vorauszahlungen; Teil-Update, Version, 409 | `backend/crates/api/src/billing/`, `backend/crates/db/src/repo/billing*` | P4 | opus | Paralleltest gleiche Kostenart; Kopie übernimmt `enabled`/Override; leerer Betrag = `null`; vor dem Anlegen von Vorauszahlung/Einheitenbetrag die Einheit sperren (MySQL `FOR UPDATE`, Entscheidung 37) mit Paralleltest gegen Umhängen; Löschen einer Kostenart bei parallel angelegten Kosten → 409 statt 500 | #25, #67, #69 |
| P6 | Dokumente: Upload (Magic-Bytes), Ablage je Kunde, Download, Löschen inkl. Dateien | `backend/crates/api/src/documents/`, `backend/crates/db/src/repo/documents*` | P5 | opus | Typprüfung per Inhalt; keine Waisen beim Kaskadenlöschen; RFC-6266-Dateiname | #44 |
| P7 | Mieter-App-API (eingefroren), Einladungscodes, Widerruf | `backend/crates/api/src/tenant_app/`, `backend/crates/db/src/repo/tenant_app*` | P6 | opus | JSON-Fixtures aus heutigem Format byte-genau in Feldnamen/Typen; Alt-JWT aus Node akzeptiert; Brute-Force-Sperre | #68, #69, #70 |
| P8 | PDF-Brücke: `pdf-renderer/`, Aufruf aus Rust, Web- und Mieter-PDF | `pdf-renderer/`, `backend/crates/api/src/pdf/`, `src/lib/billing-pdf.tsx` (nur verschieben) | P7 | sonnet | PDF für Testdaten erzeugt; Zeitlimit greift; Summen aus Domain | #69 |
| P9 | `import-legacy`: Prisma-SQLite → neues Schema, Cent, Daten, Codes, Dateien, Prüfprotokoll | `backend/crates/db/src/legacy_import/`, Unterbefehl in `api/src/main.rs` | P8 | opus | Synthetische Alt-DB (Fixture, keine echten Daten) importiert, Bericht ohne Differenzen; `--dry-run` | #70, #44 |
| P10 | Backup Export v2 / Import v1+v2 je Kunde | `backend/crates/api/src/backup/`, `backend/crates/db/src/repo/backup*` | P9 | sonnet | Rundreise v2 verlustfrei; v1-Import gerundet wie P9; Import ersetzt nur den eigenen Kunden | – |
| P11 | Web: API-Client, Login/Session ohne NextAuth, CSRF-Header, Cent-Umrechnung | `src/lib/`, `src/features/`, `src/components/`, `src/app/` (ohne `api/`), Tests daneben | P10 | sonnet | Alle Web-Tests grün; keine `next-auth`-Importe; Beträge nur im API-Client umgerechnet | – |
| P12 | Web: statischer Export, Detailroute per Query, Admin-Bereich, Benutzerverwaltung, Hinweisleiste; Altteile entfernen | `src/`, `next.config.ts`, `package.json`, `pnpm-lock.yaml`, `prisma/` (löschen), `backend/crates/api/src/static_files*` | P11 | sonnet | `pnpm build` erzeugt `out/`; keine API-Routen/Prisma mehr | #44 |
| P13 | Docker, Compose-Beispiel, Einstiegspunkt, Doku Betrieb | `Dockerfile`, `docker-entrypoint.sh`, `docker-compose.yml`, `.github/workflows/docker.yml`, `docs/{development,deployment,operations}.md`, `README.md` | P12 | sonnet | Image startet mit Alt-Volume, migriert, Health 200; läuft als UID 1001 | – |
| P14 | Abnahme Mieter-App: Vertragstests gegen laufendes Image, Aufräumen | `backend/crates/api/tests/tenant_app_contract*`, `justfile` (Rezept `contract`) | P13 | haiku | Alle eingefrorenen Endpunkte gegen Fixtures grün | #70 |

**Vorab vergebene Konfliktstellen:**
- *Migrationen:* P2 legt `0001_initial.sql` (beide Dialekte) für **alle** Tabellen an. Nachträge nur mit vom
  Koordinator vergebener Nummer; reserviert: `0002` P5, `0003` P7, `0004` P9.
- *Router:* Bereichsmodule liegen an der Crate-Wurzel (`backend/crates/api/src/<bereich>/`, wie in der Paket-Tabelle).
  P0 legt in `api/src/lib.rs` die auskommentierten Moduldeklarationen und in `api/src/routes/mod.rs` die Funktion
  `router()` mit auskommentierten `merge(crate::<bereich>::router())`-Zeilen an (`auth`, `masterdata`, `billing`,
  `documents`, `tenant_app`, `pdf`, `backup`, `static_files`). Jedes Paket aktiviert nur seine zwei Zeilen; diese
  Zeilen gehören zu den erlaubten Pfaden jedes Pakets. `static_files` (Auslieferung von `out/`) aktiviert P12 mit dem
  Pfad `backend/crates/api/src/static_files*`.
- *Cargo.toml/Cargo.lock:* Workspace-Dependencies nur in P0; spätere Pakete aktivieren sie per `workspace = true`.
  Neue Dependency → Rückfrage beim Koordinator.
- *justfile:* P0 (Grundrezepte), P9 (`import-legacy`), P11 (`dev`), P13 (`docker`), P14 (`contract`); jedes Paket
  ergänzt nur sein Rezept (zählt zu seinen erlaubten Pfaden).

## Schnittstellen-Liste

| Umgebungsvariable | Standard | Bedeutung |
|---|---|---|
| `DATABASE_URL` | `sqlite://data/vermieterme-v2.db?mode=rwc` | SQLite oder `mysql://…` (MariaDB/MySQL) |
| `LEGACY_DATABASE_PATH` | `data/vermieterme.db` | Prisma-Datei für `import-legacy` und Auto-Import |
| `AUTH_SECRET` | – (Pflicht, ≥ 32 Zeichen) | Mieter-JWT (HS256, wie heute) und HKDF-Basis für Einladungscode-HMAC |
| `ADMIN_EMAIL`, `ADMIN_PASSWORD` | – | nur Bootstrap, wenn kein Admin existiert |
| `HOST` / `PORT` | `0.0.0.0` / `3000` | Bind-Adresse |
| `STATIC_DIR` | `web` | statischer Export der Oberfläche |
| `UPLOAD_DIR` | `data/uploads` | Dokumente, Unterordner je Kunde |
| `COOKIE_SECURE` | `true` | `false` nur lokal ohne TLS |
| `SESSION_IDLE_HOURS` / `SESSION_MAX_DAYS` | `168` / `30` | Sitzungsdauer Web |
| `INVITE_CODE_TTL_DAYS` | `30` | Ablauf Einladungscode |
| `TRUSTED_PROXIES` | leer | CIDR-Liste, nur von dort `X-Forwarded-*` |
| `PDF_RENDERER_CMD` | `node pdf-renderer/render.mjs` | PDF-Brücke |
| `PDF_RENDER_TIMEOUT_SECS` / `PDF_RENDER_CONCURRENCY` | `20` / `2` | Grenzen der PDF-Brücke |
| `VPI_API_BASE` | Bundesbank-SDMX-Basis-URL (heute in `src/app/api/vpi/fetch/route.ts`) | austauschbar für Tests |
| `LOG_LEVEL` / `LOG_FORMAT` | `info` / `json` | Logging (personenbezogene Felder geschwärzt) |
| `UPLOAD_QUOTA_BYTES` | `1073741824` | Speicher je Kunde für Dokumente |
| `REQUEST_TIMEOUT_SECS` | `30` | Zeitlimit je Anfrage (Upload 300 s) |
| `PASSWORD_HASH_CONCURRENCY` | `4` | gleichzeitige argon2-Berechnungen; Wartezeit > 5 s → 429 |
| `TEST_MYSQL_URL` | – | nur Tests (MariaDB); fehlt → `mysql_*`-Tests `[skip]`, `just test-mysql` bricht ab |

Entfällt: `AUTH_TRUST_HOST`, `AUTH_APPLE_ID`, `AUTH_APPLE_SECRET`, `NEXT_TELEMETRY_DISABLED` (Laufzeit).

- **Crates:** `vermieterme-domain`, `vermieterme-db`, `vermieterme-api`; Binary `vermieterme`.
- **Pfade:** `backend/`, `backend/migrations/{sqlite,mysql}/NNNN_name.sql`, `pdf-renderer/`, Export `out/` →
  im Image `/app/web`; Daten `/app/data`.
- **Header/Cookies:** `vm_session`, `X-CSRF-Guard: 1`, Mieter `Authorization: Bearer <jwt>`.
- **just-Rezepte:** `check` (alles, wie CI), `fmt`, `lint`, `test`, `test-mysql`, `build`, `lines`, `web-check`,
  `dev` (Backend + Next-Dev), `import-legacy`, `docker`, `contract`.
- **PDF-Brücke:** stdin JSON `{landlord, property, billingPeriod, unit, tenant, costs[], totals, templateConfig}`
  (Beträge als Euro-Zahl, Daten ISO), stdout PDF, Exit ≠ 0 = Fehler; Details legt P8 in `pdf-renderer/README` fest.

## Risiken

- Upgrade bestehender Installationen (Datenmigration, Cookies ungültig → einmal neu anmelden); Rückweg nur über
  die unveränderte Altdatei und Image 1.x.
- `Any`-Treiber-Grenzen (Typen, Upsert) – früh in P2 prüfen.
- Mieter-App-Kompatibilität hängt an Details (Datumsformat mit Millisekunden, Zahlen statt Strings, Fehlertexte);
  P7/P14 sichern das per Fixtures.
- Summen ändern sich gegenüber heute (Kalendermonate im PDF, MEA im PDF, deaktivierte Kosten in der App-Liste) –
  gewollt (#69/#70), im Migrationsbericht sichtbar machen und in den Release-Hinweisen nennen.
