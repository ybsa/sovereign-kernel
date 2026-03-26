# Contributing to Sovereign Kernel

Sovereign Kernel is an open-source, global infrastructure layer for Agentic AI. We welcome contributions from developers across the world to build the "More Best" Agentic OS.

## 🌍 The Sovereign Vision

This project isn't just a personal assistant for one user; it is a universal standard. We prioritize:
1.  **Portability**: It must compile to a single, easily distributable binary.
2.  **Privacy**: Local inference and native execution are first-class citizens.
3.  **Speed**: Background idle CPU/RAM usage should remain as close to zero as possible.

## 🔌 Building MCP Connectors & Skills

The "Nervous System" of the Sovereign Kernel runs on the Model Context Protocol (MCP) and a massive library of Expert Skills.

To add a new capability or skill:
1.  **Skills**: Add a new `SKILL.md` inside a folder in `crates/sk-tools/skills/` following the established OpenClaw port format.
2.  **MCP Connectors**: Create a new file in `crates/sk-mcp/src/connectors/your_connector.rs` and register it in `sk-mcp/src/server.rs`.
3.  **Hands**: To bundle a completely new autonomous agent experience, duplicate an existing Hand in `crates/sk-hands/src/hands/` and register it in `crates/sk-hands/src/registry.rs`.

## 👨‍💻 General Guidelines

*   Ensure all code compiles with `cargo check --workspace` and `cargo clippy --workspace` with absolutely zero warnings.
*   Try to avoid adding heavy, sprawling dependencies unless strictly necessary.
*   Document your public traits and modules.

We can't wait to see what you build.
