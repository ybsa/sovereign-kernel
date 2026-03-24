# SOVEREIGN KERNEL: COMPLETE START-TO-FINISH IMPLEMENTATION PLAN

## PROJECT VISION
**Build an open-source AI agent operating system that can do anything a human can do on a computer, leveraging Universal Tooling (MCP) and on-the-fly code generation with user-controlled risk levels.**

---

# TABLE OF CONTENTS
1. Project Overview
2. Complete Architecture
3. Phase-by-Phase Implementation (Weeks 1-26)
4. **Phase 24: The Village (Agent Ecosystem)** ← NEW
5. Code Structure & File Organization
6. Technology Stack
7. Testing Strategy
8. Deployment & Release
9. Success Metrics

---

# 1. PROJECT OVERVIEW

## Goal
Create **Sovereign Kernel** — an AI agent that can:
- ✅ Control your entire computer (optional)
- ✅ Work autonomously 24/7
- ✅ Handle complex multi-step tasks
- ✅ Be safe if you want (Sandbox mode)
- ✅ Be powerful if you trust it (Unrestricted mode)
- ✅ Be fully transparent (audit everything)
- ✅ Be open source (anyone can trust it)

## User Story
```
User (Day 1): "Set up Sovereign Kernel in Sandbox mode"
              → Safe, agent restricted to /workspace
              → Test with simple tasks

User (Day 30): "I trust you completely, switch to Unrestricted"
               → Agent now has full computer access
               → Can automate entire workflow

User (Ongoing): "What did you do yesterday?"
                → Review audit logs
                → See every action taken
                → Understand agent behavior
```

---

# 2. COMPLETE ARCHITECTURE

## System Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                    USER INTERFACES                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ CLI/REPL     │  │ Web Dashboard│  │ Chat (Telegram,       │
│  │ (terminal)   │  │ (browser)    │  │  Discord, Slack)      │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
└────────────┬────────────────────────────────────────────┬─────────┘
             │ HTTP/WebSocket API                         │
