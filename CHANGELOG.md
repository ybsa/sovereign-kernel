# Changelog — Sovereign Kernel

All notable changes to this project will be documented in this file.

## [1.0.2] — 2026-04-14

### Added
- **Prompt Caching** across all three major cloud providers:
  - Anthropic: explicit `cache_control: ephemeral` on system block and last tool schema
  - OpenAI: automatic server-side caching with cache-hit logging via `prompt_tokens_details`
  - Gemini: `cachedContents` API with 1-hour TTL — system prompt hashed and re-used across turns
- **Local Model Support** (`is_local_model()` detection): Ollama, any localhost URL, and explicit `provider: "local"` configs now get a tighter 6-message rolling context window and skip JSON tool schemas (plain-text tool descriptions injected into the system prompt instead)
- **Small Model Support** (`is_small_model()`): expanded detection covers SmolLM, StableLM, DeepSeek-R1 1.5B/7B, Mistral 7B, and similar lightweight models
- **8 New Tool Categories**: `schedule_create`, `schedule_list`, `schedule_delete` (cron scheduler); `knowledge_add_entity`, `knowledge_add_relation`, `knowledge_query` (SQLite-backed knowledge graph); `event_publish` (kernel event bus); `process_list` (OS process listing)
- **`KernelEvent::Custom`** variant added to the event bus for arbitrary agent-to-agent events with typed payload
- **Model Catalog** updated with Claude 4.x (`claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`, `claude-sonnet-4-20250514`) and Gemini 2.5 (`gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.0-flash`)
- **Rolling Context Window**: agent loop now uses only the last N messages (10 for standard models, 6 for small/local). Prior session context surfaced via a summary prefix instead of full replay

### Fixed
- **All 11 Hands now fully operational**: `run.rs` previously only checked for file-based agent manifests. A third lookup branch was added to match `kernel.hands.get_definition(first_word)` — the hand's own `system_prompt` and `tools` list are now used directly
- **Token bloat** reduced from ~20K to ~3–6K per turn: hands use their own system prompt directly (no SOUL wrapper); default fallback caps changed from `"web"` (9 browser tools) to `["file_read", "web_search", "shell"]`; tool text listing removed from system prompt (LLM already receives schemas via the structured API field)
- `CronSchedule::Every { secs }` → `every_secs` field name corrected
- `cron.add()` → `cron.add_job(job, false)`, `cron.list_for_agent()` → `cron.list_jobs()`, `cron.remove()` → `cron.remove_job()` method name corrections
- `scheduler_create` alias added so mysql-reporter's HAND.toml resolves without modification
- Various build warnings resolved: unused import (`sk_types::Role`), spurious `mut` in gemini/openai drivers, `messages` pre-initialization in agent loop

### Removed
- Orphaned `crates/sk-kernel/src/tools/discovery.rs` stub (incomplete refactor, never integrated)
- SOUL wrapper injection for Hand system prompts — hands use their own identity directly
- Redundant tool text block from `build_system_prompt()` (was duplicating info the LLM already had)

---

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
