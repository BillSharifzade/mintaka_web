//! `mintaka-server`: the mintaka.tj web server. See `server --help` and docs/CONTRACTS.md.

#![forbid(unsafe_code)]

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use server::{Cli, Config, Server};
use tracing_subscriber::filter::{EnvFilter, LevelFilter, Targets};
use tracing_subscriber::prelude::*;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Logs go through a background writer so a slow stdout never delays a response. Lines carry
    // method, matched route, status and duration — never addresses, user agents, queries or bodies.
    let (writer, _flush_on_exit) = tracing_appender::non_blocking(std::io::stdout());
    let env = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // The protocol libraries can print request contents (headers, paths) at debug/trace level, so
    // they stay at info whatever RUST_LOG says.
    let privacy = Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_target("h2", LevelFilter::INFO)
        .with_target("hyper", LevelFilter::INFO)
        .with_target("hyper_util", LevelFilter::INFO)
        .with_target("axum", LevelFilter::INFO)
        .with_target("reqwest", LevelFilter::INFO)
        .with_target("rustls", LevelFilter::INFO);
    tracing_subscriber::registry()
        .with(env)
        .with(privacy)
        .with(tracing_subscriber::fmt::layer().with_ansi(std::io::stdout().is_terminal()).with_writer(writer))
        .init();

    let config = match Config::from_cli(cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("mintaka-server: {e:#}");
            return ExitCode::from(2);
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("mintaka-server: cannot start the async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = runtime.block_on(async {
        let server = Server::bind(config).await?;
        server.run(shutdown_signal()).await
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e:#}");
            eprintln!("mintaka-server: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// SIGINT (Ctrl-C) or SIGTERM (systemd, Docker).
async fn shutdown_signal() {
    let interrupt = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("cannot listen for SIGINT: {e}");
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                tracing::error!("cannot listen for SIGTERM: {e}");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        () = interrupt => tracing::info!("SIGINT received"),
        () = terminate => tracing::info!("SIGTERM received"),
    }
}
