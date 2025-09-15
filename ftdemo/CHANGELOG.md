# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/forward-market-design/flow-trading-service/compare/ftdemo-v0.3.0...ftdemo-v0.4.0) - 2025-09-15

### Added

- capability to autosolve in response to bid changes
- distinguishable id types and affordances for v7-like uuids
- basic app_data

### Other

- *(tests,ftdemo)* add tests for uuid8 handling, and to improve understanding
- more flexible batches

## [0.3.0](https://github.com/forward-market-design/flow-trading-service/compare/ftdemo-v0.2.0...ftdemo-v0.3.0) - 2025-07-03

### Added

- introduce commands to ftdemo
- extract openapi schema to file

### Fixed

- ftdemo productdata to properly take rfc3339 timestamps

## [0.2.0](https://github.com/forward-market-design/flow-trading-service/compare/ftdemo-v0.1.1...ftdemo-v0.2.0) - 2025-06-27

### Added

- rewrite of core data model and architecture

## [0.1.1](https://github.com/forward-market-design/flow-trading-service/compare/ftdemo-v0.1.0...ftdemo-v0.1.1) - 2025-05-05

### Added

- schedule functionality

### Other

- scaffolding for scheduler

## [0.1.0](https://github.com/forward-market-design/flow-trading-service/releases/tag/ftdemo-v0.1.0) - 2025-04-28

### Added

- define core config type, update interfaces
- split the crates

### Other

- rename crate
- including explicit LICENSE.md file and updated readmes
