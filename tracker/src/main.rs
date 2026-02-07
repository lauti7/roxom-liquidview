use roxom_exec_cost::{
    TRACKED_SYMBOLS,
    http::create_http_server,
    instrument_worker::InstrumentWorker,
    pg::{DBWriterWorker, DatabaseOptions, migrate_up},
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

#[tokio::main]
async fn main() {
    init_tracing();

    let db_connection_string = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| String::from("postgresql://postgres:root@localhost:5433/exec_costs"));

    let db = DatabaseOptions::new()
        .with_connection_string(&db_connection_string)
        .min_connections(5)
        .max_connections(10)
        .connect()
        .await
        .expect("must be connected ");

    migrate_up(db.pool()).await.expect("failed to migrate up");

    let task_tracker = TaskTracker::new();
    let cancelation_token = CancellationToken::new();
    for symbol in TRACKED_SYMBOLS {
        let db_writer = DBWriterWorker::new(db.clone());
        let worker = InstrumentWorker::new(symbol, db_writer, cancelation_token.child_token());
        task_tracker.spawn(async move { worker.run().await });
    }

    let server_addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| String::from("0.0.0.0:8080"));
    let server = create_http_server(&server_addr, &db);
    let server_handle = server.handle();
    task_tracker.spawn(async move {
        if let Err(err) = server.await {
            tracing::error!("Actix server stopped with error: {}", err);
        }
    });

    task_tracker.close();
    let _ = tokio::signal::ctrl_c().await;
    cancelation_token.cancel();
    server_handle.stop(true).await;
    task_tracker.wait().await;
}

fn init_tracing() {
    use tracing_subscriber::util::SubscriberInitExt;

    let env_filter = tracing_subscriber::EnvFilter::new("DEBUG");
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false) // Include the target (module path) in logs
        .with_thread_ids(false) // Include thread IDs for concurrent diagnostics
        .with_file(false) // Include source file information
        .with_line_number(false) // Include line numbers
        .finish();

    subscriber
        .try_init()
        .expect("Failed to set global default subscriber");
}
