//! SQL migration framework based on [refinery](https://crates.io/crates/refinery) and
//! [Springtime](https://crates.io/crates/springtime).
//!
//! `refinery` is powerful SQL migration toolkit for Rust, which makes creating migrations easy.
//! This crate integrates `refinery` with the broader *Springtime Framework* allowing for providing
//! database clients and migrations via dependency injection, which further eases creating and
//! applying migrations, either from files or Rust code.
//!
//! ### Features
//!
//! * `refinery` async db features: `mysql_async`, `tiberius`, `tokio-postgres`
