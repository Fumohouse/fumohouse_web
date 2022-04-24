# Fumohouse Web

Web infrastructure for running Fumohouse.

## Setup

Requirements:

- Rust (nightly for development preferred, latest stable otherwise)
- PostgreSQL (dev with version 13.4)

The default `.env` file supports the following setup:

- PostgreSQL user `fumohouse` exists with password `fumohouse` and database `fumohouse`
- In `postgresql.conf`, `password_encryption` is set to `scram-sha-256`
- In `pg_hba.conf`, user `fumohouse` (or all users) use the `scram-sha-256` method to login when connecting to localhost

