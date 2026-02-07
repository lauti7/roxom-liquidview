# Integration Tests

This project includes integration tests that test against a real TimescaleDB database.

## Prerequisites

- PostgreSQL/TimescaleDB server running
- Database with appropriate privileges for creating/dropping databases

## Running Integration Tests

1. Set the `TEST_DATABASE_URL` environment variable:
   ```bash
   export TEST_DATABASE_URL=postgresql://username:password@localhost:5432/dbname
   ```

2. Run the integration tests:
   ```bash
   cargo test --test liquidity_integration_test
   ```

## Test Database

The integration tests will:
1. Connect to your PostgreSQL server
2. Drop and recreate a test database
3. Run migrations to set up TimescaleDB schema
4. Insert sample data for testing
5. Run the actual API tests
6. Clean up after completion

## Notes

- Tests require sufficient database privileges to create/drop databases
- The test database is automatically created and destroyed during test execution
- Make sure your PostgreSQL connection allows these operations