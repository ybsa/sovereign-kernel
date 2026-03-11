# Sovereign Kernel: Security & Privacy Policy

Sovereign Kernel is designed as a "Security-First" AI Operating System. This document outlines the security architecture, data handling practices, and sandboxing mechanisms.

## Core Security Principles

1.  **Deny-by-Default Capabilities**: Agents have zero access to the host system by default. Every action (File Read, Network, Shell) requires an explicit `Capability` grant in the agent's manifest.
2.  **Sandbox Isolation**: Sovereign Kernel uses multiple layers of sandboxing:
    -   **WebAssembly (Wasmtime)**: Guest skills run in a memory-isolated WASM runtime with no direct syscall access.
    -   **Docker Sandbox**: High-risk operations (like web browsing or complex tool execution) are isolated in ephemeral Docker containers.
    -   **Workspace Root**: File operations are restricted to a specific `/workspace` directory, preventing escape to the host's sensitive system files.
3.  **Shell Bleed Protection**: Every script executed by an agent is scanned for "Environment Bleed" — ensuring potential secrets or API keys in the host environment aren't accidentally leaked to the LLM.
4.  **SSRF Protection**: Outbound network requests are intercepted. DNS resolution is checked against private IP ranges (localhost, 169.254.169.254, etc.) to prevent Server-Side Request Forgery.

## Data Privacy

-   **Zero Telemetry**: Sovereign Kernel does not "phone home." No data about your tasks, prompts, or files is sent to any centralized server owned by the developers.
-   **User-Controlled Risk**: Users can toggle between `Sandbox` and `Unrestricted` modes. In Sandbox mode, the kernel acts as a strict guard.
-   **Audit Trails**: Every action an agent takes is recorded in a tamper-evident audit log, allowing for full post-mortem analysis of agent behavior.

## Responsible Use

While Sovereign Kernel provides powerful tools for automation, it is the user's responsibility to manage the risk levels of the agents they deploy. Always review an agent's required capabilities before granting them.
