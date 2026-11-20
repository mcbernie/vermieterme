//! Pure business logic of vermieterme without any IO.
//!
//! Cent amounts, date arithmetic, billing (sums, MEA shares, months), distribution
//! keys, the VPI parser with adjustment proposals and IBAN validation live here.
//! The crate must never depend on a database, the network or the file system.
