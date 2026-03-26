# Getting Started with Sovereign Kernel 🚀

Welcome to the **Village**. This guide will get you up and running in 2 minutes.

## 1. What is Sovereign Kernel?

Sovereign Kernel is a **local-first AI operating system** built in Rust. Unlike simple chatbots, it is a **single-binary kernel** that manages autonomous agents ("Village members") that can use tools, browse the web, run code, and recover from crashes.

## 2. Why use it?

- **Total Privacy**: Everything runs on your machine; results are stored in a local SQLite "Archive".
- **Hardened Security**: Agents run in a [Landlock/seccomp](docs/SAFETY_CONTROLS.md) sandbox (The Warden).
- **Proactive Autonomy**: It doesn't just talk; it **does**. It can schedule tasks, browse, and build its own tools.
- **Expert Knowledge**: Swappable "Hands" and 106+ expert "Skills" for coding, security, research, and more.

## 3. Quick Setup

### Option A: Download Pre-built Binaries (Easiest)

1. Go to the [Latest Release](https://github.com/ybsa/sovereign-kernel/releases/latest) page.
2. Download the archive for your OS (`windows-latest.zip`, `ubuntu-latest.tar.gz`, or `macos-latest.tar.gz`).
3. Extract the archive to a folder of your choice.
4. Open a terminal in that folder.

### Option B: Build from Source

```bash
git clone https://github.com/ybsa/sovereign-kernel.git
cd sovereign-kernel
cargo build --release
```

## 4. Initialization

Regardless of how you got the binary, you must initialize the kernel:

```bash
# On Windows
.\sovereign.exe init

# On Linux/macOS
./sovereign init
```

### Add API Keys

Create a `.env` file in the same folder as the binary with at least one key:

```env
ANTHROPIC_API_KEY=your_key_here
# OR
OPENAI_API_KEY=your_key_here
```

## 5. How to Use

| Goal | Command |
| --- | --- |
| **Interactive Chat** | `sovereign chat` |
| **One-shot Task** | `sovereign run "Analyze this repo and find bugs"` |
| **Background Mode** | `sovereign start --detach` |
| **Web UI** | `sovereign dashboard` |
| **Health Check** | `sovereign doctor` |

## 📚 Deep Dive

- [Documentation Index](README.md#documentation)
- [How it works (Architecture)](docs/ARCHITECTURE.md)
- [Safety & Sandboxing](docs/SAFETY_CONTROLS.md)
