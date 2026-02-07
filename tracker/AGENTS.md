# Agent Guidelines for roxom-exec-cost

This document provides guidelines for coding agents working on the roxom-exec-cost project, a Rust-based cryptocurrency execution cost monitoring system.

## Build, Test, and Lint Commands

### Building
- `cargo build` - Compile the project in debug mode
- `cargo build --release` - Compile with optimizations for production
- `cargo check` - Verify compilation without generating binaries

### Testing
- `cargo test` - Run all unit and integration tests
- `cargo test <test_name>` - Run a specific test (e.g., `cargo test test_try_exec_mini_order`)
- `cargo test --lib` - Run only library tests, excluding integration tests
- `cargo test --doc` - Run documentation tests
- `cargo test -- --nocapture` - Show print output from tests (useful for debugging)

### Linting and Formatting
- `cargo clippy` - Run Clippy linter to catch common mistakes and style issues
- `cargo clippy -- -D warnings` - Treat warnings as errors
- `cargo fmt` - Format code according to Rust style guidelines
- `cargo fmt --check` - Check if code is properly formatted without making changes

### Documentation
- `cargo doc` - Generate HTML documentation for the project
- `cargo doc --open` - Generate and open documentation in browser

### Cleaning
- `cargo clean` - Remove build artifacts

## Code Style Guidelines

### Import Organization
Group imports in this order, separated by blank lines:
1. Standard library imports
2. External crate imports
3. Local module imports

```rust
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::instrument_worker::ExecutionCostEvent;
```

### Naming Conventions
- **Functions and variables**: `snake_case`
- **Structs, enums, traits**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`
- **Type aliases**: `PascalCase`

```rust
pub const TRACKED_SYMBOLS: [&str; 1] = ["GOLD-BTC"];
pub const ORDERBOOK_FETCH_INTERVAL_SECS: u64 = 30;

pub type InstrumentWorkerSender = UnboundedSender<Orderbook>;

pub struct ExecutionCostEvent {
    pub order_value: BtcAmount,
    pub bps_over_mid_price: f64,
    pub mid_price: BtcPrice,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

fn try_orderbook_executions(symbol: &str, mut orderbook: Orderbook) -> Option<Vec<ExecutionCostEvent>>
```

### Error Handling
- Use `thiserror::Error` for custom error types
- Return `Result<T, E>` for fallible operations
- Use `?` operator for error propagation
- Use `expect()` only for critical failures that should panic
- Provide descriptive error messages

```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    FailedToConnect(#[from] sqlx::Error),
    #[error("Connection string is empty")]
    EmptyConnectionString,
}

pub async fn connect(self) -> Result<Database, DatabaseError> {
    if self.connection_string.is_empty() {
        return Err(DatabaseError::EmptyConnectionString);
    }
    // ... rest of function
}
```

### Type Annotations and Derives
- Use explicit type annotations for public APIs
- Derive `Debug` for all data structures
- Use `Default` for configuration structs
- Prefer strongly typed wrappers over primitives where possible

```rust
#[derive(Debug)]
pub struct ExecutionCostEvent {
    pub order_value: BtcAmount,
    pub bps_over_mid_price: f64,
    pub mid_price: BtcPrice,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
pub struct DatabaseOptions {
    connection_string: String,
    pool_opts: PgPoolOptions,
    // ... other fields
}
```

### Async/Await Patterns
- Use `#[tokio::main]` for binary entry points
- Prefer async functions over blocking operations
- Use `tokio::select!` for concurrent operations
- Use `CancellationToken` for graceful shutdown

```rust
#[tokio::main]
async fn main() {
    // ... initialization
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!("Worker cancelled");
                break;
            }
            msg = receiver.recv() => {
                // handle message
            }
        }
    }
}
```

### Testing Patterns
- Place tests in `#[cfg(test)] mod tests` blocks at file bottom
- Use descriptive test names: `test_<functionality>_<scenario>`
- Test both success and error cases
- Use realistic test data

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_exec_mini_order() {
        // Arrange
        let orderbook = Orderbook::from_snapshot(/* ... */);
        let order = MiniMarketOrder {
            base_amount: InstrumentAmount::new(15, 2),
            side: OrderSide::Buy,
            limit_price: BtcPrice::new(4865500),
        };

        // Act
        let execs = orderbook.try_exec(order);
        let exec = execs.expect("must have executions");

        // Assert
        assert_eq!(exec.executed_base_amount.to_decimal(), 0.15);
        assert_eq!(exec.executed_quote_amount.to_sats_round_down().value(), 729_825);
    }
}
```

### Builder Pattern
Use the builder pattern for complex configuration structs:

```rust
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

    // ... more builder methods
}
```

### Logging
- Use `tracing` crate for structured logging
- Use appropriate log levels: `info!`, `warn!`, `error!`
- Include relevant context in log messages
- Use `tracing::instrument` for function tracing when needed

```rust
use tracing::{info, warn};

info!("Database '{}' created successfully", db_name);
warn!("Database '{}' does not exist, creating it...", db_name);
```

### Memory and Ownership
- Prefer borrowing over cloning where possible
- Use `Arc` for shared ownership in async contexts
- Use `RwLock` or `Mutex` for interior mutability when needed
- Be mindful of async bounds and `Send`/`Sync` requirements

### Project-Specific Patterns
- Database operations return `Result<T, DatabaseError>`
- API responses use `RxmApiResponse<T>` wrapper
- Workers use `CancellationToken` for graceful shutdown
- Configuration constants are defined in `lib.rs`
- Use `chrono::DateTime<chrono::Utc>` for timestamps
- Use `BtcAmount`, `BtcPrice`, `InstrumentAmount` for type safety

### Security Considerations
- Never log sensitive information (API keys, passwords)
- Use environment variables for configuration
- Validate input data before processing
- Use HTTPS for external API calls
- Implement proper error handling to prevent information leakage</content>
<parameter name="filePath">/Users/lautarobustos/projects/roxom/roxom-exec-cost/AGENTS.md