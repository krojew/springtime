# Springtime Migrate Reginery

[![crates.io version](https://img.shields.io/crates/v/springtime-migrate-refinery.svg)](https://crates.io/crates/springtime-migrate-refinery)
![build status](https://github.com/krojew/springtime/actions/workflows/rust.yml/badge.svg)
![Maintenance](https://img.shields.io/maintenance/yes/2023)

`refinery` is powerful SQL migration toolkit for Rust, which makes creating
migrations easy. This crate integrates `refinery` with the broader [*Springtime
Framework*](https://crates.io/crates/springtime) allowing for providing database
clients and migrations via dependency injection, which further eases creating 
and applying migrations, either from files or Rust code.

## Features

* Automatic migration discovery
* File-based and code-based migrations
* Automatic migration application on startup for configured db clients
* All `refinery` db clients supported
