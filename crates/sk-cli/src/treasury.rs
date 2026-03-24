//! Treasury command — manage budgets and track LLM costs.

use clap::{Parser, Subcommand};
use sk_types::SovereignResult;
use colored::Colorize;

/// Manage budgets and track LLM costs.
#[derive(Parser, Debug)]
pub struct TreasuryArgs {
    #[clap(subcommand)]
    pub command: TreasuryCommands,
}

#[derive(Subcommand, Debug)]
pub enum TreasuryCommands {
    /// Show current spend and budget status.
    Status,
    /// Show detailed spend per agent.
    Report,
    /// Reset all cost tracking.
    Reset,
}

/// Run the treasury command.
pub async fn run(args: TreasuryArgs, api_url: &str, api_key: Option<&str>) -> SovereignResult<()> {
    match args.command {
        TreasuryCommands::Status => show_status(api_url, api_key).await,
        TreasuryCommands::Report => show_report(api_url, api_key).await,
        TreasuryCommands::Reset => reset_treasury(api_url, api_key).await,
    }
}

async fn show_status(api_url: &str, api_key: Option<&str>) -> SovereignResult<()> {
    let client = reqwest::Client::new();
    let mut request = client.get(format!("{}/v1/treasury/status", api_url));
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }

    let response = request.send().await.map_err(|e| sk_types::SovereignError::Network(e.to_string()))?;
    
    if !response.status().is_success() {
        println!("{} Failed to fetch treasury status: {}", "Error:".red(), response.status());
        return Ok(());
    }

    let status: serde_json::Value = response.json().await.map_err(|e| sk_types::SovereignError::Internal(e.to_string()))?;

    println!("\n{}", "── THE TREASURY: BUDGET STATUS ──".bold().cyan());
    
    let total = status["current_spend"].as_f64().unwrap_or(0.0);
    println!("Total Session Spend: ${}", format!("{:.4}", total).yellow());

    print_window("Hourly", status["hourly_spend"].as_f64(), status["hourly_limit"].as_f64(), status["hourly_pct"].as_f64());
    print_window("Daily", status["daily_spend"].as_f64(), status["daily_limit"].as_f64(), status["daily_pct"].as_f64());
    print_window("Monthly", status["monthly_spend"].as_f64(), status["monthly_limit"].as_f64(), status["monthly_pct"].as_f64());

    println!();
    Ok(())
}

fn print_window(name: &str, spend: Option<f64>, limit: Option<f64>, pct: Option<f64>) {
    let spend = spend.unwrap_or(0.0);
    let limit = limit.unwrap_or(0.0);
    let pct = pct.unwrap_or(0.0) * 100.0;

    let bar_len = 20;
    let filled = ((pct / 100.0) * bar_len as f64).round() as usize;
    let filled = filled.min(bar_len);
    let empty = bar_len - filled;
    
    let bar_color = if pct > 90.0 { "red" } else if pct > 70.0 { "yellow" } else { "green" };
    let bar = format!("{}{}", "█".repeat(filled).color(bar_color), "░".repeat(empty).dimmed());

    if limit > 0.0 {
        println!("{:<10} [{}] {:>5.1}% (${:.2} / ${:.2})", name, bar, pct, spend, limit);
    } else {
        println!("{:<10} [Uncapped] ${:.4}", name, spend);
    }
}

async fn show_report(_api_url: &str, _api_key: Option<&str>) -> SovereignResult<()> {
    // Similar to status but maybe lists agents if available
    println!("Detailed per-agent report coming soon...");
    Ok(())
}

async fn reset_treasury(api_url: &str, api_key: Option<&str>) -> SovereignResult<()> {
    let client = reqwest::Client::new();
    let mut request = client.post(format!("{}/v1/treasury/reset", api_url));
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }

    let response = request.send().await.map_err(|e| sk_types::SovereignError::Network(e.to_string()))?;
    if response.status().is_success() {
        println!("{} Treasury costs have been reset.", "Success:".green());
    } else {
        println!("{} Failed to reset treasury.", "Error:".red());
    }
    Ok(())
}
