# Changelog — Sovereign Kernel

All notable changes to this project will be documented in this file.

## [1.0.1] — 2026-04-09

### Security
- **CRITICAL**: Scrubbed hardcoded API key from `.env` file. Key was never committed to git but existed on disk.
- **HIGH**: Removed hardcoded personal filesystem path from `config.toml`. Replaced with generic default.
- Hardened `.gitignore` to exclude `config.toml` (user-specific), `cargo_check.log`, and AI assistant directories.
- Created `config.toml.example` as the distributable template with full documentation.

### Changed
- **Modular Tool Registry**: Migrated tool execution from a monolithic `executor.rs` match statement to a dynamic `ToolRegistry` pattern with individual `ToolHandler` trait implementations.
- **Unified Approval Manager**: Consolidated the legacy `SafetyGate` and `ApprovalManager` into a single system with atomic pending-count tracking (fixes TOCTOU race condition).
- **Risk Classification**: Elevated `host_desktop_control`, `host_system_config`, and `host_install_app` to `RiskLevel::Critical`.
- **Event Bus**: Added `KernelEvent::Broadcast` variant for agent-to-agent broadcast communication.
- **Code Execution**: `code_exec` now defaults to sandboxed execution when in `ExecutionMode::Sandbox`.
- **System Prompts**: Made the system administrator authorization prompt conditional on `ExecutionMode::Unrestricted`.

### Fixed
- Fixed `AgentRegistry::get()` method call mismatch in agent tools.
- Fixed `McpServerConfig` → `McpServerConfigEntry` type mismatch in kernel MCP initialization.
- Fixed `SkillRegistry` field access (`skill.name` vs `skill.id`).
- Removed orphan `tools.rs` file from sk-kernel (module is a directory).
- Cleaned up `unused_import` warning in `sk-types/tests/config_test.rs`.

### Documentation
- Rewrote `README.md` with current 9-crate architecture.
- Updated `GETTING_STARTED.md` with `config.toml.example` workflow and full provider table.
- Updated `ARCHITECTURE.md` to reflect modular tool registry and removal of sk-channels.
- Updated `PROJECT_MAP.md` with accurate directory structure.
- Updated `SECURITY.md` with unified approval manager details.
- Updated `USER_GUIDE.md` with current API port (50051).
- Updated `docker-compose.yml` to remove obsolete dashboard port.
- Removed `cargo_check.log` from repository.

---

## [1.0.0-rc.3] — 2026-04-04

### Added
- **Backward-Compatible LLM Configuration**: Supported `[default_model]` (legacy table), `[[fallback_providers]]` (legacy array), and the new `[[llm]]` array format using custom Serde deserialization.
- **Robust API Key Resolution**: Implemented `LlmProviderSpec::resolve_api_key()` with tiered lookup (explicit key → env var → error).
- **Lock Safety Macros**: Introduced `rlock!`, `wlock!`, and `lock!` macros in `sk-kernel` to replace unsafe `.unwrap()` calls with explicit poisoning checks and file/line context.
- **Security Mitigation**: Added `blocked_args` (default: `["/C", "-c", "-Command"]`) to `ExecPolicy` to prevent arbitrary command execution via allowed shells.
- **Platform-Aware Sandbox**: Refactored `extract_base_command` to use `Path::file_name` for reliable cross-platform command extraction.

### Changed
- **Default API Port**: Standardized the REST/gRPC bridge port to `50051` across all components and documentation.
- **Windows Shell Security**: Added `cmd` and `powershell` to the default `ExecPolicy::safe_bins` while enforcing argument-level blocking.
- **Refactored `init_llm_driver`**: Prioritized the new `llm` provider array with improved error propagation for missing credentials.

### Fixed
- **Subprocess Sandbox Tests**: Resolved platform-specific failures by making tests aware of `dir` (Windows) vs `ls` (Unix).
- **Config Attributes**: Fixed duplicate `llm` field attributes causing model output errors.
- **Documentation Drift**: Synchronized `README.md`, `GETTING_STARTED.md`, and `docker-compose.yml` with the latest configuration standards.

---