┌────────────▼────────────────────────────────────────────▼─────────┐
│               SOVEREIGN KERNEL (The OS for Agents)                 │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  KERNEL CORE                                             │     │
│  │  ├─ Agent Scheduler (allocate compute time)              │     │
│  │  ├─ Cost Metering (track API spend, enforce budgets)     │     │
│  │  ├─ Heartbeat Monitor (agent health checks)              │     │
│  │  ├─ Cron Manager (schedule recurring tasks)              │     │
│  │  ├─ Approval Manager (human-in-the-loop gates)           │     │
│  │  └─ Supervisor (restart crashed agents)                  │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  EXECUTION ENGINE                                        │     │
│  │  ├─ Agent Loop (LLM thinking + tool execution)            │     │
│  │  ├─ Tool Runner (execute agents' commands safely)         │     │
│  │  ├─ Error Recovery (exponential backoff, retries)         │     │
│  │  ├─ Loop Guard (detect infinite loops)                    │     │
│  │  └─ Context Manager (manage token budgets)                │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  SECURITY & SANDBOXING                                   │     │
│  │  ├─ Execution Mode Selector                              │     │
│  │  │   ├─ SANDBOX: restricted to /workspace                │     │
│  │  │   └─ UNRESTRICTED: full computer access               │     │
│  │  ├─ Workspace Sandbox (path restrictions)                │     │
│  │  ├─ Subprocess Sandbox (command whitelist/allow-all)     │     │
│  │  ├─ Docker Sandbox (container isolation)                 │     │
│  │  ├─ Approval Gates (require human approval for risky)     │     │
│  │  ├─ Capability-Based Access Control (RBAC)               │     │
│  │  └─ Audit Trail (Merkle hash-chain of all actions)        │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │  CONFIGURATION & STATE                                   │     │
│  │  ├─ KernelConfig (execution mode, permissions, budgets)   │     │
│  │  ├─ Agent State (memory, context, status)                │     │
│  │  ├─ Execution State (running, paused, failed)            │     │
│  │  └─ Event Bus (internal communication)                    │     │
│  └──────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
             │                    │                │
    ┌────────▼──────────┐  ┌──────▼──────────┐  ┌─▼─────────────┐
    │  HANDS/TOOLS      │  │  LLM PROVIDERS  │  │  PERSISTENCE  │
    │  (Agent Hands)    │  │  (Brain)        │  │  (Memory)     │
    │                   │  │                 │  │               │
    │ ├─ Browser Hand   │  │ ├─ OpenAI       │  │ ├─ SQLite     │
    │ ├─ Shell Hand     │  │ ├─ Anthropic    │  │ ├─ PostgreSQL │
    │ ├─ Code Hand      │  │ ├─ Gemini       │  │ ├─ Redis      │
    │ ├─ File Hand      │  │ ├─ Groq         │  │ └─ Vector DB  │
    │ ├─ Web Search     │  │ └─ Fallback     │  │   (memories)  │
    │ ├─ Image Gen      │  │                 │  │               │
    │ ├─ Email          │  │                 │  │               │
    │ └─ API Calls      │  │                 │  │               │
    └───────────────────┘  └─────────────────┘  └───────────────┘
             │                    │                │
             └────────────────────┼────────────────┘
                                  │
                        ┌─────────▼──────────┐
                        │ REAL OPERATING     │
                        │ SYSTEM             │
                        │ (Linux/Mac/Windows)│
                        └────────────────────┘
```

## Data Flow

```
User Input (CLI/Web/Chat)
         ↓
   REST/WebSocket API
         ↓
   Kernel receives request
         ↓
Check Execution Mode (Sandbox vs Unrestricted)
         ↓
   Check Permissions/Approvals
         ↓
   Route to Agent Loop
         ↓
   Agent Loop:
   1. Receive prompt
   2. Call LLM (thinking)
   3. LLM requests tool use
   4. Check if tool allowed (based on mode)
   5. Request approval (if needed)
   6. Execute tool
   7. Log in audit trail
   8. Feed result back to LLM
   9. Repeat until done
         ↓
   Return results to User Interface
         ↓
User sees results (Web UI / Chat / CLI)
```

---

# 3. PHASE-BY-PHASE IMPLEMENTATION (26 WEEKS)

## PRE-PHASE: PREPARATION (Week 0)

### 0.1: Setup & Planning
- [x] Create GitHub repository (public)
- [x] Set up project management (GitHub Projects)
- [x] Create development branches
- [x] Write contributing guidelines
- [x] Set up CI/CD pipeline (GitHub Actions)

### 0.2: Documentation Structure
- [x] Create docs/ folder
- [x] Write README (project overview, quick start)
- [x] Write ARCHITECTURE.md (system design)
- [x] Write INSTALLATION.md (setup guide)
- [x] Write CONTRIBUTING.md (how to contribute)

### 0.3: Verify Current State
- [x] Confirm cargo build --release succeeds
- [x] Confirm cargo test --workspace passes
- [x] Confirm cargo clippy --workspace clean
- [x] Verify all phases 1-6 are actually done
- [x] Document current state in PROGRESS.md

**Timeline: 2-3 days**

---

## PHASE 7: EXECUTION MODES (Weeks 1-2) ✅ COMPLETED

**Goal: Let users choose Sandbox or Unrestricted mode**

### 7.1: Add ExecutionMode Enum ✅
**Tasks:**
- [x] Define ExecutionMode enum
- [x] Add to KernelConfig struct
- [x] Implement Default
- [x] Write unit tests

### 7.2: Create Config Templates ✅
**Tasks:**
- [x] Create sandbox config example
- [x] Create unrestricted config example
- [x] Create custom config example
- [x] Document each config option

### 7.3: Config Loading & Validation ✅
**Tasks:**
- [x] Load ExecutionMode from config file
- [x] Validate mode-specific settings
- [x] Show user what mode is active
- [x] Warn if using unrestricted mode
- [x] Unit tests for config loading

**Timeline: 1 week**
**Success Metric**: User can choose mode in config file

---

## PHASE 8: SANDBOX ENFORCEMENT (Weeks 3-4) ✅ COMPLETED

**Goal: Block dangerous actions in Sandbox mode**

### 8.1: Workspace Sandbox Enhancement ✅
**Tasks:**
- [x] Check file paths are within workspace (if Sandbox)
- [x] Block path traversal (`../`)
- [x] Block absolute paths outside workspace
- [x] Canonicalize paths to prevent tricks
- [x] Test: try to read /etc/passwd → blocked
- [x] Test: try to read /workspace/file → allowed
- [x] Test: unrestricted can read anywhere

### 8.2: Subprocess Sandbox Enhancement ✅
**Tasks:**
- [x] Check command against whitelist (if Sandbox)
- [x] Extract base command name
- [x] Block dangerous commands in Sandbox
- [x] Test: try `rm -rf /` → blocked in Sandbox
- [x] Test: try `git clone` → allowed in Sandbox
- [x] Test: unrestricted can run any command

### 8.3: Docker Sandbox ✅
- [x] Verify Docker sandbox works in both modes
- [x] In Sandbox mode: use Docker for extra isolation
- [x] In Unrestricted mode: use Docker optionally when host dependencies (Python/Node) are missing to ensure tool reliability.

**Timeline: 1 week**
**Success Metric**: Sandbox mode blocks path traversal and unapproved commands

---

## PHASE 9: APPROVAL GATES (Weeks 5-6) ✅ COMPLETED

**Goal: Ask human permission for risky actions**

### 9.1: Approval Manager Enhancement ✅
**Tasks:**
- [x] Update ApprovalManager to check execution mode
- [x] Different approval policies for each mode
- [x] Implement approval request flow
- [x] Add timeout for approvals (default: 60 seconds)
- [x] Test: Sandbox asks before shell_exec
- [x] Test: Unrestricted only asks for user-defined actions

### 9.2: Approval Handler (Web UI / CLI)
**Tasks:**
- [x] CLI: Show prompt "Approve action? (y/n)"
- [ ] Web UI: Show clickable Approve/Deny buttons
- [x] Handle timeout (default deny)
- [x] Log user decision in audit trail
- [x] Test: User can approve/deny from CLI
- [ ] Test: User can approve/deny from Web UI

**Timeline: 1 week**
**Success Metric**: Agent pauses for approval, waits for user response

---

## PHASE 10: AUDIT TRAIL (Weeks 7-8) ✅ COMPLETED

**Goal: Log every action with Merkle chain**

### 10.1: Enhanced Audit System ✅
**Tasks:**
- [x] Log every action (file read, shell exec, approval decision, etc.)
- [x] Include execution mode in each log entry
- [x] Calculate Merkle hash for each entry
- [x] Store in SQLite with chain verification
- [x] Create audit_log table schema
- [x] Add API endpoint to view audit logs
- [x] Test: Verify chain integrity

### 10.2: Audit Log Viewer
**Tasks:**
- [x] CLI command to list audit logs
- [x] Filter by agent, date range, action type
- [x] Verify Merkle chain integrity
- [x] Export to JSON/CSV
- [ ] Web UI: Searchable audit log viewer

**Timeline: 1.5 weeks**
**Success Metric**: Every action logged, chain can be verified

---

## PHASE 11: API BRIDGE (Weeks 9-10) ✅ COMPLETED

**Goal: REST/WebSocket API so external apps can talk to kernel**

### 11.1: HTTP/WebSocket Server
**Tasks:**
- [ ] Add actix-web & tokio dependencies
- [ ] Implement spawn endpoint (returns agent_id)
- [ ] Implement stop endpoint
- [ ] Implement status endpoint
- [ ] Implement WebSocket for events
- [ ] Implement approval resolution
- [ ] Add error handling (500, 404, etc.)
- [ ] Add request logging
- [ ] Test with curl & websocat

### 11.2: Event Bus Integration
**Tasks:**
- [ ] Define Event enum
- [ ] Emit events from agent loop
- [ ] Stream events via WebSocket
- [ ] Serialize to JSON
- [ ] Test: Events stream to client in real-time

**Timeline: 1.5 weeks**
**Success Metric**: Can start agent via curl, stream events via WebSocket

---

## PHASE 12: WEB DASHBOARD (Weeks 11-13) ✅ COMPLETED

**Goal: Beautiful UI to manage agents**

### 12.1: Backend Dashboard API ✅
**Tasks:**
- [x] Implement dashboard API endpoints (`/api/status`, `/api/hands`, `/api/agents`, `/api/channels`, `/api/audit/recent`)
- [x] Return JSON responses
- [x] OpenAI-compatible API (`/v1/chat/completions`, `/v1/models`)
- [x] Test endpoints

### 12.2: Frontend Dashboard (Terminal Web UI) ✅
**Tasks:**
- [x] Embedded single-file SPA (no Node.js — compiled into the binary)
- [x] Terminal aesthetic: jet black, Geist Mono, green/cyan/amber
- [x] Three-pane layout: agents/hands | live log | command bar
- [x] Agents and hands status panel
- [x] Live log streaming pane
- [x] Real-time stats: uptime, hands active, approvals pending
- [x] `sovereign dashboard [--port PORT] [--no-open]` CLI command

**Timeline: 2 weeks**
**Success Metric**: ✅ ACHIEVED — `sovereign dashboard` opens terminal UI at http://localhost:8080

---

## PHASE 13: HANDS/TOOLS - CORE (Weeks 14-16) ✅ COMPLETE

**Goal: Implement core tools agents use**

### 13.1: Browser Hand (Native Rust / chromiumoxide)
**Features:**
- [x] Navigate to URL (wired to BrowserManager)
- [ ] Take screenshot
- [ ] Extract text/content
- [ ] Click elements
- [ ] Fill forms
- [ ] Submit forms
- [ ] Handle JavaScript execution
- [ ] Wait for elements
- [ ] Screenshot caching

### 13.2: Shell Hand (Command Execution)
**Features:**
- [x] Execute arbitrary commands
- [x] Capture stdout/stderr (separately)
- [x] Handle exit codes
- [x] Timeout enforcement
- [x] Working directory support
- [x] Environment variable filtering
- [x] Whitelist/allow-all based on mode (ExecPolicy)

### 13.3: Code Hand (Write & Execute Code) — NEW
**Features:**
- [x] Execute Python scripts
- [x] Execute JavaScript (Node.js)
- [x] Execute Bash scripts
- [x] Capture output with timeout
- [x] Error handling
- [x] Security policy enforcement (ExecPolicy)

### 13.4: File Hand (Read/Write Files)
**Features:**
- [x] Read files (with 1MB size limit)
- [x] Write files
- [x] Append to files
- [x] Create directories
- [x] Delete files/folders
- [x] Move/copy files
- [x] List directories (rich metadata: size, type, modified)
- [x] Path safety validation (sandbox mode)

**Timeline: 2 weeks**
**Success Metric**: Agent can browse, run code, and manage files ✅ ACHIEVED

---

## PHASE 14: HANDS/TOOLS - OPTIONAL (Weeks 17-18) ✅ COMPLETE

### 14.1: Web Search Hand ✅
- [x] Brave Search API integration (`BRAVE_API_KEY`)
- [x] Tavily AI Search support (`TAVILY_API_KEY`)
- [x] Multi-phase research pipeline (query planning → search → synthesis → bias check → report)
- [x] Source credibility tiers (academic > journalism > blog)
- [x] Citation standards (every fact linked to a source URL)
- [x] Anti-hallucination protocol
- [x] HAND.toml + SKILL.md bundled in sk-hands

### 14.2: Email Hand ✅
- [x] SMTP integration (send email via Python/smtplib)
- [x] IMAP integration (read email via Python/imaplib)
- [x] Gmail App Password support
- [x] Draft mode (require approval before sending)
- [x] Inbox triage framework (action/FYI/waiting/spam)
- [x] Contact knowledge graph
- [x] Follow-up tracking via scheduler
- [x] HAND.toml + SKILL.md bundled in sk-hands

### 14.3: Image Generation Hand
- [ ] DALL-E 3 integration
- [ ] Stable Diffusion integration
- [ ] Image storage
- [ ] Cost tracking

**Timeline: 1 week**
**Success Metric**: ✅ ACHIEVED — 10 bundled hands total (7 original + web-search + email + otto)

---

## PHASE 15: MULTI-AGENT & COORDINATION (Weeks 19-20) - ✅ COMPLETE

### 15.1: Agent-to-Agent Communication ✅
- [x] A2A protocol (message passing via Inter-Agent Bus)
- [x] Persistent routing (messages saved directly to recipient's session)
- [x] Integrated `agent_message` tool

### 15.2: Delegation System ✅
- [x] Agent A delegates task to Agent B (Witch Spawning)
- [x] Forced Sandbox mode for all witch_skeleton agents (Security Guard)
- [x] Status polling via `check_witch_skeleton`

### 15.3: Shared Semantic Memory ✅
- [x] Global `global_knowledge` SQLite table in Memory Substrate
- [x] Semantic recall tools for cross-agent knowledge sharing
- [x] Capability-gated access (`SharedMemory` permission)

**Timeline: 1.5 weeks**
**Success Metric**: Two agents can work together on same task

---

## PHASE 16: PRODUCTION HARDENING (Weeks 21-22) ✅ COMPLETE

### 16.1: Logging & Tracing ✅
- [x] Structured logging (tracing crate)
- [x] Multiple log levels (DEBUG, INFO, WARN, ERROR)
- [x] Log rotation via `tracing-appender`
- [x] Non-blocking background worker
- [x] Centralized daily log files

### 16.2: Error Recovery ✅
- [x] Fallback LLM providers (Sentinel Driver)
- [x] Automatic retries with exponential backoff
- [x] Failover chain: Anthropic → OpenAI → Gemini → Groq

### 16.3: Simplified CLI (`sovereign do`) ✅
- [x] Added `do` alias for natural language task execution
- [x] Unified NL Builder entry point

### 16.4: Live Dashboard Wiring ✅
- [x] Connected Axum API to live kernel state
- [x] Real-time agent status and audit logs in dashboard

### 16.3: Performance Optimization
- [ ] Benchmark agent loop latency
- [ ] Optimize context window usage
- [ ] Cache LLM responses (where safe)
- [ ] Parallelize tool execution
- [ ] Memory & CPU profiling
- [ ] Load testing (100+ agents)

### 16.4: Security Audit
- [ ] Verify no shell escapes
- [ ] Verify no path traversal
- [ ] Verify no credential leaks
- [ ] RBAC enforcement check
- [ ] Audit trail tamper-proof verification
- [ ] Consider third-party security review

**Timeline: 1.5 weeks**
**Success Metric**: System stable with 100+ concurrent agents

---

## PHASE 17: CHAT INTEGRATIONS (Weeks 23-24) ✅ COMPLETED

### 17.1: Telegram Bot
**Features:**
- [ ] Receive messages
- [ ] Send to Sovereign Kernel
- [ ] Stream responses back
- [ ] Handle inline keyboards (approve/deny)
- [ ] Long polling

### 17.2: Discord Bot
- [ ] Receive messages
- [ ] Send to Sovereign Kernel
- [ ] Embed responses
- [ ] Button interactions

### 17.3: Slack Integration
- [ ] Receive messages
- [ ] Send to Sovereign Kernel
- [ ] Block Kit UI (approve/deny)
- [ ] OAuth setup

**Timeline: 1.5 weeks**
**Success Metric**: Can control agent via Telegram/Discord/Slack

---

## PHASE 18: THE BUILDER AGENT (Weeks 25-26) ✅ COMPLETE

**Goal: Shift from hand-coded tools to autonomous compilation and execution where the agent builds its own capabilities.**

### 18.1: The Synthesis Engine ✅
- [x] Dynamic code execution environments (`ottos_outpost`).
- [x] Zero-Pollution Sandbox execution using Docker.
- [x] Native fallback support for OS-level interactions.

### 18.2: The OTTO Hand ✅
- [x] Custom system prompt and specialized tooling.
- [x] Integration with Kernel executor.
- [x] Dynamic fetching and execution of arbitrary python/node packages without polluting host OS.

**Timeline: 2 weeks**
**Success Metric**: ✅ ACHIEVED — Agent can complete a task requiring external dependencies by building a Docker isolation container.

---

## PHASE 19: UNIVERSAL TOOLING & MCP (Next Milestone) ✅ COMPLETED

**Goal: Expand the Laboratory infinitely by connecting to standard Model Context Protocol servers.**

### 19.1: Universal Tooling (MCP)
- [x] Implement full MCP client in `sk-mcp`.
- [x] Connect agent to local MCP servers.
- [x] **Autonomous Discovery**: Agent searches online registries (e.g., Smithery.ai) for missing tools.

### 19.2: Zero-Touch Install
- [x] Agent automatically pulls, configures, and runs MCP servers in Docker containers via OTTO.

**Timeline: 2 weeks**
**Success Metric**: Agent can complete a "human-level" task using an external MCP server.

---

## PHASE 20: DOCUMENTATION & LORE (Weeks 27-28) ✅ COMPLETE

### 20.1: Dark Fantasy Lore Integration ✅
- [x] Internal dictionaries updated to match the lore (Witch Skeleton, King, Laboratory, Grimoires).
- [x] Complete PROJECT_MAP.md terminology guide.

### 20.2: Core Documentation Update
- [ ] README.md (overview, quick start)
- [ ] ARCHITECTURE.md (system design)
- [ ] API_DOCS.md (REST/WebSocket API)
- [ ] SECURITY_MODEL.md (Universal Tooling & Sandboxing)

**Timeline: 1.5 weeks**
**Success Metric**: Public release readiness, documentation polished.

---

## PHASE 21: ULTRA-SOVEREIGN CAPABILITIES (The Horizon) ✅ COMPLETED

**Goal: Achieve total autonomy where the agent continuously improves and collaborates.**

### 21.1: Self-Refactoring (Native Optimization)
- [x] **Python to Rust Conversion**: When a self-built Python tool is used frequently, the agent rewrites it in Rust for 100x performance.
- [x] **Dynamic Compilation**: Kernel automatically compiles new Rust "Skills" and hot-reloads them into the core without stopping.

### 21.2: The Global Skill Graph (P2P Sharing)
- [ ] **Peer Discovery**: (Opt-in) Agents share the "blueprints" of tools they've built with other Sovereign Kernels.
- [ ] **Collaborative Learning**: If one agent builds a "PDF-to-Braille" tool, all other agents in the network can "learn" and use it instantly.

---

## PHASE 22: FULL GUI & SCREEN CONTROL (The Eyes & Hands) ✅ COMPLETED

**Goal: Give the agent full human-like access to the graphical desktop — see the screen, click, type, scroll, and interact with ANY application, not just the terminal.**

### 22.1: Browser Automation (Complete)
- [x] Wire `browser_click` to native Rust `chromiumoxide`.
- [x] Wire `browser_type` to native Rust `chromiumoxide`.
- [x] Wire `browser_screenshot` to native Rust CDP calls.
- [x] Wire `browser_read_page` to extract visible text/DOM.
- [x] Wire `browser_close` to close tabs/browser via CDP.
- [ ] Add `browser_scroll`, `browser_wait`, `browser_evaluate_js`.
- [ ] Test: Agent can log into a website, fill a form, and submit it.

### 22.2: Screen Capture & Vision
- [ ] Implement cross-platform screenshot tool (Windows: `win32`, Linux: `scrot`/`xdotool`, Mac: `screencapture`).
- [ ] Send screenshots to multimodal LLM (GPT-4o, Gemini, Claude) for visual understanding.
- [ ] Agent can answer "What's on my screen right now?"
- [ ] Agent can locate UI elements by description ("find the Submit button").
- [ ] Implement screen region capture (crop to specific area).

### 22.3: Desktop Automation (Mouse & Keyboard)
- [ ] Implement cross-platform mouse control (`click`, `move`, `drag`, `scroll`).
- [ ] Implement cross-platform keyboard control (`type`, `hotkey`, `press`).
- [ ] Platform backends: Windows (Win32 API), Linux (X11/xdotool), Mac (CoreGraphics).
- [ ] Coordinate system: pixel-based with screen resolution detection.
- [ ] Safety: Always require approval in Sandbox mode before GUI actions.
- [ ] Test: Agent can open Notepad, type text, save the file, and close it.

### 22.4: Application Awareness
- [ ] List running applications/windows (cross-platform).
- [ ] Focus/switch between windows.
- [ ] Read window titles for context.
- [ ] Detect active application for context-aware assistance.

**Timeline: 3-4 weeks**
**Success Metric**: Agent can complete a task that requires GUI interaction — e.g., open a spreadsheet app, enter data, save it, and email it.

---

## PHASE 23: UNIVERSAL CROSS-PLATFORM SUPPORT (Every Device) ✅ COMPLETED

**Goal: Sovereign Kernel runs on EVERY device — from Raspberry Pi to enterprise servers. One binary, any platform.**

### 23.1: Tier 1 Platforms (Full Support)
- [x] **Windows** (x86_64) — current primary development platform.
- [ ] **Linux** (x86_64) — CI builds, Docker, bare metal servers.
- [ ] **macOS** (x86_64 + Apple Silicon/aarch64) — native ARM support.

### 23.2: Tier 2 Platforms (ARM / Embedded)
- [ ] **Linux ARM64** (aarch64) — Raspberry Pi 4/5, NVIDIA Jetson, AWS Graviton.
- [ ] **Linux ARMv7** (armhf) — Raspberry Pi 3, older ARM boards.
- [ ] Cross-compile with `cross` or `cargo-zigbuild` for ARM targets.
- [ ] Optimize build: strip debug symbols, LTO, `opt-level=s` for embedded.
- [ ] Test: Sovereign Kernel runs on Raspberry Pi 4 with 4GB RAM.

### 23.3: Lightweight Mode (Resource-Constrained Devices)
- [ ] Detect available RAM at startup and auto-tune memory usage.
- [ ] Disable non-essential features on low-RAM devices (< 2GB): no browser, no Docker sandbox.
- [ ] Use smaller/local LLMs on edge devices (Ollama, llama.cpp integration).
- [ ] SQLite-only mode (disable optional PostgreSQL/Redis).
- [ ] Headless mode (no dashboard) for IoT/server deployments.

### 23.4: CI Cross-Platform Build Matrix
- [x] GitHub Actions build matrix: `ubuntu-latest`, `macos-latest`, `windows-latest`.
- [ ] ARM cross-compilation in CI: `aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`.
- [ ] Release binaries for all 5+ targets on every tagged release.
- [x] Platform-specific integration tests (filesystem paths, shell commands, process management).

### 23.5: Platform-Specific Adaptations
- [ ] Shell commands: auto-detect `cmd.exe` vs `bash` vs `sh`.
- [ ] Path separators: `\` (Windows) vs `/` (Unix) — already handled via `std::path`.
- [ ] Process tree management: Windows (`taskkill`) vs Unix (`kill -SIGTERM`).
- [ ] Docker availability detection: graceful fallback if Docker not installed.
- [ ] Home directory detection: `%USERPROFILE%` vs `$HOME`.

**Timeline: 2-3 weeks**
**Success Metric**: Same `sovereign chat` binary works on Windows laptop, Linux server, Mac desktop, and Raspberry Pi — with zero code changes.

---

## PHASE 24: THE VILLAGE — AGENT ECOSYSTEM ✅ COMPLETE

The following gaps were identified during a comprehensive code-level audit:

| # | Problem | Where It Hurts | Severity |
|---|---------|---------------|----------|
| P1 | **No host-level tools** — `ExecutionMode::Unrestricted` exists but there are no tools for desktop control, system config, or app installation | Agents can't interact with the full OS in Unrestricted mode | 🔴 Critical |
| P2 | **No crash recovery** — `Supervisor` detects dead agents and restarts, but loses all state (memory, context, progress) | Long-running agents lose hours of work on crash | 🔴 Critical |
| P3 | **Monolithic executor** — `executor.rs` (1,339 LOC) is a single match statement dispatching 25+ tools; untestable | Adding new tools requires editing a god-function; zero tests on executor | 🟡 High |
| P4 | **Low-level agent spawning** — `spawn_witch_skeleton` requires explicit capability lists and system prompts | Users must understand internal architecture to create agents | 🟡 High |
| P5 | **No simplified CLI** — requires `sovereign daemon` → `sovereign hands activate` chain | New users bounce; too many steps to "just do a task" | 🟡 Medium |
| P6 | **Dashboard serves static data** — API endpoints return hardcoded values, not live kernel state | Dashboard is visual-only; not operational | 🟡 Medium |

### 🏘️ Village Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        THE VILLAGE (Sovereign Kernel)               │
│                                                                     │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐          │
│  │   COTTAGE    │  │     MANOR     │  │      ALARM       │          │
│  │  (Sandbox)   │  │ (Unrestricted │  │   (Scheduled)    │          │
│  │              │  │  + Approval)  │  │                  │          │
│  │ • File read  │  │ • Full FS     │  │ • Sleeps until   │          │
│  │ • Web fetch  │  │ • System cfg  │  │   wake time      │          │
│  │ • Safe code  │  │ • Install apps│  │ • Becomes Manor  │          │
│  │ • Docker     │  │ • Desktop ctl │  │   or Cottage     │          │
│  │   isolated   │  │ • All tools   │  │ • Dies after     │          │
│  │              │  │ + Elder gate  │  │   completion     │          │
│  └──────┬───────┘  └───────┬───────┘  └────────┬─────────┘          │
│         └──────────────────┼───────────────────┘                    │
│                            ▼                                        │
│              ┌────────────────────────┐                              │
│              │     VILLAGE SQUARE     │  ← InterAgentBus (exists)   │
│              │       (A2A Bus)        │                              │
│              └──────────┬─────────────┘                              │
│                         │                                           │
│    ┌────────────────────┼────────────────────────┐                   │
│    ▼                    ▼                        ▼                  │
│ ┌───────────┐    ┌────────────┐    ┌──────────────────┐             │
│ │ RESURRECTOR│    │  BUILDER   │    │     ELDER        │             │
│ │ (enhanced  │    │ (enhanced  │    │   (Approval UI)  │             │
│ │ Supervisor)│    │  Wizard)   │    │                  │             │
│ │            │    │            │    │ • Risk tiers     │             │
│ │ Watches    │    │ NL task →  │    │ • Auto-approve   │             │
│ │ all agents │    │ agent with │    │   low-risk       │             │
│ │ Revives    │    │ right tools│    │ • Prompt for     │             │
│ │ from       │    │ right mode │    │   high-risk      │             │
│ │ checkpoint │    │ right prompt│   │ • Block critical │             │
│ └───────────┘    └────────────┘    └──────────────────┘             │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              MEMORY POND (sk-memory)                        │    │
│  │    SQLite + BM25 + Semantic + NEW: Checkpoint table          │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

---

### 24.1: Host Tools & Unrestricted Tooling (Week 1)

**Problem:** `ExecutionMode::Unrestricted` and `SafetyGate` exist, but there are no tools that actually leverage full host access.

**Solution:** Add 4 new tool modules in `sk-tools/src/host/` that are automatically available in Unrestricted mode. Enhance `SafetyGate` with 3-tier risk classification.

**Existing code to extend (NOT replace):**
- `sk-types/src/config.rs` — `ExecutionMode::Unrestricted` already defined
- `sk-kernel/src/approval.rs` — `SafetyGate` already enforces approval flow
- `sk-kernel/src/executor.rs` — tool dispatch already checks mode
- `sk-memory/src/audit.rs` — Merkle audit trail already logs every action

**Tasks:**
- [ ] Create `sk-tools/src/host/mod.rs` — module root
- [ ] Create `sk-tools/src/host/desktop_control.rs` — Wallpaper, theme, notification tools
- [ ] Create `sk-tools/src/host/system_config.rs` — Edit system configs, manage services
- [ ] Create `sk-tools/src/host/app_installer.rs` — Install via `winget`/`apt`/`brew` (auto-detect OS)
- [ ] Create `sk-tools/src/host/file_full.rs` — Full filesystem access (no workspace restriction)
- [ ] Add 3-tier risk classification to `SafetyGate`:
  - **Low**: Read system info, list processes → auto-approve
  - **Medium**: Write files outside workspace → warn + approve
  - **High**: Install apps, modify system config → explicit approval + timeout
- [ ] Register host tools in `executor.rs` (only when `ExecutionMode::Unrestricted`)
- [ ] Tests: Verify host tools are blocked in Sandbox, available in Unrestricted
- [ ] Tests: Verify risk tier classification routes to correct approval flow

**Files created/modified:**
| File | Action | Lines |
|------|--------|-------|
| `sk-tools/src/host/mod.rs` | NEW | ~30 |
| `sk-tools/src/host/desktop_control.rs` | NEW | ~150 |
| `sk-tools/src/host/system_config.rs` | NEW | ~200 |
| `sk-tools/src/host/app_installer.rs` | NEW | ~250 |
| `sk-tools/src/host/file_full.rs` | NEW | ~100 |
| `sk-kernel/src/approval.rs` | MODIFY | +50 (risk tiers) |
| `sk-kernel/src/executor.rs` | MODIFY | +30 (register host tools) |

**Success Metric:** `sovereign run "install firefox" --mode unrestricted` → approval prompt → installs → audit logged.

---

### 24.2: The Resurrector — Crash Recovery (Week 2)

**Problem:** `Supervisor` (in `sk-kernel/src/supervisor.rs`) detects dead agents via heartbeat and restarts them, but fresh — all conversation history, tool state, and progress are lost.

**Solution:** Add a `checkpoints` table to `MemorySubstrate`, periodic state snapshots, and checkpoint-aware restart in `Supervisor`.

**Existing code to extend:**
- `sk-kernel/src/supervisor.rs` — already has `check_health()` + `restart_agent()` (7,365 LOC)
- `sk-memory/src/substrate.rs` — schema init, need 1 new table
- `sk-memory/src/session.rs` — session save/load already works

**Note on naming:** The existing "HEALER" is the context compactor (`compactor.rs`). This feature is the **Resurrector** — agent crash recovery — to avoid confusion.

**Tasks:**
- [ ] Add `checkpoints` table to `MemorySubstrate::initialize_schema()`:
  ```sql
  CREATE TABLE IF NOT EXISTS checkpoints (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      agent_id TEXT NOT NULL,
      session_id TEXT NOT NULL,
      agent_config TEXT NOT NULL,
      tool_state TEXT DEFAULT '{}',
      created_at TEXT NOT NULL,
      FOREIGN KEY (agent_id) REFERENCES agents(id)
  );
  ```
- [ ] Create `sk-memory/src/checkpoint.rs` — `CheckpointStore` with `save()`, `load_latest()`, `prune(keep_last: usize)`
- [ ] Add periodic checkpoint call (every 30s) in agent loop background task
- [ ] Modify `Supervisor::restart_agent()` → `Supervisor::resurrect_agent()`:
  - Load latest checkpoint
  - Restore session from checkpoint
  - Inject system message: "[Resurrector] You were restarted from checkpoint. Previous session restored."
  - Resume agent loop
- [ ] Add `/resurrector` bot command to show resurrection history
- [ ] Tests: Kill agent → verify auto-restart from checkpoint within 10s
- [ ] Tests: Checkpoint pruning keeps only last 5

**Files created/modified:**
| File | Action | Lines |
|------|--------|-------|
| `sk-memory/src/checkpoint.rs` | NEW | ~200 |
| `sk-memory/src/substrate.rs` | MODIFY | +15 (schema) |
| `sk-memory/src/lib.rs` | MODIFY | +2 (pub mod) |
| `sk-kernel/src/supervisor.rs` | MODIFY | +80 (resurrect logic) |
| `sk-engine/src/agent_loop.rs` | MODIFY | +20 (checkpoint trigger) |

**Success Metric:** Kill -9 an agent → Resurrector detects within 10s → restarts from last checkpoint → user sees "Resurrector revived agent-123".

---

### 24.3: Natural Language Builder (Week 3)

**Problem:** `spawn_witch_skeleton` requires explicit `skeleton_name`, `task_description`, and `capabilities` list. `SetupWizard` generates manifests but isn't accessible from CLI. Users need to understand internal tool names to spawn agents.

**Solution:** Enhance `SetupWizard` to use the LLM for task analysis — given a plain English description, automatically determine the right tools, mode, system prompt, and capabilities.

**Existing code to extend:**
- `sk-kernel/src/wizard.rs` — `SetupWizard::build_plan()` already exists
- `sk-kernel/src/executor.rs` — `spawn_witch_skeleton` tool handler exists (lines 196-263)
- `sk-types/src/config.rs` — `ExecutionMode` enum exists
- `sk-kernel/src/cron.rs` — `CronScheduler` already handles scheduled execution

**Tasks:**
- [ ] Add `SetupWizard::build_from_natural_language()` — sends task to LLM, gets back:
  - Recommended tools (from `AVAILABLE_TOOLS` constant)
  - Execution mode (Sandbox/Unrestricted)
  - Generated system prompt
  - Suggested name
- [ ] Enhance `spawn_witch_skeleton` to accept `mode_hint` parameter: `safe`, `full_access`, `scheduled`
- [ ] For `scheduled` mode: auto-create `CronJob` via existing `CronScheduler`
- [ ] Add `builder` tool that agents can call to spawn sub-agents via NL description
- [ ] Tests: NL "monitor CPU" → spawns agent with `shell_exec` tool
- [ ] Tests: NL "email me tomorrow" → spawns scheduled agent with cron job

**Files created/modified:**
| File | Action | Lines |
|------|--------|-------|
| `sk-kernel/src/wizard.rs` | MODIFY | +150 (NL builder) |
| `sk-kernel/src/executor.rs` | MODIFY | +40 (mode hint, builder tool) |
| `sk-types/src/config.rs` | MODIFY | +10 (ExecutionModeHint enum) |

**Success Metric:** `sovereign run "Create an agent that monitors CPU and alerts if >90%"` → Builder analyzes → spawns Cottage agent with `shell_exec` → agent starts.

---

### 24.4: Simplified CLI — `sovereign run` (Week 3)

**Problem:** Current CLI requires multi-step workflow: `sovereign daemon start` → `sovereign hands activate` → `sovereign chat`. Too complex for one-shot tasks.

**Solution:** Add `sovereign run "task"` command that wraps the NL Builder + agent spawn into a single ergonomic command.

**Existing code to extend:**
- `sk-cli/src/main.rs` — Clap command definitions
- `sk-kernel/src/wizard.rs` — (enhanced in 24.3)

**Tasks:**
- [ ] Add `Run` subcommand to CLI:
  ```
  sovereign run "task description" [--mode sandbox|unrestricted|auto] [--schedule "cron or NL time"]
  ```
- [ ] `--mode auto` (default): LLM determines Sandbox vs Unrestricted based on task
- [ ] `--schedule`: Parses NL time ("tomorrow 9am", "every Monday") via LLM, creates CronJob
- [ ] Add `sovereign status` command (list all active agents, village state)
- [ ] Add `sovereign kill <id>` command (stop agent by ID)
- [ ] Update `--help` text with village terminology

**Files created/modified:**
| File | Action | Lines |
|------|--------|-------|
| `sk-cli/src/main.rs` | MODIFY | +80 (run/status/kill commands) |
| `sk-cli/src/commands/run.rs` | NEW | ~120 |

**Success Metric:**
```
sovereign run "Calculate monthly expenses from CSV"          # Sandbox
sovereign run "Clean up temp files" --mode unrestricted      # Manor + approval
sovereign run "Message mom good morning" --schedule "09:00"  # Alarm
sovereign status                                             # List all agents
```

---

### Phase 24 Summary

| Sub-Phase | Week | New Files | Modified Files | Est. LOC |
|-----------|------|-----------|----------------|----------|
| 24.1 Host Tools | 1 | 5 | 2 | ~810 |
| 24.2 Resurrector | 2 | 1 | 4 | ~320 |
| 24.3 NL Builder | 3 | 0 | 3 | ~200 |
| 24.4 Simplified CLI | 3 | 1 | 1 | ~200 |
| **Total** | **3 weeks** | **7** | **10** | **~1,530** |

**Overall Success Metric — The Village Test:**
```bash
# 1. Sandbox (Cottage): Safe task
sovereign run "Read my todo.txt and summarize it"

# 2. Unrestricted (Manor): Dangerous task with approval
sovereign run "Delete ~/temp/old_files" --mode unrestricted
# → Elder shows approval dialog → User approves → Executes → Audit logged

# 3. Scheduled (Alarm): Timed task
sovereign run "Check disk space" --schedule "every 6 hours"
# → Creates cron job → Agent wakes, checks, reports

# 4. Resurrector: Crash recovery
# Kill -9 an agent → Resurrector revives from checkpoint within 10s

# 5. Builder: Natural language agent creation
sovereign run "Create agent that monitors CPU and alerts if >90%"
# → Analyzes → Spawns Cottage with shell_exec → Runs autonomously
```

**Timeline: 3 weeks from execution start**

---

# 4. CODE STRUCTURE & FILE ORGANIZATION

*(See full architecture details for expected hierarchy spanning `crates/`, `docs/`, `scripts/`, etc. as defined during project scaffold).*

---

# 5. TECHNOLOGY STACK

## Core
- **Language**: Rust (memory-safe, fast, concurrent)
- **Async Runtime**: Tokio (async/await)
- **Build**: Cargo (dependency management)
- **Cross-Compile**: `cross` / `cargo-zigbuild` (ARM targets)

## Web Framework
- **HTTP**: Actix-web or Axum
- **WebSocket**: Tokio-tungstenite
- **Serialization**: Serde (JSON)

## Database
- **SQLite**: Local storage (primary — memory, audit trail, sessions)
- **PostgreSQL**: Optional (production scale)
- **Redis**: Optional (caching, session state)
- **Vector DB**: Qdrant or Milvus (agent memories)

## LLM Integration
- **OpenAI**: openai-api crate
- **Anthropic**: anthropic-sdk
- **Google**: google-generativeai
- **Groq**: groq-sdk
- **Local**: Ollama / llama.cpp (edge devices)

## Hands/Tools
- **Browser**: Native Rust (chromiumoxide)
- **Screen Control**: Platform-native APIs (Win32, X11, CoreGraphics)
- **Code Execution**: Docker
- **Search**: Brave / Tavily / DuckDuckGo
- **Email**: lettre (SMTP)
- **Image Gen**: DALL-E API

## Chat Integrations
- **Telegram**: Teloxide
- **Discord**: Serenity
- **Slack**: Slack-morphism

## Frontend
- **Option 1**: Leptos (Rust, full-stack)
- **Option 2**: Yew (Rust, WASM)
- **Option 3**: React/TypeScript (separate project)

## Target Platforms
- **Windows**: x86_64 (Tier 1)
- **Linux**: x86_64 (Tier 1), aarch64 (Tier 2), armv7 (Tier 2)
- **macOS**: x86_64 + Apple Silicon (Tier 1)
- **Embedded**: Raspberry Pi 3/4/5, NVIDIA Jetson

## DevOps
- **Containerization**: Docker
- **Orchestration**: Docker Compose, Kubernetes
- **CI/CD**: GitHub Actions (cross-platform matrix)
- **Monitoring**: Prometheus (metrics), Grafana (dashboards)

---

# PHASE 24: THE VILLAGE — AGENT ECOSYSTEM ✅ COMPLETE
> **Status**: All 4 sub-phases implemented and compiling.

## 24.1: Host Tools & Unrestricted Tooling ✅
- Created `sk-tools/src/host/` with 4 modules: `desktop_control`, `system_config`, `app_installer`, `file_full`
- Registered host tools in executor with mode-dependent loading (Unrestricted only)
- Added 3-tier risk classification in `approval.rs`: Low (auto-approve) → Medium (warn) → High/Critical (explicit approval)

## 24.2: The Resurrector (Crash Recovery) ✅
- Added `checkpoints` table to memory substrate schema
- Implemented `CheckpointStore` (`sk-memory/src/checkpoint.rs`) with save/load/prune/list
- Added periodic checkpoint trigger in `agent_loop.rs` (every 30 seconds)
- Enhanced `Supervisor` with resurrection logic — auto-restarts agents from last checkpoint

## 24.3: Natural Language Builder ✅
- Enhanced `SetupWizard` with `analyze_task_intent()` — LLM-powered task analysis.
- Split roles: **The Witch (Summoner)** plans missions, while **The Builder (Architect)** forges permanent blueprints.
- Added `mode_hint` to `summon_skeleton` (safe/full_access/scheduled).
- Implemented `builder` tool for creating agents from natural language descriptions.

---

# PHASE 25: THE GREAT MERGER (OpenClaw + NemoClaw + Sovereign Kernel) ✅ COMPLETED
> **Status**: v1.0.0 Stable Release.

## 25.1: Rust Port of OpenClaw Core ✅
- Ported 50+ LLM providers to `sk-engine/The Oracle`.
- Ported 30+ channel adapters to `sk-channels/The Bridge`.
- Ported 30+ autonomous Hands to `sk-hands`.

## 25.2: Rust Port of NemoClaw Security ✅
- Ported Landlock/seccomp/network egress to `sk-kernel/The Warden`.
- Implemented `ApprovalManager` for human-in-the-loop gating.

## 25.3: Voice & AI UI Integration ✅
- Implemented `sk-tools/The Voice` with OpenAI STT/TTS.
- Integrated `sk-cli/The Watchtower` with real-time logs and approval queue.
- Verified Merkle audit trail integrity in `sk-kernel/The Ledger`.

---

# PHASE 26: DAY 2 OPERATIONS + ECOSYSTEM ✅ COMPLETED
> **Status**: Sprints 7, 8, and 9 Completed.

## 26.1: MCP Management CLI ✅
- Implemented `sk-cli/src/mcp.rs` for adding/removing Model Context Protocol servers.
- Wired zero-downtime hot-reload API endpoint for dynamic configurations.

## 26.2: Background Daemonization ✅
- Added `--detach` flag to the `sovereign start` command.
- Integrated OS-native sub-process detachment logic across Windows and Unix.

## 26.3: Docker Packaging ✅
- Shipped multi-stage `Dockerfile` relying on distroless Debian runtimes.
- Built a standard `docker-compose.yml` mapped to internal Data/SQL pathways.

---

# 6. TESTING STRATEGY
## Unit Tests
- Per-module strict testing for Sandbox boundary enforcement.
## Integration Tests
- Full end-to-end API, CLI, and workflow test commands (`cargo test --test integration_tests`).
## Cross-Platform Tests
- Platform-specific path handling, shell command, and process management tests.
- ARM emulation tests via QEMU in CI.
## Manual Testing
- Human verification of Approval gates, API WS streaming behavior, and Sandbox execution constraints mapping out paths via Teloxide or Serenity bot interfaces.

---

# 7. DEPLOYMENT & RELEASE
- Binaries shipped from GitHub releases for **all platforms**: Windows (x64), Linux (x64, ARM64, ARMv7), macOS (x64, Apple Silicon).
- Distroless Docker deployment vectors generated through CI pipelines onto Docker Hub.
- Docker-compose clusters spinning up VectorDBs and Postgres backing.
- Raspberry Pi install script: `curl -sSL https://install.sovereign-kernel.dev | sh`.

---

# 8. SUCCESS METRICS

## Overall Success Metric
**User can do this end-to-end on ANY device:**
1. Install Sovereign Kernel (one binary — Windows, Mac, Linux, or Raspberry Pi)
2. Create config (choose Sandbox or Unrestricted)
3. Start: `sovereign chat`
4. Tell the agent: "Monitor news for Company X daily, email me summaries"
5. Close laptop
6. Agent runs autonomously (via cron), browses the web, emails the user, and audits every action onto tamper-proof Merkle structures locally.
7. User views audit trails securely via dashboard.

## Full Computer Access Metric
**Agent can perform ANY task a human can do on the computer:**
1. Read, write, move files on the filesystem
2. Run shell commands and code in any language
3. Browse the web — navigate, click, fill forms, submit
4. See the screen and interact with desktop GUI applications
5. Send emails, search the web, manage processes
6. Remember everything across sessions
7. Work 24/7 on scheduled tasks without human supervision
8. All while respecting the user's security boundaries

**Total effort: ~350-450 hours**
**Ending point: 30 weeks from execution start**
**Final goal: Public release of production-ready Sovereign Kernel — runs on every device, does everything a human can do on a computer, privately and securely.**

