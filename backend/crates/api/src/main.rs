//! Entry point of the `vermieterme` binary.
//!
//! Subcommands: `serve` (default) and `healthcheck`; `import-legacy` follows with P9.

use std::net::SocketAddr;

use anyhow::{Context as _, bail};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tracing_subscriber::EnvFilter;
use vermieterme_api::auth::{bootstrap, housekeeping};
use vermieterme_api::config::{Config, LogFormat};
use vermieterme_api::routes::{AppState, router};
use vermieterme_db::Db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        None | Some("serve") => serve().await,
        Some("healthcheck") => healthcheck().await,
        Some(_) => bail!("Unbekannter Befehl (erlaubt: serve, healthcheck)"),
    }
}

/// Reads the configuration, migrates the database, bootstraps the admin and serves
/// until SIGINT/SIGTERM.
async fn serve() -> anyhow::Result<()> {
    let config = Config::from_lookup(|name| std::env::var(name).ok())?;
    init_logging(&config);
    let db = Db::connect_and_migrate(&config.database_url)
        .await
        .context("Datenbank nicht erreichbar oder Migration fehlgeschlagen")?;
    match bootstrap::run(&db, &config).await? {
        bootstrap::Outcome::Created | bootstrap::Outcome::Promoted => {
            tracing::info!("Admin angelegt; ADMIN_PASSWORD kann jetzt entfernt werden");
        }
        bootstrap::Outcome::AdminExists if config.admin_password.is_some() => {
            tracing::warn!("ADMIN_PASSWORD ist gesetzt, wird aber nicht mehr verwendet");
        }
        _ => {}
    }
    let addr = SocketAddr::new(config.host, config.port);
    let state = AppState::new(db.clone(), config);
    housekeeping::spawn(state.clone());
    let listener = TcpListener::bind(addr).await.context("Port belegt")?;
    tracing::info!(%addr, "Server gestartet");
    let app = router(state).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    db.close().await;
    Ok(())
}

/// Logging per `LOG_LEVEL` / `LOG_FORMAT` (`RUST_LOG`-style directives allowed).
fn init_logging(config: &Config) {
    let filter = EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match config.log_format {
        LogFormat::Json => builder.json().init(),
        LogFormat::Text => builder.init(),
    }
}

/// Resolves on Ctrl-C or SIGTERM (container stop).
async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("Server wird beendet");
}

/// `GET /api/health` on the local port (Docker `HEALTHCHECK`); fails unless 200. Uses a
/// plain HTTP/1.0 request, so the binary needs no HTTP client library.
async fn healthcheck() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "3000".into());
    let port: u16 = port.parse().context("PORT ist ungültig")?;
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .context("Server nicht erreichbar")?;
    stream
        .write_all(b"GET /api/health HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .await?;
    let mut head = [0u8; 16];
    let n = stream.read(&mut head).await?;
    let status = std::str::from_utf8(&head[..n]).unwrap_or("");
    if status.starts_with("HTTP/1.0 200") || status.starts_with("HTTP/1.1 200") {
        Ok(())
    } else {
        bail!("Health-Check fehlgeschlagen")
    }
}
