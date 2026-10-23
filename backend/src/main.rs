//! OpenSyllabus server and administration CLI.

use std::{io::Write, net::SocketAddr};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use opensyllabus::{
    build_router,
    config::Config,
    models::user::Role,
    repositories::db,
    services::{accounts, background, seed},
    AppState,
};
use tokio::sync::watch;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "opensyllabus", about = "OpenSyllabus server and admin tools", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server (default).
    Serve,
    /// Apply database migrations and exit.
    Migrate,
    /// Create an administrator / instructor account.
    /// The password is read from OPENSYLLABUS_ADMIN_PASSWORD or prompted interactively.
    CreateAdmin {
        #[arg(long)]
        email: String,
        #[arg(long, default_value = "Instructor")]
        name: String,
    },
    /// Generate a one-time password reset link for any account.
    ResetLink {
        #[arg(long)]
        email: String,
    },
    /// Exit 0 when the local server answers /api/health/ready (used by the container health check).
    Healthcheck,
    /// Write a consistent online backup of the database to the given file.
    Backup {
        #[arg(long)]
        output: std::path::PathBuf,
    },
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn"));
    let json = std::env::var("LOG_FORMAT").map(|v| v == "json").unwrap_or(false);
    let registry = tracing_subscriber::registry().with(filter);
    if json {
        registry.with(fmt::layer().json()).init();
    } else {
        registry.with(fmt::layer()).init();
    }
}

fn read_password() -> Result<String> {
    if let Ok(p) = std::env::var("OPENSYLLABUS_ADMIN_PASSWORD") {
        return Ok(p);
    }
    print!("Password: ");
    std::io::stdout().flush()?;
    let p1 = rpassword::read_password()?;
    print!("Confirm password: ");
    std::io::stdout().flush()?;
    let p2 = rpassword::read_password()?;
    if p1 != p2 {
        bail!("passwords do not match");
    }
    Ok(p1)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    init_tracing();
    let cli = Cli::parse();
    let config = Config::from_env()?;
    if matches!(cli.command, Some(Command::Healthcheck)) {
        return healthcheck(config.bind_addr.port()).await;
    }
    let pool = db::connect(&config.database_path).await?;
    db::migrate(&pool).await?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Migrate => {
            println!("Migrations applied to {}", config.database_path.display());
            Ok(())
        }
        Command::CreateAdmin { email, name } => {
            let password = read_password()?;
            let user = accounts::create_user(&pool, &email, &name, Role::Admin, Some(password), &config.default_timezone)
                .await
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            accounts::adopt_orphan_courses(&pool).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Administrator {} created.", user.email);
            Ok(())
        }
        Command::ResetLink { email } => {
            let id: Option<String> =
                sqlx::query_scalar("SELECT id FROM users WHERE email = ?").bind(email.trim().to_lowercase()).fetch_optional(&pool).await?;
            let id = id.context("no account with that email")?;
            let (token, expires) = accounts::create_reset_token(&pool, &id, None, 24).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{}/reset-password?token={token}\n(valid until {expires})", config.public_url);
            Ok(())
        }
        Command::Backup { output } => {
            if output.exists() {
                bail!("{} already exists", output.display());
            }
            sqlx::query("VACUUM INTO ?").bind(output.to_string_lossy().to_string()).execute(&pool).await?;
            println!("Backup written to {}", output.display());
            Ok(())
        }
        Command::Serve => serve(config, pool).await,
        Command::Healthcheck => unreachable!(),
    }
}

async fn serve(config: Config, pool: sqlx::SqlitePool) -> Result<()> {
    std::fs::create_dir_all(&config.storage_dir).with_context(|| format!("creating {}", config.storage_dir.display()))?;
    accounts::bootstrap_admin_from_env(&pool, &config.default_timezone).await?;
    if config.seed_initial_course {
        seed::seed_initial_course(&pool).await?;
    }
    if accounts::admin_count(&pool).await.map_err(|e| anyhow::anyhow!("{e}"))? == 0 {
        tracing::warn!("no administrator exists yet; run `opensyllabus create-admin --email you@example.com`");
    }

    let addr = config.bind_addr;
    let state = AppState::new(pool.clone(), config).await?;
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let jobs = background::spawn(state.clone(), shutdown_rx);

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await.with_context(|| format!("binding {addr}"))?;
    tracing::info!(%addr, "OpenSyllabus listening");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).with_graceful_shutdown(shutdown_signal()).await?;

    let _ = shutdown_tx.send(true);
    let _ = jobs.await;
    pool.close().await;
    tracing::info!("shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}

async fn healthcheck(port: u16) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fut = async {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
        stream.write_all(b"GET /api/health/ready HTTP/1.0\r\nHost: localhost\r\n\r\n").await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        anyhow::Ok(String::from_utf8_lossy(&buf).starts_with("HTTP/1.0 200") || String::from_utf8_lossy(&buf).starts_with("HTTP/1.1 200"))
    };
    match tokio::time::timeout(std::time::Duration::from_secs(5), fut).await {
        Ok(Ok(true)) => Ok(()),
        _ => bail!("unhealthy"),
    }
}
