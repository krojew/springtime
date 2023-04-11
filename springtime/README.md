# Springtime

[![crates.io version](https://img.shields.io/crates/v/springtime.svg)](https://crates.io/crates/springtime)
![build status](https://github.com/krojew/springtime/actions/workflows/rust.yml/badge.svg)
![Maintenance](https://img.shields.io/maintenance/yes/2023)

Application framework based on 
[springtime-di](https://crates.io/crates/springtime-di) dependency injection.
Inspired by the [Spring Framework](https://spring.io/) in Java, *Springtime*
provides a way to create advanced modular Rust applications by ensuring all
components of the application are properly decoupled from each other, and are
managed by the dependency injection system.

The core concept revolves around providing basic application services, e.g. 
logging, and running ordered `ApplicationRunner`s. An `ApplicationRunner`
represents root application service which starts the application logic. Examples
of runners are HTTP servers, messaging systems consumers, or even command line
applications.

## Features

* Automatic application logic discovery and running (based on DI)
* Runner priorities
* Async + sync support (runtime agnostic)
