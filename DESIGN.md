# Gestaltung

Die Oberfläche baut auf [`@zweiund40gmbh/ui`](https://www.npmjs.com/package/@zweiund40gmbh/ui) (genaue Version in
`package.json`) und den Design-Tokens in `src/app/globals.css`. Ziel: sachlich, dicht, gut lesbar – ein Werkzeug,
keine Landingpage.

## Grundsätze

- **Systemschrift** (`--app-sans`), keine Web- oder Serifenschriften, kein `font-serif`, keine Inline-`fontFamily`.
- **Eckig:** `--r: 0`. Keine `rounded-*`-Klassen (Ausnahme: echte Kreise wie Status-Punkte, `rounded-full`).
- **Flach:** keine `shadow-*`, keine Verläufe, keine Glas-/Blur-Effekte, keine dekorativen Icons in Kreisen.
- **Farben nur über Tokens**, nie Tailwind-Palettenfarben (`zinc-*`, `red-*`, `gray-*`, `green-*` …) oder Hex-Werte.
  Hell und Dunkel funktionieren damit automatisch.
- **Akzent sparsam:** Petrol (`--accent`) für primäre Aktion, aktive Navigation, Links. Status über
  `ok`/`warn`/`bad`/`info` (+ `*-soft` als Fläche).
- **Zahlen** rechtsbündig mit `num` (tabellarische Ziffern), Beträge über `src/lib/format.ts`.
- **Texte** deutsch mit Umlauten, Nutzer wird gesiezt; Überschriften kurz und ohne Emoji.

## Tailwind-Klassen der Tokens

| Zweck | Klasse |
|---|---|
| Seitenhintergrund / Fläche / Feld / Hover | `bg-bg`, `bg-layer`, `bg-field`, `bg-hover` |
| Linien | `border-line`, `border-strong` |
| Text (stark → schwach) | `text-ink`, `text-ink2`, `text-ink3` |
| Akzent | `text-accent`, `text-accentstrong` (Links), `bg-accentsoft` |
| Status | `text-ok`, `text-warn`, `text-bad`, `text-info` |
| Bibliothek | zusätzlich `bg-zui-*`, `text-zui-*`, `border-zui-*` aus `@zweiund40gmbh/ui/tailwind.css` |

Hilfsklassen (`globals.css`): `panel` (Fläche mit Rahmen und Innenabstand), `page-title`, `section-title`,
`label-caps`, `num`, `table-plain` (einfache Tabelle).

## Komponenten

Immer zuerst die Bibliothek (Referenz: `docs/components/*.md` im Repo `react-components` bzw. die Doku-Website).

| Bedarf | Komponente |
|---|---|
| Seitenkopf mit Titel, Beschreibung, Aktionen | `PageHeader` |
| Knopf | `Button` (`primary` für die eine Hauptaktion, sonst `secondary`/`ghost`, `danger` zum Löschen), `IconButton` |
| Formularfeld | `Field` + `TextInput` / `Textarea` / `MoneyInput` / `DateInput` / `PasswordInput` / `Switch` |
| Auswahl | `Select`, `MultiSelect`, `AsyncSelect`, `SegmentedControl` – nie natives `<select>` |
| Dialog / Seitenpanel | `Modal`, `Sheet` |
| Bestätigung | `useConfirm()` (Provider ist eingebunden) |
| Tabelle | `DataTable` (Sortierung, Suche, Auswahl) oder `table-plain` für kleine, statische Tabellen |
| Kennzahlen | `StatTile` |
| Leer / Laden / Fehler | `EmptyState`, `LoadingState`, `SkeletonRows`, `ErrorState`, `InlineNotice` |
| Reiter | `Tabs` |
| Status-Kennzeichen | `Tag` |
| Datei-Upload | `FileDropzone` |

Fehlt etwas Allgemeines, wird es in `@zweiund40gmbh/ui` ergänzt, nicht lokal nachgebaut. Fachliche Bausteine
liegen in `src/features/<bereich>/`.

## Aufbau einer Seite

`src/app/(app)/<pfad>/page.tsx` bleibt dünn (Daten laden, Bereich rendern). Teile und Dialoge liegen in
`src/features/<bereich>/` (je Datei < 300 Zeilen). Der Rahmen (Kopfzeile, Navigation, Breite, Innenabstand) kommt
aus `src/app/(app)/layout.tsx`; Seiten setzen keinen eigenen `min-h-screen`-/`max-w`-Container.
