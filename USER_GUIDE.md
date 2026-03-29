# 🗺️ Sovereign Kernel User Guide

This guide details how to interact with the Sovereign Kernel at a deep structural level. Now that Sovereign Kernel is v1.0 and fully verified, you can interface with it directly using its headless REST API or the CLI.

## 1. Using The REST API Bridge

If you launched Sovereign Kernel in daemon mode (`cargo run --release -- start --detach`), it will run a lightweight `Axum` HTTP server in the background (default port `3030`).

You can easily route queries to the active LLM memory pool using standard JSON payloads.

**Example Request:**
```bash
curl -X POST http://127.0.0.1:3030/v1/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Hello Oracle, what is the status of the Treasury?"
  }'
```

The server automatically negotiates the token constraints and maps the response onto the Event Bus natively!

## 2. Using "The Forge" (Tool Execution)

By default, Sovereign Kernel operates in a strict sandbox. If you explicitly want an agent to modify files on your computer or run a shell command, the agent must be granted the `shell_exec` or `code_exec` capability.

- If you interact via the CLI (`cargo run -- run "Do X"`), the CLI will pause and prompt you: "The agent wants to execute: 'ls -la'. Approve? [y/N]"
- This is part of **The Warden** security protocol preventing autonomous agents from maliciously overwriting directories.

## 3. Monitoring The Treasury 💰

Sovereign Kernel natively tracks every fractional cent consumed by your API keys (Anthropic, NVIDIA, OpenAI).

To view your exact lifetime usage without parsing logs, hit the Treasury API:
```bash
curl http://127.0.0.1:3030/v1/treasury/status
```
It returns an instantaneous JSON read-out of your `total_cents_used` against your configured maximum budget.

## 4. Troubleshooting 

**Context Overflow?**
If you pass too many tokens into a long-running session, Sovereign Kernel features "The Healer." It will seamlessly activate a background LLM pipeline to explicitly summarize the last 50 messages, drastically shrinking your token count without destroying context.

**Database Drift?**
If you want to view exactly what a specific agent remembers, simply use standard SQLite tools to view `memory.db` generated inside the kernel's execution directory.
