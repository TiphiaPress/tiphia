use std::net::SocketAddr;
use tiphia_core::{
    AppError, Config, build_router_with_plugins, connect_database, init_tracing,
    plugins::PluginRegistryBuilder,
};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();

    let config = Config::load()?;
    let _log_guard = init_tracing(&config.log)?;
    let db = connect_database(&config.database).await?;
    let app = build_router_with_plugins(db, config.clone(), register_compiled_plugins).await?;

    let addr: SocketAddr = config.http.bind.parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!(%addr, "tiphia server started");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn register_compiled_plugins(builder: &mut PluginRegistryBuilder) -> Result<(), AppError> {
    tiphia_plugin_audit::register(builder)?;
    tiphia_plugin_links::register(builder)?;
    tiphia_plugin_filing::register(builder)?;
    tiphia_plugin_geetest::register(builder)?;
    tiphia_plugin_highlight::register(builder)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
