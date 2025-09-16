## [springtime-di] 1.0.4

## [springtime-web-axum] 3.0.1

* Fixed `RUSTSEC-2025-0057` by aschaeffer.

## [springtime-di] 1.0.3

* Fixed `RUSTSEC-2024-0388` by aschaeffer.

## [springtime-web-axum] 3.0.1

* Using `axum` 0.8.
* Updated dependencies to the latest versions.

## [springtime] 1.0.3

## [springtime-di] 1.0.2

## [springtime-migrate-refinery] 0.2.2

## [springtime-web-axum] 2.0.1

* Updated dependencies to the latest versions.

## [springtime] 1.0.1

## [springtime-di] 1.0.1

## [springtime-migrate-refinery] 0.2.1

## [springtime-web-axum] 2.0.0

* Updated dependencies to the latest versions.

## [springtime-web-axum] 1.0.0

### Changed

* Some internal improvements and dependency updates.

## [springtime] 1.0.0

### Changed

* Some internal improvements and dependency updates.

## [springtime-di] 1.0.0

### New

* Some errors now contain textual type names for easy debugging.

## [springtime] 0.3.0

### New

* Reading config from optional `springtime.json` file in current directory.
* Re-exporting `BoxFuture` from `springtime-di`.
* `ApplicationConfig` derives now `Deserialize`.

### Changed

* Moved `BoxFuture`, `FutureExt` re-exports to `future` mod.
* Runners with the same priority run concurrently with `async` feature.
* Removed `async-examples` feature.

## [springtime-di] 0.3.2

### Changed

* Removed `async-examples` feature.

## [springtime-di] 0.3.1

### Fixed

* Fixed passing down selected features.

## [springtime-di] 0.3.0

### New

* Fallible custom constructors - they should return `Result<Type, ErrorPtr>`,
  where `Type` is the type being returned previously. This also implies new
  entry
  in `ComponentInstanceProviderError` and loosing some of its derived traits.

## [springtime-di] 0.2.1

### Fixed

* Doc fixes.
