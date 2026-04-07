mod routes;
mod state;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wynn_api=debug,tower_http=debug".into()),
        )
        .init();

    tracing::info!("wynn-api starting");

    // TODO: load item DB, set up routes, start server
}
