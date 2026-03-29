# Contributing to Sovereign Kernel

First off, thank you for considering contributing to Sovereign Kernel!
It's people like you that make this Agentic OS powerful, memory-safe, and truly autonomous.

## How Can I Contribute?

### 1. Reporting Bugs

- Check the issues tab to see if your bug is already reported.
- If not, open a new issue. Include your `SOVEREIGN_VERSION` (run `sovereign --version`), OS details, log output (`cli.log.*`), and steps to reproduce.

### 2. Suggesting Enhancements / Core Features

- For new Tools, Drivers, or major OS core features (like new LSM integrations), please open an architecture discussion in the Issues tab before writing huge patches.
- Detail the exact payload, LLM schema, and security implications of your idea.

### 3. Pull Requests

- Fork the repo and create your branch from `main`.
- If you've added code that should be tested, add tests! (e.g. `cargo test --workspace`)
- Ensure the test suite passes and your code lints cleanly with `cargo clippy --workspace --all-targets -- -D warnings`.
- Format your code using `cargo fmt`.

## Code Architecture

When writing Rust for Sovereign Kernel, please match the prevailing style:

- Use `SovereignResult` and `SovereignError` for all fallible operations.
- For agent instructions, use the `sk_types::MessageContent` enum properly (`MessageContent::Text()` wrapping).
- No blocking operations in async contexts! Use `tokio::task::spawn_blocking` or `.read().await` correctly.

## Setting up your Development Environment

```bash
# Clone the repository
git clone https://github.com/your-org/sovereign-kernel
cd sovereign-kernel

# Build the workspace
cargo build

# Run the test suite
cargo test --workspace
```

Welcome to the Village!
