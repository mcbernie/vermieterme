//! Repositories: one module per area. Functions take `&mut AnyConnection` (from
//! `pool.acquire()` or a transaction) and, where data has an account reference, a scope
//! from [`crate::scope`].
//!
//! Business repositories (master data, billing, documents, tenant app) are added by later
//! packages as `masterdata*`, `billing*`, `documents*`, `tenant_app*`, `backup*`.

pub mod account;
pub mod audit;
// P4
pub mod masterdata;
pub mod session;
pub mod user;
pub mod user_token;
