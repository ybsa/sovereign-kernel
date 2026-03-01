# Autonomous Agents & Hands

Sovereign Kernel supports multiple "Hands" (autonomous routines) that can be run in the background.

## Available Hands

### 1. Researcher
- **Goal:** Deep-dive into complex topics, summarize findings, and generate reports.
- **Tools:** `web_search`, `web_fetch`, `read_file`, `write_file`, `remember`

### 2. Coder
- **Goal:** Write, review, and refactor code.
- **Tools:** `read_file`, `write_file`, `list_dir`, `shell_exec`, `web_search`

### 3. Sysadmin
- **Goal:** Manage local files, monitor system health, and automate routine tasks.
- **Tools:** `shell_exec`, `list_dir`, `read_file`, `write_file`

### 4. Browser
- **Goal:** Automate web interactions (future).
- **Tools:** `web_search`, `web_fetch`, `shell_exec`

### 5. Memory Manager
- **Goal:** Organize and compress long-term memories.
- **Tools:** `recall`, `remember`, `forget`

### 6. Social Media
- **Goal:** Draft and post updates (via channels).
- **Tools:** `web_search`, `remember`

### 7. Analyst
- **Goal:** Process data files and generate insights.
- **Tools:** `read_file`, `write_file`, `shell_exec`

*(Note: Currently, these Hands are metadata-only. Full autonomous execution is planned for a future release.)*
