# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.2](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.5.1...fts-solver-v0.5.2) - 2025-09-15

### Added

- rename demand_group to demand

### Other

- *(tests,ftdemo)* add tests for uuid8 handling, and to improve understanding
- fix formatting
- rename product_group to basis

## [0.5.1](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.5.0...fts-solver-v0.5.1) - 2025-07-03

### Added

- dedicated types for demand and product groups

### Other

- update clarabel construction to new api
- *(deps)* bump clarabel from 0.10.0 to 0.11.1

## [0.5.0](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.4.0...fts-solver-v0.5.0) - 2025-06-27

### Added

- rewrite of core data model and architecture

## [0.4.0](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.3.1...fts-solver-v0.4.0) - 2025-04-28

### Added

- error propagation from solver
- attach demand curves directly to auths

### Other

- rename crate

## [0.3.1](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.3.0...fts-solver-v0.3.1) - 2025-04-21

### Added

- lp format export
- mps export

### Fixed

- remove export feature flag

### Other

- test suite for qp export
- skeleton for export functionality

## [0.2.0](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.1.2...fts-solver-v0.2.0) - 2025-04-11

### Added

- *(api)* replace per-auth outcome stream with per-submission outcome stream

### Fixed

- update osqp call to new api

### Other

- Merge pull request #38 from forward-market-design/dependabot/cargo/osqp-1.0.0
- *(deps)* bump osqp from 0.6.3 to 1.0.0

## [0.1.2](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.1.1...fts-solver-v0.1.2) - 2025-03-21

### Other

- explicit versions for all crates, no workspace-version

## [0.1.1](https://github.com/forward-market-design/flow-trading-service/compare/fts-solver-v0.1.0...fts-solver-v0.1.1) - 2025-03-21

### Other

- release v0.1.0
