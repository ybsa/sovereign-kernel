#!/bin/bash
# Sovereign Kernel — Unix/Mac Setup Script
# This script ensures Rust is installed and then launches the Sovereign Setup Wizard.

set -e

echo -e "\033[0;36m═══════════════════════════════════════════════════════\033[0m"
echo -e "\033[1;37m  ⚡ Sovereign Kernel — Unix/Mac Setup\033[0m"
echo -e "\033[0;36m═══════════════════════════════════════════════════════\n\033[0m"

# 1. Check for Rust
echo "Step 1: Checking for Rust environment..."
if ! command -v cargo &> /dev/null; then
    echo -e "\033[0;31m❌ Rust is not installed.\033[0m"
    echo "We need Rust to run the Sovereign Kernel. Don't worry, it's easy to install."
    echo "Please run the following command to install Rust:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo -e "After installing, please RESTART your terminal and run this script again.\n"
    exit 1
fi
echo -e "\033[0;32m✅ Rust is already installed.\n\033[0m"

# 2. Build the CLI
echo "Step 2: Preparing the Sovereign Kernel (this might take a few minutes)..."
cargo build --release -p sk-cli
echo -e "\033[0;32m✅ Build complete.\n\033[0m"

# 3. Launch the Setup Wizard
echo -e "Step 3: Launching the Setup Wizard...\n"
if [ -f "target/release/sovereign" ]; then
    ./target/release/sovereign setup
else
    ./target/debug/sovereign setup
fi
