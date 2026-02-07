use actix_web::{App, test, web};
use chrono::{DateTime, Utc};
use roxom_exec_cost::instrument_worker::ExecutionCostEvent;
use roxom_exec_cost::prices::{BtcAmount, BtcPrice};
use roxom_exec_cost::{api, pg};
use serde_json::Value;
use std::env;

async fn setup_test_database() -> pg::Database {
    let test_db_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("WARNING: TEST_DATABASE_URL not set, using default connection string");
        eprintln!("Set TEST_DATABASE_URL environment variable to run integration tests");
        eprintln!(
            "Example: export TEST_DATABASE_URL=postgresql://user:password@localhost:5432/roxom_test"
        );
        panic!("TEST_DATABASE_URL environment variable must be set for integration tests");
    });

    // Connect to default postgres database to create test database
    let admin_conn_str = test_db_url.replace("/roxom_test", "/postgres");
    let admin_db_opts = pg::DatabaseOptions::new()
        .with_connection_string(&admin_conn_str)
        .max_connections(1);

    let admin_db = admin_db_opts
        .connect()
        .await
        .expect("Failed to connect to admin database");

    // Drop and recreate test database
    let pool = admin_db.pool();
    pg::drop_database_if_exists(pool, "roxom_test")
        .await
        .expect("Failed to drop test database");
    pg::create_database_if_not_exists(pool, "roxom_test")
        .await
        .expect("Failed to create test database");

    // Close admin connection
    admin_db.close().await;

    // Connect to test database and run migrations
    let test_db_opts = pg::DatabaseOptions::new()
        .with_connection_string(&test_db_url)
        .max_connections(5)
        .with_run_migrations(pg::DatabaseMigrationType::Up, "./migrations");

    let test_db = test_db_opts
        .connect()
        .await
        .expect("Failed to connect to test database");

    // Insert test data
    insert_test_data(&test_db).await;

    test_db
}

async fn insert_test_data(db: &pg::Database) {
    let base_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    // Insert test execution events
    let mut events = Vec::new();

    // Create test data for different order values and times
    for hour_offset in 0..24 {
        // 24 hours of data
        for order_value in roxom_exec_cost::ORDERS_VALUES_IN_SATS.iter().take(3) {
            // Use first 3 order values
            for event_idx in 0..10 {
                // 10 events per hour per order value
                let timestamp = base_time
                    + chrono::Duration::hours(hour_offset)
                    + chrono::Duration::minutes(event_idx * 6);
                let bps = 10.0 + (hour_offset as f64 * 0.5) + (event_idx as f64 * 0.1);

                events.push(ExecutionCostEvent {
                    order_value: BtcAmount::new(*order_value),
                    bps_over_mid_price: bps,
                    mid_price: BtcPrice::new(4850000),
                    timestamp,
                });
            }
        }
    }

    // Insert events in batches
    let pool = db.pool();
    let symbol = "GOLD-BTC";

    let order_values: Vec<i64> = events.iter().map(|e| e.order_value.value()).collect();
    let mid_prices: Vec<i64> = events.iter().map(|e| e.mid_price.value()).collect();
    let bps_values: Vec<f64> = events.iter().map(|e| e.bps_over_mid_price).collect();
    let timestamps: Vec<DateTime<Utc>> = events.iter().map(|e| e.timestamp).collect();

    sqlx::query(
        "INSERT INTO execution_events (symbol, order_value, mid_price, bps_over_mid_price, created_at)
         SELECT $1, UNNEST($2::BIGINT[]), UNNEST($3::BIGINT[]), UNNEST($4::NUMERIC[]), UNNEST($5::TIMESTAMP[])"
    )
    .bind(symbol)
    .bind(&order_values)
    .bind(&mid_prices)
    .bind(&bps_values)
    .bind(&timestamps)
    .execute(pool)
    .await
    .expect("Failed to insert test data");

    // Refresh materialized views to ensure data is available for queries
    sqlx::query("CALL refresh_continuous_aggregate('exec_cost_5min', NULL, NULL)")
        .execute(pool)
        .await
        .expect("Failed to refresh 5min aggregate");

    sqlx::query("CALL refresh_continuous_aggregate('exec_cost_1hour', NULL, NULL)")
        .execute(pool)
        .await
        .expect("Failed to refresh 1hour aggregate");
}

#[actix_web::test]
async fn test_liquidity_endpoint_basic_success() {
    // Setup test database
    let test_db = setup_test_database().await;

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(
                roxom_exec_cost::liquidity_service::LiquidityService::new(test_db.clone()),
            ))
            .configure(api::configure),
    )
    .await;

    // Create test request
    let from_timestamp = 1704067200; // 2024-01-01 00:00:00 UTC
    let to_timestamp = 1704153600; // 2024-01-02 00:00:00 UTC
    let order_value = roxom_exec_cost::ORDERS_VALUES_IN_SATS[0]; // First order value

    let req = test::TestRequest::get()
        .uri(&format!(
            "/liquidity?from={}&to={}&symbol=GOLD-BTC&amount={}",
            from_timestamp, to_timestamp, order_value
        ))
        .to_request();

    // Make request
    let resp = test::call_service(&app, req).await;

    // Assert response status
    assert!(resp.status().is_success());

    // Parse response body
    let body: Value = test::read_body_json(resp).await;

    // Verify response structure
    assert!(body.get("data").is_some());

    let data = body["data"].as_array().expect("Data should be an array");

    // Should have data points for the time range
    assert!(!data.is_empty(), "Should return liquidity data");

    // Verify structure of each data point
    for point in data {
        assert!(
            point.get("bucketTsUnix").is_some(),
            "Should have bucketTsUnix field"
        );
        assert!(point.get("avgBps").is_some(), "Should have avgBps field");

        let bucket_ts = point["bucketTsUnix"]
            .as_i64()
            .expect("bucketTsUnix should be a number");
        let avg_bps = point["avgBps"].as_f64().expect("avgBps should be a number");

        // Verify timestamp is within expected range
        assert!(
            bucket_ts >= from_timestamp,
            "Timestamp should be within range"
        );
        assert!(
            bucket_ts <= to_timestamp,
            "Timestamp should be within range"
        );

        // Verify BPS is a reasonable value
        assert!(avg_bps > 0.0, "Average BPS should be positive");
        assert!(avg_bps < 1000.0, "Average BPS should be reasonable");
    }

    // Clean up
    test_db.close().await;
}
