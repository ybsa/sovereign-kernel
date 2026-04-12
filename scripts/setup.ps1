# Sovereign Kernel — Windows Setup Script
# This script ensures Rust is installed and then launches the Sovereign Setup Wizard.

$ErrorActionPreference = "Stop"

# Use standard characters to avoid encoding issues in older PowerShell versions
Write-Host "-------------------------------------------------------" -ForegroundColor Cyan
Write-Host "  Sovereign Kernel - Windows Setup" -ForegroundColor White
Write-Host "-------------------------------------------------------" -ForegroundColor Cyan
Write-Host ""

# 1. Check for Rust
Write-Host "Step 1: Checking for Rust environment..."
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "[!] Rust is not installed." -ForegroundColor Red
    Write-Host "We need Rust to run the Sovereign Kernel. Don't worry, it's easy to install."
    Write-Host "Please download and run the installer from: https://rustup.rs/"
    Write-Host "After installing, please RESTART this terminal and run this script again."
    Write-Host ""
    exit
}
Write-Host "[+] Rust is already installed." -ForegroundColor Green
Write-Host ""

# 2. Build the CLI
Write-Host "Step 2: Preparing the Sovereign Kernel (this might take a few minutes)..."
cargo build --release -p sk-cli
if ($LASTEXITCODE -ne 0) {
    Write-Host "[!] Compilation failed. Please ensure you have the latest Visual Studio C++ Build Tools installed." -ForegroundColor Red
    exit
}
Write-Host "[+] Build complete." -ForegroundColor Green
Write-Host ""

# 3. Launch the Setup Wizard
Write-Host "Step 3: Launching the Setup Wizard..."
Write-Host ""

$EXE = Join-Path $PSScriptRoot "..\target\release\sovereign.exe"
if (-not (Test-Path $EXE)) {
    $EXE = Join-Path $PSScriptRoot "..\target\debug\sovereign.exe"
}

if (Test-Path $EXE) {
    & $EXE setup
    
    Write-Host ""
    Write-Host "-- Setup Complete! ------------------------------------" -ForegroundColor Green
    Write-Host "Next steps:"
    Write-Host "  1. Run './target/release/sovereign chat' to start chatting."
    Write-Host "  2. Run './target/release/sovereign --help' to see all commands."
    Write-Host ""
    Write-Host "Happy hacking!" -ForegroundColor White
} else {
    Write-Host "[!] Could not find sovereign.exe. Please ensure the build completed successfully." -ForegroundColor Red
}
