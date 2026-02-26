# Contributing to Sovereign Kernel

Sovereign Kernel is an open-source, global infrastructure layer for Agentic AI. We welcome contributions from developers across the world to build the "More Best" Agentic OS.

## 🌍 The Sovereign Vision
This project isn't just a personal assistant for one user; it is a universal standard. We prioritize:
1.  **Portability**: It must compile to a single, easily distributable binary.
2.  **Privacy**: Local inference and native execution are first-class citizens.
3.  **Speed**: Background idle CPU/RAM usage should remain as close to zero as possible.

## 🔌 Building MCP Connectors
The "Nervous System" of the Sovereign Kernel runs on the Model Context Protocol (MCP). The best way to contribute is by adding new native connectors!

To add a new capability to the OS:
1.  Create a new file in `crates/sk-mcp/src/connectors/your_connector.rs`.
2.  Implement an async Rust function that performs your desired logic.
3.  Add the JSON schema definition for your tool to the `McpServer` registry in `sk-mcp/src/server.rs`.
4.  The Sovereign Kernel will automatically discover your tool on boot and seamlessly integrate it into the `sk-engine` Dynamic LLM Router.

## 👨‍💻 General Guidelines
*   Ensure all code compiles with `cargo check --workspace` and `cargo clippy --workspace` with absolutely zero warnings.
*   Try to avoid adding heavy, sprawling dependencies unless strictly necessary.
*   Document your public traits and modules.

We can't wait to see what you build.
