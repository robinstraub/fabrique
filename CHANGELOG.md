# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/robinstraub/fabrique/compare/fabrique-v0.2.0...fabrique-v0.2.1) - 2026-02-09

### Added

- add named join support ([#113](https://github.com/robinstraub/fabrique/pull/113))
- auto-create belongs-to relations with factories ([#111](https://github.com/robinstraub/fabrique/pull/111))
- add insert entrypoint to model ([#110](https://github.com/robinstraub/fabrique/pull/110))
- add type-safe column selection support ([#89](https://github.com/robinstraub/fabrique/pull/89))
- add support for multiple backends ([#79](https://github.com/robinstraub/fabrique/pull/79))

### Other

- centralize changelog ([#116](https://github.com/robinstraub/fabrique/pull/116))
- allow Into types to be passed to reduce noise ([#109](https://github.com/robinstraub/fabrique/pull/109))
- clarify fabrique positionning ([#108](https://github.com/robinstraub/fabrique/pull/108))
- make select implicit on base model ([#90](https://github.com/robinstraub/fabrique/pull/90))
- add markdown codestyle check ([#83](https://github.com/robinstraub/fabrique/pull/83))

## [0.2.0](https://github.com/robinstraub/fabrique/compare/fabrique-v0.1.1...fabrique-v0.2.0) - 2026-01-04

### Added

- type-safe model selection after joins ([#76](https://github.com/robinstraub/fabrique/pull/76))
- add type-safe tracking of joined models ([#63](https://github.com/robinstraub/fabrique/pull/63))
- add direction-agnostic joins ([#62](https://github.com/robinstraub/fabrique/pull/62))
- add faker support for factories ([#61](https://github.com/robinstraub/fabrique/pull/61))
- add support for many-to-many lazy-loading functions ([#59](https://github.com/robinstraub/fabrique/pull/59))
- support multiple belongs-to relations to the same model ([#56](https://github.com/robinstraub/fabrique/pull/56))
- add support for many-to-many relations ([#50](https://github.com/robinstraub/fabrique/pull/50))
- add has-many relationship ([#45](https://github.com/robinstraub/fabrique/pull/45))
- [**breaking**] add database-agnostic Error type with TryFrom support ([#40](https://github.com/robinstraub/fabrique/pull/40))
- add save method ([#38](https://github.com/robinstraub/fabrique/pull/38))
- add a two-layer query builder with typestate pattern ([#35](https://github.com/robinstraub/fabrique/pull/35))
- [**breaking**] reorganize trait hierarchy from Persistable to Model ([#33](https://github.com/robinstraub/fabrique/pull/33))
- add a `Factory` trait ([#30](https://github.com/robinstraub/fabrique/pull/30))
- add soft deletes ([#26](https://github.com/robinstraub/fabrique/pull/26))

### Other

- derive root model from joins list ([#77](https://github.com/robinstraub/fabrique/pull/77))
- reorganize and document the 2-layer query builder ([#70](https://github.com/robinstraub/fabrique/pull/70))
- improve documentation coverage ([#60](https://github.com/robinstraub/fabrique/pull/60))
- add cargo-husky ([#53](https://github.com/robinstraub/fabrique/pull/53))
- sanitize demonstration sql schema ([#51](https://github.com/robinstraub/fabrique/pull/51))
- [**breaking**] rename belongs-to relationship ([#47](https://github.com/robinstraub/fabrique/pull/47))
- migrate user documentation to mdbooks following diataxis framework ([#43](https://github.com/robinstraub/fabrique/pull/43))
- add query builder documentation ([#29](https://github.com/robinstraub/fabrique/pull/29))
- add span to errors ([#27](https://github.com/robinstraub/fabrique/pull/27))
