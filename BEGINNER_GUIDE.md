# ⚡ Sovereign Kernel: Zero-to-Hero Guide 🚀

Welcome! This guide is for you if you've never used AI agents or code before. Let's get your own autonomous assistant running in **3 simple steps**.

---

## 🏁 Step 0: Prerequisites
You only need one thing installed on your computer:
*   **Rust**: The engine that powers Sovereign.
    *   **Download it here**: [rustup.rs](https://rustup.rs/)
    *   *Just follow the default instructions (press 1 if asked).*

---

## 🛠️ Step 1: Automatic Setup
Once Rust is installed, open your **Terminal** (Command Prompt or PowerShell on Windows) and paste these commands one by one:

### 1. Download Sovereign
```powershell
git clone https://github.com/OpenEris/sovereign-kernel.git
cd sovereign-kernel
```

### 2. Run the Setup Tool
```powershell
./scripts/setup.ps1
```
*(On Mac/Linux, use `bash ./scripts/setup.sh` instead)*

---

## 🔑 Step 2: Keys & Soul
When the setup script finishes, it will launch the **Sovereign Setup Wizard**. 

1.  **Choose your AI**: We recommend **NVIDIA NIM** or **OpenAI**. 
2.  **Add your API Key**: Paste the key you got from your provider.
3.  **Choose a "Soul"**: This is your agent's personality! You can pick "The Coder," "The Researcher," or create your own.

> [!TIP]
> **Need an API Key?**
> *   [Get an OpenAI key here](https://platform.openai.com/)
> *   [Get an NVIDIA key here](https://build.nvidia.com/explore/discover) (Free credits often available!)

---

## 🤖 Step 3: Talking to your Agent
Now the fun part! You can talk to your agent in two ways:

### A. The Chat Room
Type this to start a conversation:
```powershell
sovereign chat
```

### B. Giving a Job
Want the agent to actually **do** something? Give it a "One-Shot" task:
```powershell
sovereign run "Find the latest news on SpaceX and write a summary to a file called spacex.txt"
```

---

## 🏥 Troubleshooting
Is it not working? Just type:
```powershell
sovereign doctor
```
The "Doctor" will check your system and tell you exactly what is missing!

---

## 🌟 Pro-Tips
*   **Approval**: By default, the agent will ask for your permission before doing something "risky" like deleting a file. Just type **"yes"** in the chat to let it proceed!
*   **Souls**: You can create many different agents with different skills using `sovereign soul create`.

**Congratulations! You are now running an Agentic OS.** 🚀
