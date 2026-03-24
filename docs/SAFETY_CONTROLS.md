# Safety & Budgeting Controls

Sovereign Kernel includes a comprehensive safety system designed to prevent runaway costs, infinite loops, and accidental destructive actions. This layer acts as a financial and operational "circuit breaker" for autonomous agents.

## Core Safety Mechanisms

### 1. Hard Reasoning Limits
To prevent agents from getting stuck in infinite loops (hallucinating or repeatedly failing at the same step), the kernel enforces computational bounds. By default, these are **unlimited** (or bound only by the LLM context window), but can be explicitly constrained:
- **Max Iterations**: `--max-iterations` (Default: Unlimited)
- **Max Tokens**: `--max-tokens` (Default: Unlimited)
- **Persona Guidance**: The Witch (Summoner) and Builder (Architect) are programmed to respect these limits when planning missions.

If an agent exceeds these limits, the task is immediately aborted with a `LoopLimitExceeded` error, preventing further token burn.

### 2. Global USD Budget Cap
You can set a global "kill switch" for the entire kernel across all agents:
- **Config**: `total_token_budget_usd_cents`
- **Behavior**: Once the accumulated cost of all agents reaches this cap (e.g., $10.00), all further LLM calls are blocked until the kernel is restarted or the budget is reset.

### 3. Strict Approval Gating
Even in `Unrestricted` mode, the kernel enforces a safety gate for "Dangerous" tools:
- **High/Critical Risk**: Tools like `shell_exec` (with destructive commands), `delete_file`, or `browser_automation` always require human approval by default.
- **Whitelist Override**: You can explicitly whitelist specific tool names or command patterns (e.g., `ls`, `git status`) to bypass the approval queue.

### 4. Forensic Step Dumping
For transparency and post-mortem analysis, the kernel can record every step:
- **JSONL Format**: Every prompt, assistant response, and tool result is saved in `.steps/[session_id]/step_N.jsonl`.
- **API Key Redaction**: Common secrets (OpenAI, Anthropic, AWS, etc.) are automatically scrubbed from the forensic logs before they hit the disk.
- **Summary Reports**: A `summary.json` is generated at the end of each session containing final token counts, cost estimation, and status.

## Configuration Example

Add this to your `kernel.toml` (usually in `~/.Sovereign Kernel/config/`):

```toml
[safety]
max_iterations_per_task = 30          # local override
max_tokens_per_task = 500_000         # local override
total_token_budget_usd_cents = 500    # $5.00 global cap
step_dump_enabled = true
approval_whitelist = [
  "shell_exec:ls",                    # allow 'ls' always
  "shell_exec:git status",            # allow 'git status'
  "read_file"                         # allow all read_file calls
]
```

## CLI Usage

You can override these safety settings on the fly for any execution:

```bash
# Chat with detailed step logging enabled
sovereign chat --step-dump --budget-usd 0.05

# Run a task with explicit iteration limits
sovereign run "Refactor the whole project" --max-iterations 100 --max-tokens 500000

# Start the daemon with a strict $2 budget
sovereign start --budget-cents 200
```

## Risk Classification

The kernel classifies actions into three tiers:
- **Low**: Read-only operations (list files, current directory).
- **Medium**: Modifications to non-critical files (logs a warning).
- **High/Critical**: System changes, network access, or destructive commands (requires approval). PEKA (The Terminal Master) always operates in a High-visibility mode due to his deep shell access.

For more on the security architecture, see [SECURITY.md](SECURITY.md).
