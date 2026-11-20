# Schnittstellen-Vertrag (Phase 2, Rust-Backend)

Stand: 2026-11-20. Plan und Entscheidungen: [`PLAN.md`](PLAN.md). Belege `Datei:Zeile` beziehen sich auf den
heutigen Next.js-Stand (vor Phase 2). Änderungen an dieser Datei kommen **vor** Server und Clients.

## 1. Grundlagen

- Basis: gleicher Origin wie die Oberfläche, alle Endpunkte unter `/api/`. JSON, UTF-8, Feldnamen `camelCase`.
- **Beträge (Web):** Ganzzahl in Cent, Feldname endet auf `Cents` (z. B. `totalAmountCents`). **Mieter-App:**
  Euro als JSON-Zahl wie heute (Server rechnet `cents / 100`).
- **Kalenderdaten (Web):** `"YYYY-MM-DD"`. **Zeitpunkte:** ISO 8601 UTC mit Millisekunden
  (`"2026-01-31T12:00:00.000Z"`). **Mieter-App:** Kalenderdaten als `"YYYY-MM-DDT00:00:00.000Z"` wie heute
  (Prisma/`Date.toJSON`).
- **IDs:** Strings (Bestand: cuid, neu: UUID v7). Clients dürfen das Format nicht auswerten.
- **VPI-Werte (Web):** Zehntel als Ganzzahl (`valueTenths`, z. B. 1196 = 119,6).
- **Berechtigung:** `öffentlich` · `Mieter` (Mieter-JWT) · `Kunde` (Web-Sitzung, Rolle `member` oder `admin` im
  aktiven Kunden) · `Admin` (Rolle `admin`). Scope `Kunde` = Kunde aus der Sitzung (bzw. gewechselter Kunde des
  Admins), nie aus der Anfrage. IDs eines anderen Kunden verhalten sich wie nicht vorhandene: **404**.
- **Web-Sitzung:** Cookie `vm_session`. Mutierende Web-Anfragen brauchen Header `X-CSRF-Guard: 1`
  (fehlt → `403 {"error":"Anfrage abgelehnt","code":"csrf"}`).

### Fehlerform

Web: `{"error": string, "code"?: string, "fields"?: {[feld]: string}}` – `error` deutsch, für Menschen;
`code` maschinenlesbar; `fields` bei Validierungsfehlern. Mieter-App: nur `{"error": string}` (wie heute,
`src/lib/api-utils.ts:24-36`; die App liest nur `error`, `apps/mieter-app/lib/api.ts:40-42`).

| Status | `code` | Bedeutung |
|---|---|---|
| 400 | `validation` | Eingabe ungültig (`fields` gesetzt) |
| 401 | `unauthenticated` | keine/abgelaufene Sitzung bzw. Token |
| 403 | `forbidden`, `csrf` | Rolle reicht nicht (nur für eigene, bekannte Ressourcen) / CSRF-Header fehlt |
| 404 | `not_found` | nicht vorhanden **oder** fremder Kunde |
| 409 | `conflict`, `version_conflict`, `overlap` | Eindeutigkeit, veraltete Version, überlappender Zeitraum |
| 405 | `method_not_allowed` | Methode für den Pfad nicht erlaubt |
| 413 | `too_large` | Anfrage zu groß (Upload > 10 MB, sonstige Bodies > 2 MB) |
| 429 | `rate_limited` | zu viele Versuche (mit `Retry-After`) |
| 500 | `internal` | `{"error":"Interner Fehler"}`, keine Details |
| 502 | `upstream` | externer Dienst (Bundesbank) fehlgeschlagen |

