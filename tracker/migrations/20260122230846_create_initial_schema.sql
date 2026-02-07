-- Add migration script here

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE TABLE execution_events (
    id BIGSERIAL,
    symbol TEXT NOT NULL,
    order_value BIGINT NOT NULL,
    mid_price BIGINT NOT NULL,
    bps_over_mid_price NUMERIC NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
WITH (
   tsdb.hypertable,
   tsdb.partition_column='created_at',
   tsdb.chunk_interval = '1 day',
   tsdb.segmentby = 'symbol',
   tsdb.orderby = 'created_at DESC'
);

-- 5-minute aggregate
CREATE MATERIALIZED VIEW exec_cost_5min
WITH (timescaledb.continuous) AS
SELECT
  time_bucket(INTERVAL '5 minutes', created_at) AS bucket_ts,
  symbol,
  order_value,
  AVG(bps_over_mid_price) AS avg_bps
FROM execution_events
GROUP BY bucket_ts, symbol, order_value
WITH NO DATA;

-- 1-hour aggregate (hierarchical on 5min)
CREATE MATERIALIZED VIEW exec_cost_1hour
WITH (timescaledb.continuous) AS
SELECT
  time_bucket(INTERVAL '1 hour', bucket_ts) AS bucket_ts,
  symbol,
  order_value,
  AVG(avg_bps) AS avg_bps
FROM exec_cost_5min
GROUP BY time_bucket(INTERVAL '1 hour', bucket_ts), symbol, order_value
WITH NO DATA;

-- Refresh policies (refresh every 5min/1hour, exclude recent data from refreshes)
SELECT add_continuous_aggregate_policy('exec_cost_5min',
  start_offset => INTERVAL '1 day',
  end_offset => INTERVAL '5 minutes',
  schedule_interval => INTERVAL '5 minutes');

SELECT add_continuous_aggregate_policy('exec_cost_1hour',
  start_offset => INTERVAL '7 days',
  end_offset => INTERVAL '1 hour',
  schedule_interval => INTERVAL '1 hour');

-- -- Compression policies (compress old data to save space, keep recent uncompressed for real-time)
-- -- Raw data: compress after 30 days
-- SELECT add_compression_policy('execution_events', INTERVAL '30 days');

-- -- Aggregates: compress after 90 days (recent stays uncompressed for real-time queries)
-- SELECT add_compression_policy('exec_cost_5min', INTERVAL '90 days');
-- SELECT add_compression_policy('exec_cost_1hour', INTERVAL '90 days');
