//! The reference of all environment variables (name, default, meaning).

/// One environment variable of the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvVar {
    /// Variable name.
    pub name: &'static str,
    /// Default value; `None` = no default (required or optional without value).
    pub default: Option<&'static str>,
    /// Meaning (German, for the operations reference).
    pub description: &'static str,
}

/// All environment variables of the server (plan, interface list). `TEST_MYSQL_URL` is
/// test-only and lives in `vermieterme_db::testing`.
pub const VARIABLES: &[EnvVar] = &[
    var(
        "DATABASE_URL",
        Some("sqlite://data/vermieterme-v2.db?mode=rwc"),
        "SQLite oder `mysql://…` (MariaDB/MySQL)",
    ),
    var(
        "LEGACY_DATABASE_PATH",
        Some("data/vermieterme.db"),
        "Prisma-Datei für `import-legacy` und Auto-Import",
    ),
    var(
        "AUTH_SECRET",
        None,
        "Pflicht, mindestens 32 Zeichen: Mieter-JWT (HS256) und HKDF-Basis für Einladungscodes",
    ),
    var(
        "ADMIN_EMAIL",
        None,
        "nur Bootstrap: Admin anlegen, wenn noch keiner existiert",
    ),
    var(
        "ADMIN_PASSWORD",
        None,
        "nur Bootstrap: Passwort des ersten Admins (danach entfernen)",
    ),
    var("HOST", Some("0.0.0.0"), "Bind-Adresse"),
    var("PORT", Some("3000"), "Port"),
    var(
        "STATIC_DIR",
        Some("web"),
        "statischer Export der Oberfläche",
    ),
    var(
        "UPLOAD_DIR",
        Some("data/uploads"),
        "Dokumente, Unterordner je Kunde",
    ),
    var(
        "UPLOAD_QUOTA_BYTES",
        Some("1073741824"),
        "Speicher je Kunde für Dokumente in Bytes; darüber 413 `quota_exceeded`",
    ),
    var(
        "REQUEST_TIMEOUT_SECS",
        Some("30"),
        "Zeitlimit je Anfrage in Sekunden (Upload fest 300 s); darüber 503 `timeout`",
    ),
    var("COOKIE_SECURE", Some("true"), "`false` nur lokal ohne TLS"),
    var(
        "SESSION_IDLE_HOURS",
        Some("168"),
        "Sitzung endet nach so vielen Stunden ohne Anfrage",
    ),
    var(
        "SESSION_MAX_DAYS",
        Some("30"),
        "Höchstdauer einer Sitzung in Tagen",
    ),
    var(
        "INVITE_CODE_TTL_DAYS",
        Some("30"),
        "Ablauf eines Einladungscodes der Mieter-App in Tagen",
    ),
    var(
        "TRUSTED_PROXIES",
        Some(""),
        "CIDR-Liste (Komma); nur von dort wird `X-Forwarded-For` übernommen",
    ),
    var(
        "PDF_RENDERER_CMD",
        Some("node pdf-renderer/render.mjs"),
        "PDF-Brücke",
    ),
    var(
        "PDF_RENDER_TIMEOUT_SECS",
        Some("20"),
        "Zeitlimit je PDF in Sekunden",
    ),
    var(
        "PDF_RENDER_CONCURRENCY",
        Some("2"),
        "gleichzeitige PDF-Erzeugungen",
    ),
    var(
        "PASSWORD_HASH_CONCURRENCY",
        Some("4"),
        concat!(
            "gleichzeitige Passwort-Berechnungen (argon2id, je ca. 19 MiB); ",
            "weitere Anfragen warten bis 5 s, dann 429"
        ),
    ),
    var(
        "VPI_API_BASE",
        Some("https://api.statistiken.bundesbank.de/rest/data"),
        "Basis-URL der Bundesbank-SDMX-Schnittstelle (VPI-Abruf)",
    ),
    var(
        "LOG_LEVEL",
        Some("info"),
        "Log-Stufe (`error`, `warn`, `info`, `debug`, `trace`)",
    ),
    var("LOG_FORMAT", Some("json"), "`json` oder `text`"),
];

/// Builds a table entry (keeps [`VARIABLES`] on one line per variable).
const fn var(
    name: &'static str,
    default: Option<&'static str>,
    description: &'static str,
) -> EnvVar {
    EnvVar {
        name,
        default,
        description,
    }
}
