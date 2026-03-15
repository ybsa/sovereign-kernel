use sk_types::config::KernelConfig;
use serde::Deserialize;
use anyhow::{Result, anyhow};
use std::env;

#[derive(Debug, Deserialize)]
struct ApiAgentEntry {
    id: String,
    name: String,
    #[allow(dead_code)]
    description: String,
    state: String,
    is_running: bool,
    last_active: String,
}

#[derive(Debug, Deserialize)]
struct ActionResponse {
    success: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ThinkingResponse {
    thoughts: Vec<String>,
}

pub async fn run(_config: KernelConfig, action: &str, id: Option<String>) -> Result<()> {
    let api_key = env::var("SOVEREIGN_API_KEY").unwrap_or_default();
    let base_url = "http://127.0.0.1:4242/v1";

    let client = reqwest::Client::new();

    match action {
        "list" => {
            let resp = client.get(format!("{}/agents", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(anyhow!("Failed to list agents: {}", resp.status()));
            }

            let agents: Vec<ApiAgentEntry> = resp.json().await?;
            
            println!("\n🏘️  Sovereign Village Inhabitants:");
            println!(
                "  {:<36} {:<15} {:<10} {:<12} {:<20}",
                "AGENT ID", "NAME", "STATUS", "STATE", "LAST ACTIVE"
            );
            println!(
                "  {:-<36} {:-<15} {:-<10} {:-<12} {:-<20}",
                "", "", "", "", ""
            );

            for agent in agents {
                let status = if agent.is_running { "🟢 RUNNING" } else { "⚪ IDLE" };
                println!(
                    "  {:<36} {:<15} {:<10} {:<12} {:<20}",
                    agent.id,
                    agent.name,
                    status,
                    agent.state,
                    agent.last_active
                );
            }
        }
        "inspect" => {
            let target_id = id.ok_or_else(|| anyhow!("Please provide an Agent ID or Name to inspect"))?;
            
            // Try to resolve name to ID if it's not a UUID
            let final_id = if uuid::Uuid::parse_str(&target_id).is_err() {
                 resolve_name_to_id(&client, base_url, &api_key, &target_id).await?
            } else {
                target_id
            };

            let resp = client.get(format!("{}/agents/{}/thinking", base_url, final_id))
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(anyhow!("Failed to inspect agent: {}", resp.status()));
            }

            let thinking: ThinkingResponse = resp.json().await?;
            println!("\n🧠 Thinking for Agent {}:", final_id);
            if thinking.thoughts.is_empty() {
                println!("  (No recent thoughts found)");
            } else {
                 // Show the last 5 thoughts
                let tail = if thinking.thoughts.len() > 5 {
                    &thinking.thoughts[thinking.thoughts.len()-5..]
                } else {
                    &thinking.thoughts[..]
                };
                for (i, thought) in tail.iter().enumerate() {
                    println!("\n  [Thought {}]:\n  {}", i + 1, thought.replace('\n', "\n  "));
                }
            }
        }
        "stop" => {
            let target_id = id.ok_or_else(|| anyhow!("Please provide an Agent ID or Name to stop"))?;
            
            let final_id = if uuid::Uuid::parse_str(&target_id).is_err() {
                 resolve_name_to_id(&client, base_url, &api_key, &target_id).await?
            } else {
                target_id
            };

            let resp = client.post(format!("{}/agents/{}", base_url, final_id))
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            let result: ActionResponse = resp.json().await?;
            if result.success {
                println!("✅ Agent {} stopped.", final_id);
            } else {
                println!("❌ Failed to stop agent: {}", result.message);
            }
        }
        "remove" => {
            let target_id = id.ok_or_else(|| anyhow!("Please provide an Agent ID or Name to remove"))?;
            
            let final_id = if uuid::Uuid::parse_str(&target_id).is_err() {
                 resolve_name_to_id(&client, base_url, &api_key, &target_id).await?
            } else {
                target_id
            };

            let resp = client.delete(format!("{}/agents/{}", base_url, final_id))
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            let result: ActionResponse = resp.json().await?;
            if result.success {
                println!("🗑️ Agent {} removed from Village.", final_id);
            } else {
                println!("❌ Failed to remove agent: {}", result.message);
            }
        }
        _ => {
            println!("Unknown action: {}. Available: list, inspect, stop, remove", action);
        }
    }

    Ok(())
}

async fn resolve_name_to_id(client: &reqwest::Client, base_url: &str, api_key: &str, name: &str) -> Result<String> {
    let resp = client.get(format!("{}/agents", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!("Failed to resolve name: {}", resp.status()));
    }

    let agents: Vec<ApiAgentEntry> = resp.json().await?;
    for agent in agents {
        if agent.name == name {
            return Ok(agent.id);
        }
    }

    Err(anyhow!("Agent named '{}' not found", name))
}
