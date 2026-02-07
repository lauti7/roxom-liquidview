use crate::instrument_worker::ExecutionCostEvent;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{ConnectOptions, Connection, Postgres, Transaction};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    FailedToConnect(#[from] sqlx::Error),
    #[error("Failed to query database: {0}")]
    FailedToCheckExistingDatabae(sqlx::Error),
    #[error("Failed while initing migrator: {0}")]
    InitMigrationsError(sqlx::migrate::MigrateError),
    #[error("Failed to run migrations: {0}")]
    MigrationError(sqlx::migrate::MigrateError),
    #[error("Failed to get connection: {0}")]
    FailedToGetConnection(sqlx::Error),
    #[error("Connection string is empty")]
    EmptyConnectionString,
}

pub async fn create_database_if_not_exists(
    pool: &PgPool,
    db_name: &str,
) -> Result<(), DatabaseError> {
    // Check if database exists
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(db_name)
            .fetch_one(pool)
            .await?;

    if !exists {
        warn!("Database '{}' does not exist, creating it...", db_name);

        // Create database
        sqlx::query(&format!("CREATE DATABASE {db_name}"))
            .execute(pool)
            .await?;

        info!("Database '{}' created successfully", db_name);
    } else {
        info!("Database '{}' already exists", db_name);
    }

    Ok(())
}

pub async fn drop_database_if_exists(pool: &PgPool, db_name: &str) -> Result<(), DatabaseError> {
    // Check if database exists
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(db_name)
            .fetch_one(pool)
            .await?;

    if exists {
        info!("Database '{}' exists, dropping it...", db_name);

        // First ensure no active connections to the database
        let disconnect_query = format!(
            "SELECT pg_terminate_backend(pg_stat_activity.pid)
             FROM pg_stat_activity
             WHERE pg_stat_activity.datname = '{db_name}'
             AND pid <> pg_backend_pid()"
        );

        // Execute the disconnect query
        sqlx::query(&disconnect_query).execute(pool).await?;

        // Drop database
        sqlx::query(&format!("DROP DATABASE {db_name}"))
            .execute(pool)
            .await?;

        info!("Database '{}' dropped successfully", db_name);
    } else {
        info!("Database '{}' does not exist, nothing to drop", db_name);
    }

    Ok(())
}

#[derive(Default)]
pub enum DatabaseMigrationType {
    #[default]
    Up,
    Down,
}

#[derive(Default)]
pub struct DatabaseOptions {
    connection_string: String,
    pool_opts: PgPoolOptions,
    enable_log_statements: bool,
    enable_log_slow_statements: bool,
    enable_log_slow_statements_duration: Option<Duration>,
    run_migrations: bool,
    run_migrations_type: DatabaseMigrationType,
    migrations_path: String,
}

