# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/forward-market-design/flow-trading-service/compare/fts-core-v0.1.5...fts-core-v0.2.0) - 2025-04-28

### Added

- define core config type, update interfaces
- attach demand curves directly to auths

### Other

- unrelated - replace fxhash with rustc_hash
- rename crate
- unrelated change to fix comment

## [0.1.5](https://github.com/forward-market-design/flow-trading-service/compare/fts-core-v0.1.4...fts-core-v0.1.5) - 2025-04-21

### Other

- updated the following local packages: fts-solver

## [0.1.4](https://github.com/forward-market-design/flow-trading-service/compare/fts-core-v0.1.3...fts-core-v0.1.4) - 2025-04-15

### Fixed

- release-plz needs explicit version for binary dep
- simplify awkward iterator arguments
- update other crates to new solver API

## [0.1.3](https://github.com/forward-market-design/flow-trading-service/compare/fts-core-v0.1.2...fts-core-v0.1.3) - 2025-04-11

### Added

- *(api)* replace per-auth outcome stream with per-submission outcome stream

### Fixed

- promote portfolio and group to proper newtype
- specify param location in query
- ensure all schemas are exported
- rearrange product types

### Other

- update READMEs with workspace & crate information

## [0.1.2](https://github.com/forward-market-design/flow-trading-service/compare/fts-core-v0.1.1...fts-core-v0.1.2) - 2025-03-21

### Other

- explicit versions for all crates, no workspace-version

## [0.1.1](https://github.com/forward-market-design/flow-trading-service/compare/fts-core-v0.1.0...fts-core-v0.1.1) - 2025-03-21

### Other

- release v0.1.0

## [0.1.0](https://github.com/forward-market-design/flow-trading-service/releases/tag/fts-core-v0.1.0) - 2025-03-21

### Fixed

- explicit versioning of workspace co-dependencies

### Other

- 🚀 initial release of flow trading software
