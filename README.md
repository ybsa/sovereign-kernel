# 👑 Sovereign Kernel v1.0

![Rust](https://img.shields.io/badge/language-Rust-orange?style=flat-square)
![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Status](https://img.shields.io/badge/status-v1.0.0--stable-brightgreen?style=flat-square)

> **A local-first, memory-safe, locally audited AI Operating System. Single Rust binary. Runs everywhere.**

Sovereign Kernel is a production-grade Agentic framework built entirely in Rust. It serves as a strict, deeply isolated, and financially metered operating system that governs LLM actions (from over 50+ remote and local models) before they touch your host machine.

```text
┌─────────────────────────────────────────────────────────────┐
│                  Sovereign Kernel (Rust)                    │
│               The Agentic Operating System                  │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ 50+ LLM  │  │ 30+ Chat │  │  106+    │  │ Security │   │
│  │ Providers│  │ Channels │  │  Skills  │  │ Sandbox  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Memory  │  │  Agents  │  │  Tools   │  │ Axum REST│   │
│  │ Substrate│  │  Swarm   │  │ (30+)    │  │  Bridge  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 🏆 Empirically Verified Architecture

Version 1.0 of the Sovereign Kernel has undergone a deep, interior binary trace logic test to mathematically prove its stability. 

1. **The Archive (Memory Substrate):** Every single conversational step is physically serialized natively to an SQLite WAL loop. Semantic embedding searches parse the binary cleanly. No context is ever lost.
2. **The Oracle (LLM Matrix):** Dynamically proven to handle Anthropic, OpenAI, NVIDIA NIM, and Ollama APIs asynchronously. It seamlessly handles fractional token parsing.
3. **The Treasury (Fiscal Engine):** The architecture guarantees budget enforcement natively intercepts and tracks the cost of an LLM query down to the millionth of a cent before routing it back into the Agent.
4. **The Healer (Agent Memory Compaction):** Actively traces oversized context windows. It natively spins up parallel LLM triggers to compress histories and protect your token threshold limits without deleting critical context.
5. **The Forge (Tool Execution Sandbox):** Isolated logic guarantees dangerous Shell/Python/File System scripts are explicitly gated before interacting with OS layers like PTY, `cmd /C`, or `sh`.

---

## 🛡️ "The Warden" Security Core

Because an LLM can generate arbitrary operational commands or Python scripts, the Kernel isolates every Agent. 
- **Filesystem Isolation:** `Landlock LSM` ensures no physical directory can be overwritten by a rogue agent unless explicitly mounted.
- **Syscall Filtering:** `seccomp-bpf` restricts the syscalls accessible to background threads.
- **Merkle Audit:** Every network egress and `stdout` pipe returned by an agentic loop is cryptographically wrapped into a tamper-evident audit trail!

## 🚀 Use Cases

1. **DevOps & Infrastructure Autonomy:** Provide read-only keys to an agent and ask it to diagnose remote Kubernetes registries, using built-in Sandboxed shell tools to generate patching manifests dynamically.
2. **Always-On Threat Intelligence:** Span background daemons via the API Bridge to automatically funnel remote CVE JSONs into the SQLite brain for semantic search correlation.
3. **Autonomous Software Code Reviews:** Hook the `Axum` Rest Bridge up to your Discord, and direct the kernel to summarize thousands of lines of PRs automatically into a channel using highly complex "Skills."

## 📚 Documentation

Everything you need to successfully execute internal Agents or bind external frontends to the OS is fully documented:

1. [🏎️ GETTING STARTED](GETTING_STARTED.md) - How to configure your LLM APIs (`.env`) safely and correctly build the release binary!
2. [🧭 USER GUIDE](USER_GUIDE.md) - Exploring the Headless API Bridge and Tool Sandboxes.
3. [🛡️ SECURITY POLICY](SECURITY.md) - Rules for vulnerability disclosure and "The Warden" boundaries.
4. [🤝 CONTRIBUTING](CONTRIBUTING.md) - How to push PRs, code style choices, and Rust type specifications!

## ⚙️ Requirements
- `Rust 1.75+` (Required)
- `SQLite3` (Required)
- Windows 10+, macOS, Linux (Kernel 5.13+ Recommended)

## 📜 License
Generously licensed under the [MIT License](LICENSE).
