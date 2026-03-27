# 🗺️ Step-by-Step Setup Guide (Windows)

Welcome! This guide will teach you exactly how to set up the Sovereign Kernel on your computer using the pre-built Windows version.

## Phase 1: Download & Extract

1.  **Go to the Releases Page**: Visit [github.com/ybsa/sovereign-kernel/releases/latest](https://github.com/ybsa/sovereign-kernel/releases/latest).
2.  **Download the Zip**: Click on `sovereign-kernel-windows-latest.zip`.
3.  **Extract**: Right-click the downloaded file and select **"Extract All..."**. Choose a folder like `C:\SovereignKernel`.

## Phase 2: Adding Your API Keys (The "Brain")

The Kernel needs an API key to "think". You can get one from [Anthropic (Claude)](https://console.anthropic.com/) or [OpenAI (ChatGPT)](https://platform.openai.com/).

1.  Inside your `SovereignKernel` folder, create a new text file.
2.  Rename it exactly to `.env` (make sure there is a dot at the beginning and no `.txt` at the end).
3.  Open it with Notepad and paste your key like this:

    ```env
    ANTHROPIC_API_KEY=your_key_here
    ```

4.  Save and close.

## Phase 3: First-Time Initialization

1.  Press `Win + R`, type `powershell`, and hit Enter.
2.  Type `cd C:\SovereignKernel` (or wherever you extracted it).
3.  Run the setup wizard:
    ```powershell
    .\sovereign.exe init
    ```
4.  **Follow the Prompts**:
    - Pick your LLM provider (e.g., Anthropic).
    - The wizard will check your key and create your configuration file.

## Phase 4: Testing Your First Agent

Now for the fun part! Let's talk to your agent.

### Option A: Interactive Chat
Run this to enter a direct conversation:
```powershell
.\sovereign.exe chat
```

### Option B: Autonomous Task
Ask the agent to do something on your computer:
```powershell
.\sovereign.exe run "Check my disk space and summarize it"
```

## 💡 Pro Tips for New Users

- **The Dashboard**: Run `.\sovereign.exe dashboard` to open a beautiful web interface in your browser.
- **Safety**: By default, the agent is in **Sandbox Mode**. It will ask for your permission before editing files or running commands.
- **Help**: Type `.\sovereign.exe --help` at any time to see all available commands.

---

*Congratulations! You are now running your own sovereign agent village locally.* 🚀
