//! Pure business logic of vermieterme without any IO.
//!
//! Cent amounts, date arithmetic, billing (sums, MEA shares, months), distribution
//! keys, the VPI parser with adjustment proposals and IBAN validation live here.
//! The crate must never depend on a database, the network or the file system.
//!
//! Money is always an integer number of cents ([`Cents`]); floats appear only at
//! the edges (legacy import, euro numbers for the tenant app), never in arithmetic.

pub mod billing;
pub mod date;
mod decimal;
pub mod money;
pub mod months;
mod rounding;
pub mod vpi;

pub use billing::DistributionKey;
pub use date::CalendarDate;
pub use money::Cents;
pub use months::{CalendarMonths, MonthRule};
pub use vpi::VpiValue;
