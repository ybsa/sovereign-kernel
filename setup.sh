#!/bin/bash

# Sovereign Kernel - Quick Setup Script for Non-Developers

echo "================================================="
echo "👑 Welcome to Sovereign Kernel Quick Setup 👑"
echo "================================================="
echo ""

# 1. Check for Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Sovereign Kernel requires Rust."
    echo "Please install Rust by running the following command:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "After installation, restart your terminal and run this script again."
    exit 1
fi
echo "✅ Rust is installed."

# 2. Setup Configuration Files
echo ""
echo "Let's set up your API keys."
echo "Which AI provider would you like to use?"
echo "1) Anthropic (Claude) - Recommended"
echo "2) OpenAI (ChatGPT)"
echo "3) Google Gemini"
echo "4) Groq (Fast Inference)"
echo "5) Ollama (Local, no API key needed)"
read -p "Enter the number of your choice (1-5): " provider_choice

PROVIDER=""
ENV_VAR_NAME=""

case $provider_choice in
    1)
        PROVIDER="anthropic"
        ENV_VAR_NAME="ANTHROPIC_API_KEY"
        ;;
    2)
        PROVIDER="openai"
        ENV_VAR_NAME="OPENAI_API_KEY"
        ;;
    3)
        PROVIDER="gemini"
        ENV_VAR_NAME="GEMINI_API_KEY"
        ;;
    4)
        PROVIDER="groq"
        ENV_VAR_NAME="GROQ_API_KEY"
        ;;
    5)
        PROVIDER="ollama"
        ENV_VAR_NAME=""
        ;;
    *)
        echo "❌ Invalid choice. Defaulting to Anthropic."
        PROVIDER="anthropic"
        ENV_VAR_NAME="ANTHROPIC_API_KEY"
        ;;
esac

echo ""
# Ask for API key if not using local Ollama
if [ "$PROVIDER" != "ollama" ]; then
    read -p "Please paste your $PROVIDER API key: " API_KEY

    # Create or update .env file
    if [ ! -f .env ]; then
        if [ -f .env.example ]; then
            cp .env.example .env
        else
            touch .env
        fi
    fi

    # Check if the key already exists and replace it, otherwise append
    if grep -q "^${ENV_VAR_NAME}=" .env; then
        # Use sed to replace the existing key (works on both Mac and Linux)
        sed -i.bak "s|^${ENV_VAR_NAME}=.*|${ENV_VAR_NAME}=${API_KEY}|" .env
        rm -f .env.bak
    else
        echo "${ENV_VAR_NAME}=${API_KEY}" >> .env
    fi
    echo "✅ API key saved to .env file (kept secret)."
else
    echo "✅ Local Ollama selected. No API key required."
    # Ensure Ollama is running
    echo "⚠️ Please make sure the Ollama app is running on your computer."
fi

# Create config.toml
echo ""
echo "Creating config.toml..."
cat > config.toml << EOF
log_level = "info"
execution_mode = "sandbox"

[[llm]]
provider = "$PROVIDER"
api_key_env = "$ENV_VAR_NAME"
EOF

echo "✅ Configuration files created successfully!"
echo ""

# 3. Build and Run
echo "================================================="
echo "Building the kernel (this might take a few minutes the first time)..."
echo "================================================="

# Compile the project
cargo build --release --workspace

if [ $? -eq 0 ]; then
    echo ""
    echo "🎉 Build successful! Starting the interactive chat..."
    echo "You can exit the chat anytime by typing 'exit' or pressing Ctrl+C."
    echo "-------------------------------------------------"

    # Run the chat interface
    cargo run --release -- chat
else
    echo "❌ Build failed. Please check the errors above."
    exit 1
fi
