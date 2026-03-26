# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-03-27

### Added
- **50+ New Expert Skills**: Comprehensive SKILL.md guides across Backend, Frontend, Cloud (AWS/Azure/GCP), DevOps, Security, Data Science, and Mobile development.
- **Memory Migration CLI**: Added `sovereign memory export`, `import`, and `stats` to the core CLI.
- **Unified Village Integration**: Improved multi-agent coordination and cross-agent shared memory.
- **Natural Language Builder**: Enhanced `sovereign run` for autonomous agent spawning.
- **Getting Started Guide**: New concise `GETTING_STARTED.md` for both pre-built binary users and developers.
- **Integration Tests**: New workspace-wide memory consistency and tool tests.

### Fixed
- **Markdown Linting**: Resolved 344+ warnings across 139 files for 100% standards compliance.
- **Build Quality**: Fixed all `cargo clippy` and `rustfmt` issues in `sk-memory` and `sk-kernel`.
- **Formatting**: Unified whitespace and trailing character standards across the Rust crates.

### Changed
- **CLI Ergonomics**: Flattened memory commands for better usability.

## [1.0.0] - 2026-03-25

### Added
- **Core Village Engine**: The initial release of the Sovereign Kernel.
- **Security Sandbox**: Landlock LSM and seccomp-bpf integration.
- **Channel Bridge**: Support for Telegram, Discord, and Slack adapters.
- **The Forge**: CDP-based browser automation tools.
- **The Resurrector**: SQLite-based agent crash recovery and checkpoints.
