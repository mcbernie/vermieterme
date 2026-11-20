//! Data access of vermieterme: connection pool, SQL dialect, migrations and
//! repositories.
//!
//! Every repository function with account (customer) reference takes an explicit
//! scope parameter, so access to foreign data is structurally impossible.