impl DatabaseOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_connection_string(mut self, connection_string: &str) -> Self {
        self.connection_string = connection_string.to_string();
        self
    }

    pub fn min_connections(mut self, min_connections: u32) -> Self {
        self.pool_opts = self.pool_opts.min_connections(min_connections);
        self
    }

    pub fn max_connections(mut self, max_connections: u32) -> Self {
        self.pool_opts = self.pool_opts.max_connections(max_connections);
        self
    }

    pub fn with_enabled_log_statements(mut self) -> Self {
        self.enable_log_statements = true;
        self
    }

    pub fn with_disabled_log_statements(mut self) -> Self {
        self.enable_log_statements = false;
        self
    }

    pub fn with_acquire_time_level(mut self, log: log::LevelFilter) -> Self {
        self.pool_opts = self.pool_opts.acquire_time_level(log);
        self
    }

    /// Log statements that takes more than the given duration to complete
    pub fn with_enabled_log_slow_statements(mut self, time: Duration) -> Self {
        self.enable_log_slow_statements = true;
        self.enable_log_slow_statements_duration = Some(time);
        self
    }

    pub fn with_run_migrations(mut self, m_type: DatabaseMigrationType, path: &str) -> Self {
        self.run_migrations = true;
        self.run_migrations_type = m_type;
        self.migrations_path = path.to_string();
        self
    }

    pub async fn connect(self) -> Result<Database, DatabaseError> {
        if self.connection_string.is_empty() {
            return Err(DatabaseError::EmptyConnectionString);
        }

        let mut conn_opts = PgConnectOptions::from_str(&self.connection_string)
            .expect("Invalid connection string for Database");

        if self.enable_log_statements {
            conn_opts = conn_opts.log_statements(log::LevelFilter::Debug);
        } else {
            conn_opts = conn_opts.log_statements(log::LevelFilter::Off)
        }

        if self.enable_log_slow_statements {
            conn_opts = conn_opts.log_slow_statements(
                log::LevelFilter::Warn,
                self.enable_log_slow_statements_duration
                    .expect("duration must be set"),
            )
        }

        let pool = self
            .pool_opts
            .connect_with(conn_opts)
            .await
            .map_err(DatabaseError::FailedToConnect)?;

        if self.run_migrations {
            let migrator = sqlx::migrate::Migrator::new(Path::new(&self.migrations_path))
                .await
                .map_err(DatabaseError::InitMigrationsError)?;

            match self.run_migrations_type {
                DatabaseMigrationType::Up => {
                    migrator
                        .run(&pool)
                        .await
                        .map_err(DatabaseError::MigrationError)?;
                }
                DatabaseMigrationType::Down => {
                    migrator
                        .undo(&pool, 0)
                        .await
                        .map_err(DatabaseError::MigrationError)?;
                }
            }
        }

        Ok(Database::new(pool))
    }
}

#[derive(Clone, Debug)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn get_connection(&self) -> Result<DatabaseConnection, DatabaseError> {
        let conn = self
            .pool
            .acquire()
            .await
            .map_err(DatabaseError::FailedToGetConnection)?;

        Ok(DatabaseConnection::new(conn))
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub async fn migrate_up(&self, path: &str) -> Result<(), DatabaseError> {
        let migrator = sqlx::migrate::Migrator::new(Path::new(path))
            .await
            .map_err(DatabaseError::InitMigrationsError)?;

        migrator
            .run(&self.pool)
            .await
            .map_err(DatabaseError::MigrationError)?;

        Ok(())
    }

    pub async fn migrate_down(&self, path: &str, undo_num: i64) -> Result<(), DatabaseError> {
        let migrator = sqlx::migrate::Migrator::new(Path::new(path))
            .await
            .map_err(DatabaseError::InitMigrationsError)?;

        migrator
            .undo(&self.pool, undo_num)
            .await
            .map_err(DatabaseError::MigrationError)?;

        Ok(())
    }
}

pub struct DatabaseConnection {
    conn: PoolConnection<Postgres>,
}

#[derive(Debug, Error)]
pub enum DBConnectionError {
    #[error("Failed to begin transaction: {0}")]
    FailedToBeginTransaction(sqlx::Error),
}

impl DatabaseConnection {
    fn new(conn: PoolConnection<Postgres>) -> Self {
        Self { conn }
    }

    pub fn into_inner(self) -> PoolConnection<Postgres> {
        self.conn
    }

    pub fn conn(&self) -> &PoolConnection<Postgres> {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut PoolConnection<Postgres> {
        &mut self.conn
    }

    pub async fn tx(&mut self) -> Result<DatabaseTransaction<'_>, DBConnectionError> {
        Ok(DatabaseTransaction::new(
            self.conn
                .begin()
                .await
                .map_err(DBConnectionError::FailedToBeginTransaction)?,
        ))
    }
}

pub struct DatabaseTransaction<'a> {
    tx: Transaction<'a, Postgres>,
}

#[derive(Debug, Error)]
pub enum DBTransactionError {
    #[error("Failed to commit transaction: {0}")]
    FailedToCommitTransaction(sqlx::Error),
    #[error("Failed to rollback transaction: {0}")]
    FailedToRollbackTransaction(sqlx::Error),
}

impl<'a> DatabaseTransaction<'a> {
    fn new(tx: Transaction<'a, Postgres>) -> Self {
        Self { tx }
    }

