## [springtime-di] 0.3.2

### Changed

* Removed `async-examples` feature.

## [springtime-di] 0.3.1

### Fixed

* Fixed passing down selected features.

## [springtime-di] 0.3.0

### New

* Fallible custom constructors - they should return `Result<Type, ErrorPtr>`,
where `Type` is the type being returned previously. This also implies new entry
in `ComponentInstanceProviderError` and loosing some of its derived traits.

## [springtime-di] 0.2.1

### Fixed

* Doc fixes.