„Nicht gefunden“ aus der Datenbank wird immer 404 (heute 500, #69).

## 2. Mieter-App – EINGEFROREN

Pfade, Methoden, Felder, Typen und Header unverändert. Auth: `Authorization: Bearer <jwt>`
(`apps/mieter-app/lib/api.ts:29`), JWT HS256, Claims `{tenantId, unitId}`, `exp` 30 Tage (`src/lib/tenant-auth.ts:16-22`).
401 löscht in der App das Token (`api.ts:35-38`). Typen: `apps/mieter-app/lib/types.ts`.

**Gemeinsame Abweichung (Sicherheitsfix):** Jede Mieter-Anfrage prüft zusätzlich in der DB, dass Mieter und Kunde
existieren, nicht gesperrt sind und das Token nicht widerrufen ist. Sonst `401 {"error":"Ungültiges oder abgelaufenes
Token"}` – heute liefern gelöschte Mieter 404 bzw. `[]` (z. B. `src/app/api/tenant-app/profile/route.ts:19-21`,
`billing-periods/route.ts:15-17`). Begründung: Widerruf (#70); die App behandelt 401 bereits korrekt.

### POST /api/tenant-app/auth — öffentlich
Request `{"code": string}` (Server: `toUpperCase().trim()`, `src/app/api/tenant-app/auth/route.ts:15`).
Response 200 (`auth/route.ts:36-51`, `types.ts:91-107`):
```json
{"token":"<jwt>","tenant":{"id":"…","firstName":"…","lastName":"…"},
 "unit":{"id":"…","name":"…"},"property":{"street":"…","zip":"…","city":"…"}}
```
Fehler: 400 `Einladungscode fehlt` (`:10-12`), 401 `Ungültiger Einladungscode` (`:27-29`).
**Abweichungen (Sicherheitsfix, #70):** Code gehasht, einmalig, mit Ablauf; abgelaufen/verbraucht → ebenfalls 401
`Ungültiger Einladungscode` (keine Unterscheidung nach außen). Neu 429 `{"error":"Zu viele Versuche. Bitte später erneut
versuchen."}` nach Fehlversuchen je IP. Gleiche Antwortzeit für gültige/ungültige Codes.

### GET /api/tenant-app/profile — Mieter
Response 200 (`profile/route.ts:23-50`, `types.ts:1-29`): `id, salutation, firstName, lastName, salutation2|null,
firstName2|null, lastName2|null, phone|null, email|null, bankName|null, iban|null, accountHolder|null, moveInDate,
moveOutDate|null, unit{id,name,floor,shares}, property{id,street,zip,city,totalShares}`.
Fehler: 401 (siehe oben; heute 404 `Mieter nicht gefunden`).

### PUT /api/tenant-app/profile — Mieter
Request: beliebige Teilmenge von `{phone, email, bankName, iban, accountHolder}`; fehlendes Feld = unverändert,
`null` = leeren (`profile/route.ts:60-70`). Response 200 `{id, phone, email, bankName, iban, accountHolder}` (`:73-80`).
**Abweichung (Validierung):** `iban` wird serverseitig geprüft (Prüfziffer), `email` auf Form; ungültig →
`400 {"error":"IBAN ist ungültig"}` bzw. `{"error":"E-Mail-Adresse ist ungültig"}`. Die App zeigt `error` an.

### GET /api/tenant-app/billing-periods — Mieter
Response 200: Array, sortiert `startDate` absteigend (`billing-periods/route.ts:27,38-53`, `types.ts:31-46`):
`id, startDate, endDate, billingDate|null, sentDate|null, paidDate|null, property{street,zip,city},
totalUnitCosts, totalPrepayment, difference` (Euro).
**Abweichungen (Korrektur, #69/#70):** deaktivierte Kosten zählen nicht mehr mit (heute `:31` ungefiltert);
Summen aus der Domain (Kalendermonate, MEA-Anteil wie im Detail). Nur Zeiträume des eigenen Kunden.

### GET /api/tenant-app/billing-periods/{id} — Mieter
Response 200 (`billing-periods/[id]/route.ts:77-101`, `types.ts:48-78`):
`id, startDate, endDate, billingDate, sentDate, paidDate, property{street,zip,city},
costs[{id, category, distributionKey, totalAmount, unitAmount}],
prepayments[{id, monthlyAmount}], totals{totalCosts, totalUnitCosts, totalPrepayment, difference},
documents[{id, originalName, mimeType, size, category, createdAt}]`.
`distributionKey` als heutiger Anzeigetext (`MEA`, `laut Bescheid`, `siehe Anlage`; neu ggf. `nach Einheiten`, #2).
`unitAmount` (Euro): bei `MEA` und `nach Einheiten` immer aus `totalAmount` berechnet (je Position auf Cent gerundet);
bei `laut Bescheid`/`siehe Anlage` der gespeicherte Betrag **dieser Einheit**, sonst `0` (Plan, Entscheidungen 5 und 29).
**Abweichung (#81, #80):** heute gewinnt ein gespeicherter Betrag auch bei MEA, und er gilt für alle Einheiten gleich
(`:47-57`).
Fehler: 404 `Abrechnungszeitraum nicht gefunden` (`:34-36`).
**Abweichung (Sicherheitsfix, #68):** `documents` enthält nur Dokumente ohne Mieterzuordnung oder des eigenen
Mieters (heute alle Dokumente des Zeitraums, auch die anderer Mieter, `:30,68-75`). Die Felder `billingPeriodId`/
`tenantId` aus `DocumentItem` fehlen weiterhin (wie heute).

### GET /api/tenant-app/billing-periods/{id}/pdf — Mieter
Response 200 `application/pdf`, `Content-Disposition: attachment; filename="Betriebskostenabrechnung-{Jahr}.pdf"`
(`billing-periods/[id]/pdf/route.tsx:105-110`). Fehler: 404 `Abrechnungszeitraum nicht gefunden`, 400
`Vermieterinformationen nicht konfiguriert`, 500 `PDF-Generierung fehlgeschlagen` (`:49-62,118-122`).
**Abweichungen (#69/#70):** Vorauszahlung nach Kalendermonaten statt Tage/30,44 (`:83-86`); MEA-Anteil wie im Detail.
`?token=` wird **nicht** ausgewertet (wie heute; App-Links `api.ts:85-95`) – Plan, Entscheidung 18.

### GET /api/tenant-app/documents — Mieter
Response 200: Array, `createdAt` absteigend (`documents/route.ts:33-46`, `types.ts:80-89`):
`id, originalName, mimeType, size, category, billingPeriodId|null, tenantId|null, createdAt`.
**Abweichung (Sicherheitsfix, #68):** heute alle Dokumente mit eigenem `tenantId` **oder** einem Zeitraum des Objekts
(`:26-32`), also auch Dokumente anderer Mieter desselben Zeitraums. Neu: eigener Mieter **oder** (Zeitraum des Objekts
**und** ohne Mieterzuordnung). Dokumente ganz ohne Zuordnung: nicht sichtbar (Plan, Entscheidung 20; #71).

### GET /api/tenant-app/documents/{id}/file — Mieter
Response 200: Bytes, `Content-Type` = gespeicherter Typ, `Content-Disposition: inline; filename*=UTF-8''…`.
**Abweichungen (Sicherheitsfix, #68):** fremde/unzugeordnete Dokumente → `404 {"error":"Dokument nicht gefunden"}`
(heute 403 `Zugriff verweigert`, `documents/[id]/file/route.ts:32-43`; ohne Zuordnung heute ausgeliefert).
Dateiname nach RFC 6266 (heute `encodeURIComponent` im Quoted-String, `:51`). `?token=` wie beim PDF.

### GET /api/health — öffentlich
Response 200 `{"status":"ok","version":"2.x.y"}`, 503 `{"status":"error"}` (`src/app/api/health/route.ts`).
Die App prüft nur `res.ok` (`apps/mieter-app/app/login.tsx:49-50`). Unverändert.

## 3. Anmeldung und Sitzung (Web) — neu

| Methode Pfad | Berechtigung | Request | Response | Fehler |
|---|---|---|---|---|
| POST /api/auth/login | öffentlich | `{email, password}` | 200 Session (s. u.), setzt `vm_session` | 401 `Anmeldung fehlgeschlagen` (generisch), 429 |
| POST /api/auth/logout | Sitzung | – | 204, löscht Cookie | – |
| GET /api/auth/session | Sitzung | – | 200 Session | 401 |
| POST /api/auth/accept-invite | öffentlich | `{token, name, password}` | 200 Session | 400 (Passwort < 10 Zeichen), 401 Token ungültig/abgelaufen, 429 |
| POST /api/auth/reset-password | öffentlich | `{token, password}` | 204; beendet alle Sitzungen des Benutzers | 400, 401, 429 |

Session: `{"user":{"id","name","email","role":"admin"|"member"},"account":{"id","name"}|null,"impersonating":bool}`.
`POST /api/auth/login` ersetzt `/api/auth/[...nextauth]` (entfällt ganz, ebenso Apple-Login).
Passwortregel überall: mindestens 10 Zeichen, höchstens 256 (heute 8 bei `/api/users`, keine bei `/api/profile`, #69).

## 4. Admin — neu (#44)

Alle: Berechtigung `Admin`; jede Änderung erzeugt einen Audit-Eintrag.

| Methode Pfad | Request | Response | Fehler |
|---|---|---|---|
| GET /api/admin/accounts | – | `[{id,name,locked,createdAt,userCount}]` | – |
| POST /api/admin/accounts | `{name}` | 201 Account | 400 |
| PATCH /api/admin/accounts/{id} | `{name?, locked?}` | Account; Sperre beendet Sitzungen und Mieter-Tokens des Kunden | 404 |
| GET/POST /api/admin/accounts/{id}/users | POST `{name, email}` | GET `[User]`; POST 201 `{user, inviteUrl, expiresAt}` | 404, 409 `E-Mail existiert bereits` |
| PATCH /api/admin/users/{id} | `{locked?, role?}` | User | 400 eigener Admin, 404 |
| POST /api/admin/users/{id}/reset-link | – | 201 `{resetUrl, expiresAt}` (einmalig angezeigt) | 404 |
| POST /api/admin/switch | `{accountId}` | 200 Session (`impersonating: true`) | 404 |
| DELETE /api/admin/switch | – | 200 Session | – |
| GET /api/admin/audit-log?accountId=&cursor= | – | `{items:[{id,at,userId,userEmail|null,accountId,action,entity,entityId}], nextCursor|null}` (max. 100, neueste zuerst) | `cursor` ist undurchsichtig (Base64url aus `at` und `id`), ungültig → 400; `userEmail` ist `null`, wenn der Benutzer gelöscht ist |

Audit-Einträge enthalten keine personenbezogenen Feldwerte (IBAN, Adressen), nur Aktion, Entität, ID.

## 5. Benutzer und Profil — Kunde

| Methode Pfad | Request | Response | Fehler / Abweichung |
|---|---|---|---|
| GET /api/profile | – | `{id,name,email}` | heute zusätzlich `image` (entfällt mit Apple) |
| PUT /api/profile | `{name?, currentPassword?, newPassword?}` | `{id,name,email}` | 400 Passwort falsch/zu kurz; Umgebungs-Passwort nicht mehr gültig (#69); Passwortwechsel beendet andere Sitzungen |
| GET /api/users | – | `[{id,name,email,role,locked}]` des eigenen Kunden | heute alle Benutzer |
| POST /api/users/invitations | `{name, email}` | 201 `{user, inviteUrl, expiresAt}` | ersetzt `POST /api/users` mit Passwort; 409 |
| DELETE /api/users/{id} | – | 204 | 400 selbst; 404 fremd |
| POST /api/users/{id}/reset-link | – | 201 `{resetUrl, expiresAt}` | 404 |

## 6. Stammdaten — Kunde

Gemeinsam: unbekannte oder fremde ID → 404; Validierungsfehler → 400 mit `fields`; Erfolg beim Anlegen 201,
beim Löschen 204 (heute `{success:true}` mit 200). Antworten liefern das vollständige Objekt.

| Ressource | Endpunkte | Felder (Request = Response ohne `id`, `createdAt`, `updatedAt`) | Abweichungen |
|---|---|---|---|
| Objekte | GET/POST `/api/properties`, GET/PUT/DELETE `/api/properties/{id}` | `street, zip, city, totalShares (int ≥ 1)`; Liste mit `unitCount`, Detail mit `units[]` inkl. `tenants[]` | Liste: `unitCount` statt `_count.units`; 404 deutsch |
| Einheiten | GET `/api/units?propertyId=`, POST, PUT/DELETE `/api/units/{id}` | `propertyId, name, floor, shares (int ≥ 0, ≤ `totalShares` des Objekts), kind ("apartment"|"garage"|"carport"|"other")` | `kind` neu (#2); `shares > totalShares` → 400 (heute ungeprüft, #85); ebenso lehnt das Objekt ein `totalShares` unter der größten Einheit ab |
| Mieter | GET `/api/tenants?unitId=`, POST, PUT/DELETE `/api/tenants/{id}` | `unitId, salutation, firstName, lastName, salutation2?, firstName2?, lastName2?, phone?, email?, bankName?, iban?, accountHolder?, moveInDate, moveOutDate?, leaseType ("standard"|"index"|"staffel"), indexBaseYear?, indexReferenceValueTenths?, indexReferenceDate?, indexMinMonths` | IBAN serverseitig geprüft; VPI-Bezug in Zehnteln |
| Mieter-App-Zugang | GET/POST/DELETE `/api/tenants/{id}/access-token` | GET `{hasToken, expiresAt, usedAt}`; POST 201 `{code, expiresAt}`; DELETE 204 | **Code nur einmal im POST** (heute GET liefert Klartext); DELETE widerruft zusätzlich alle App-Tokens des Mieters (#70) |
| Mietänderungen | GET `/api/rent-changes?unitId=`, POST, PUT/DELETE `/api/rent-changes/{id}` | `unitId, type ("rent"|"prepayment"), amountCents, effectiveDate, reason?` | `type` geprüft; Cent |
| Kostenarten | GET/POST `/api/cost-categories`, PUT/DELETE `/api/cost-categories/{id}` | `name, distributionKey ("mea"|"per_notice"|"per_attachment"|"per_unit"), sortOrder` | je Kunde; Schlüssel als Code; Löschen mit Kosten → 409 statt Kaskade |
| Vermieter | GET/PUT `/api/settings` | `name, street, zip, city, phone?, email?, bankName?, iban?, accountHolder?` | je Kunde; PUT statt POST; Body geprüft (#69); GET ohne Eintrag 200 `null` |
| PDF-Vorlage | GET/PUT `/api/pdf-template` | GET `{id, name, config}`; PUT `{config, name?}` → `{id, name}` | je Kunde; ungültig → 400 statt 500 (#69); Default wie `src/lib/pdf-template.ts` |

## 7. Abrechnung — Kunde

| Methode Pfad | Request | Response | Fehler / Abweichung |
|---|---|---|---|
| GET /api/billing-periods?propertyId= | – | `[BillingPeriodSummary]` mit `costCount`, `unreviewedCount` | – |
| POST /api/billing-periods | `{propertyId, startDate, endDate, billingDate?, copyFromId?}` | 201 Detail | 400 Pflicht/Ende ≤ Beginn/Objekt ohne Einheiten; 404; 409 `overlap`. Kopie übernimmt `enabled` und `distributionKeyOverride` (heute nicht, #69), setzt `reviewed=false`; atomar in einer Transaktion |
| GET /api/billing-periods/{id} | – | Detail: Zeitraum, `property{…, units[{…, tenants[]}]}`, `costs[]`, `prepayments[]`, `totalsByUnit[{unitId, totalUnitCostsCents, totalPrepaymentCents, differenceCents, months}]` | Summen serverseitig (Domain). Ungültige Anteile einer Einheit bei aktiver MEA-Position → 422 `invalid_shares`, Meldung nennt die Einheit (#85) |
| PUT /api/billing-periods/{id} | `{startDate?, endDate?, billingDate?, sentDate?, paidDate?}` (`null` = leeren) | Detail | 409 `overlap` (heute keine Prüfung); `propertyId` nicht mehr änderbar |
| DELETE /api/billing-periods/{id} | – | 204 | löscht zugehörige Upload-Dateien mit |
| GET /api/billing-periods/{id}/costs | – | `[Cost]`; `Cost.unitAmounts[{unitId, amountCents}]` (nur gesetzte) | Beträge je Einheit neu (#80) |
| PATCH /api/billing-periods/{id}/costs/{costCategoryId} | `{totalAmountCents?, enabled?, distributionKeyOverride?, reviewed?, version?}` | 200 Cost (neu angelegt: 201) | ersetzt `POST …/costs` (Upsert). Nur gesendete Felder ändern sich, ein `UPDATE` (#25). `null` leert (`totalAmountCents: null` → Position ohne Betrag, #25). `reviewed` wird nur `false`, wenn sich ein Betrag wirklich ändert (#67). `version` gesendet und veraltet → 409 `version_conflict` mit aktuellem Stand in `current` (#67) |
| GET /api/billing-periods/{id}/prepayments | – | `[Prepayment]` | – |
| PATCH /api/billing-periods/{id}/costs/{costCategoryId}/units/{unitId} | `{amountCents (int\|null), version?}` | 200 Cost | Betrag dieser Einheit für `per_notice`/`per_attachment` (#80); `null` leert; fremde Einheit → 404; Version wie Kosten (#67). `unitAmountCents` im Kosten-`PATCH` entfällt |
| PATCH /api/billing-periods/{id}/prepayments/{unitId} | `{monthlyAmountCents?, reviewed?, version?}` | 200/201 Prepayment | wie Kosten (#25, #67) |
| GET /api/billing-periods/{id}/pdf?unitId= | – | `application/pdf`, Name wie heute | `unitId` optional; ohne: erste Einheit mit Mieter im Zeitraum (wie heute). 400 `Kein aktiver Mieter im Zeitraum`, 400 `Vermieterinformationen nicht konfiguriert` (heute englisch) |

`Cost`: `{id, billingPeriodId, costCategoryId, categoryName, distributionKey (wirksam), distributionKeyOverride|null,
totalAmountCents|null, unitAmounts[{unitId, amountCents}], enabled, reviewed, version}`.
`Prepayment`: `{id, billingPeriodId, unitId, monthlyAmountCents, reviewed, version}`.

## 8. Dokumente — Kunde

| Methode Pfad | Request | Response | Fehler / Abweichung |
|---|---|---|---|
| GET /api/documents?billingPeriodId=&tenantId= | – | `[{id, originalName, mimeType, size, category, billingPeriodId, tenantId, createdAt}]` | – |
| POST /api/documents | multipart: `file`, `billingPeriodId?`, `tenantId?`, `category?` | 201 Document | 400 Typ (PDF/JPEG/PNG/WebP **per Inhalt** geprüft, heute nur Client-Header), 413 > 10 MB, 404 fremde Zuordnungs-IDs (heute ungeprüft), `category` ∈ `invoice|receipt|contract|other` |
| GET /api/documents/{id}/file | – | Bytes inline | 404; Datei fehlt → 404 statt 500 |
| DELETE /api/documents/{id} | – | 204 | – |

## 9. VPI — Kunde (Daten global)

| Methode Pfad | Request | Response | Fehler / Abweichung |
|---|---|---|---|
| GET /api/vpi | – | `[{id, year, month, valueTenths, baseYear}]` | Zehntel statt Float |
| POST /api/vpi | `{year, month, valueTenths, baseYear}` | 201 (Upsert auf `year+month+baseYear`) | 400; schreibend nur `Admin`, da global (neu) |
| DELETE /api/vpi/{id} | – | 204 | nur `Admin` |
| POST /api/vpi/fetch | `{baseYear: 2020|2015|2010}` | `{entries:[{year,month,valueTenths}], baseYear}` | 400, 404 leer, 502 |
| GET /api/vpi/suggestions | – | `[{tenantId, unitId, tenantName, unitName, propertyAddress, leaseType, currentRentCents, referenceVpiTenths, currentVpiTenths, percentageChangeBasisPoints, suggestedRentCents, monthsSinceLastAdjustment, minMonths, eligible, baseYear}]` | Logik in die Domain (heute inline, `src/app/api/vpi/suggestions/route.ts:39-89`) |

## 10. Backup — Kunde

| Methode Pfad | Request | Response | Fehler / Abweichung |
|---|---|---|---|
| GET /api/backup | – | Datei `{version:2, exportedAt, data:{…}}` nur des eigenen Kunden, Beträge in Cent | heute Version 1, alle Daten |
| POST /api/backup | Format 1 oder 2 | 200 `{importedAt, counts:{…}}` | ersetzt nur Daten des eigenen Kunden, in einer Transaktion, jede Zeile validiert; 400 `Ungültiges Backup-Format` |

## 11. Übersicht der Abweichungen gegenüber heute

| Bereich | Abweichung | Bezug |
|---|---|---|
| Mieter-App, alle | DB-Prüfung des Tokens, gelöschter/gesperrter Mieter → 401 | #70 |
| Mieter-App, auth | Code gehasht, einmalig, Ablauf, 429 | #70 |
| Mieter-App, Liste/Detail/PDF | deaktivierte Kosten nie, Kalendermonate, MEA im PDF | #69, #70 |
| Mieter-App, Dokumente | nur eigene bzw. Zeitraums-Dokumente ohne Mieterzuordnung; fremd → 404 | #68 |
| Mieter-App, Profil | IBAN/E-Mail serverseitig geprüft | – |
| Web, alle | Cent-Felder, Datumsstrings, 204 beim Löschen, 404 statt 500, deutsche Fehlertexte, Scope je Kunde | #44, #69, #70 |
| Web, Kosten/Vorauszahlungen | `PATCH` je Position statt `POST`-Upsert, Version, 409 | #25, #67 |
| Web, Auth | eigene Endpunkte statt NextAuth, kein Apple-Login, CSRF-Header | #70 |
