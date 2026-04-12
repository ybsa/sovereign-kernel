# ⚡ Sovereign Kernel — Easy Start Guide

Welcome to the Sovereign Kernel! This guide is designed for **non-developers** who want to get their own autonomous AI agents running in minutes.

## 1. Quick Install (One Command)

Run the setup script to install dependencies and configure your AI engine:

### Windows (PowerShell)
```powershell
./scripts/setup.ps1
```

### Mac / Linux
```bash
bash ./scripts/setup.sh
```

This will automatically check if you have Rust installed and launch the **Sovereign Setup Wizard**.

---

## 2. Using the Setup Wizard

When you run the command above, you'll see a menu. Here's what to do:

1.  **Select Provider**: Use your **Arrow Keys** to choose a provider. We recommend **NVIDIA NIM** for the best balance of speed and intelligence.
2.  **Paste your Key**: When asked for an API Key, simply paste it and press Enter.
    - *Tip: Don't worry if you paste the whole "Bearer" header; the wizard will automatically clean it up for you!*
3.  **Choose a Soul**: The wizard will ask if you want to create a persona (Soul). Say **Yes** to give your agent a specific job (e.g., "The Coder" or "Personal Assistant").

---

## 3. Creating & Spawning Agents

The Sovereign Kernel is all about **Souls**. A Soul is a personality and a set of rules for your AI.

### To create a new Soul:
```bash
sovereign soul create
```
Follow the prompts to define how your agent should act. 

### To spawn an agent for a specific task:
Once you have a soul, you can instantly give it a job:
```bash
sovereign soul spawn --soul "Researcher" --task "Find the latest news on AI safety"
```

---

## 4. Key Commands to Know

| Command | What it does |
| :--- | :--- |
| `sovereign chat` | Start a friendly conversation with the kernel. |
| `sovereign setup` | Re-run the setup wizard at any time. |
| `sovereign status` | Check if your agents are running ok. |
| `sovereign doctor` | Run a health check to make sure everything is working. |

---

## 👩‍💻 Need Help?
If something isn't working, run `sovereign doctor` to find out why. The most common issue is a missing API key or an expired one!

**Happy Hacking!** 🚀