    pub fn tx(&mut self) -> &mut Transaction<'a, Postgres> {
        &mut self.tx
    }

    pub async fn commit(self) -> Result<(), DBTransactionError> {
        self.tx
            .commit()
            .await
            .map_err(DBTransactionError::FailedToCommitTransaction)
    }

    pub async fn rollback(self) -> Result<(), DBTransactionError> {
        self.tx
            .rollback()
            .await
            .map_err(DBTransactionError::FailedToRollbackTransaction)
    }

    pub fn into_inner(self) -> Transaction<'a, Postgres> {
        self.tx
    }
}

pub async fn migrate_up(pgpool: &PgPool) -> Result<(), DatabaseError> {
    info!("Running migrations ups...");
    let migrator = sqlx::migrate::Migrator::new(Path::new("./migrations"))
        .await
        .map_err(DatabaseError::InitMigrationsError)?;
    migrator
        .run(pgpool)
        .await
        .map_err(DatabaseError::MigrationError)?;
    info!("Migrations completed successfully");
    Ok(())
}

pub async fn migrate_down(pgpool: &PgPool) -> Result<(), DatabaseError> {
    info!("Running migrations down...");
    let migrator = sqlx::migrate::Migrator::new(Path::new("./migrations"))
        .await
        .map_err(DatabaseError::InitMigrationsError)?;
    migrator
        .undo(pgpool, 0)
        .await
        .map_err(DatabaseError::MigrationError)?;
    info!("Migrations reverted successfully");
    Ok(())
}

#[derive(Debug, Error)]
pub enum DBWriterWorkerError {
    #[error("DB Writer is dead to receive work")]
    DBWorkerDead,
}

#[derive(Debug)]
pub struct DBWriterWorker {
    tx: UnboundedSender<(String, Vec<ExecutionCostEvent>)>,
    _inner_worker_handle: JoinHandle<()>,
}

impl DBWriterWorker {
    pub fn new(db: Database) -> Self {
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<(String, Vec<ExecutionCostEvent>)>();
        let db_clone = db.clone();
        let _inner_worker_handle = tokio::spawn(async move {
            let mut w = InnerDBWorker::new(db_clone.clone()).await;

            while let Some((symbol, events)) = rx.recv().await {
                w.events_buff.extend(events);
                if let Err(e) = w.insert_in_bulk(&symbol).await {
                    tracing::error!("Failed to insert execution events: {}", e);
                }
            }
        });

        Self {
            tx,
            _inner_worker_handle,
        }
    }

    pub fn send_events(
        &self,
        symbol: &str,
        events: Vec<ExecutionCostEvent>,
    ) -> Result<(), DBWriterWorkerError> {
        self.tx
            .send((symbol.to_string(), events))
            .map_err(|_| DBWriterWorkerError::DBWorkerDead)?;
        Ok(())
    }
}

struct InnerDBWorker {
    events_buff: Vec<ExecutionCostEvent>,
    db: Database,
}

impl InnerDBWorker {
    async fn new(db: Database) -> Self {
        Self {
            events_buff: Vec::with_capacity(100),
            db,
        }
    }

    async fn insert_in_bulk(&mut self, symbol: &str) -> Result<(), DatabaseError> {
        let order_values: Vec<i64> = self
            .events_buff
            .iter()
            .map(|e| e.order_value.value())
            .collect();
        let mid_prices: Vec<i64> = self
            .events_buff
            .iter()
            .map(|e| e.mid_price.value())
            .collect();
        let bps_over_mid_prices: Vec<f64> = self
            .events_buff
            .iter()
            .map(|e| e.bps_over_mid_price)
            .collect();
        let timestamps: Vec<chrono::DateTime<chrono::Utc>> =
            self.events_buff.iter().map(|e| e.timestamp).collect();

        sqlx::query(
                "INSERT INTO execution_events (symbol, order_value, mid_price, bps_over_mid_price, created_at)
                 SELECT $1, UNNEST($2::BIGINT[]), UNNEST($3::BIGINT[]), UNNEST($4::NUMERIC[]), UNNEST($5::TIMESTAMP[])"
            )
            .bind(symbol)
            .bind(&order_values)
            .bind(&mid_prices)
            .bind(&bps_over_mid_prices)
            .bind(&timestamps)
            .execute(&self.db.pool)
            .await?;

        self.events_buff.clear();

        Ok(())
    }
}
