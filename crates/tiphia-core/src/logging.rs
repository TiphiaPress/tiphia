use crate::{config::LogConfig, error::AppError};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing(config: &LogConfig) -> Result<WorkerGuard, AppError> {
    std::fs::create_dir_all(&config.directory)?;

    let file_appender = tracing_appender::rolling::daily(&config.directory, &config.file_prefix);
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_new(&config.level)
        .or_else(|_| EnvFilter::try_new("tiphia=info,tower_http=info"))?;

    if config.json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().compact().with_writer(std::io::stdout))
            .with(fmt::layer().json().with_writer(file_writer))
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().compact().with_writer(std::io::stdout))
            .with(fmt::layer().with_ansi(false).with_writer(file_writer))
            .try_init()?;
    }

    Ok(guard)
}
