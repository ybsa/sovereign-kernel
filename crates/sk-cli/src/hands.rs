//! Hands management — list, activate, deactivate autonomous capability packages.

use sk_hands::registry::HandRegistry;
use sk_types::config::KernelConfig;
use sk_kernel::wizard::SetupWizard;
use sk_kernel::SovereignKernel;

/// Run the hands subcommand.
pub async fn run(config: KernelConfig, action: &str, args: &[String]) -> anyhow::Result<()> {
    let mut registry = HandRegistry::new();
    let loaded_bundled = registry.load_bundled();
    
    let custom_hands_dir = config.data_dir.join("hands");
    if !custom_hands_dir.exists() {
        let _ = std::fs::create_dir_all(&custom_hands_dir);
    }
    let loaded_custom = registry.load_custom_hands(&custom_hands_dir);
    let total_loaded = loaded_bundled + loaded_custom;

    match action {
        "list" => {
            println!("⚡ Sovereign Kernel — Autonomous Hands");
            println!("══════════════════════════════════════════");
            println!();

            let defs = registry.list_definitions();
            if defs.is_empty() {
                println!("  No hands available.");
            } else {
                println!("  {} hand(s) loaded ({} bundled, {} custom):\n", total_loaded, loaded_bundled, loaded_custom);
                for def in &defs {
                    let status_icon = "⬚"; // inactive by default
                    println!("  {} {} — {}", status_icon, def.name, def.description);
                    println!(
                        "    Category: {} | Version: {}",
                        def.category, def.agent.model
                    );

                    if !def.requires.is_empty() {
                        if let Ok(reqs) = registry.check_requirements(&def.id) {
                            println!("    Requirements:");
                            for (req, satisfied) in &reqs {
                                let icon = if *satisfied { "✅" } else { "❌" };
                                let desc = req.description.as_deref().unwrap_or("");
                                println!("      {} {} {}", icon, req.label, desc);
                            }
                        }
                    }
                    println!();
                }
            }
        }
        "activate" => {
            if args.is_empty() {
                println!("Usage: sovereign hands activate <hand-name>");
                return Ok(());
            }
            let hand_name = &args[0];

            // Find the hand by name
            let defs = registry.list_definitions();
            let matching = defs
                .iter()
                .find(|d| d.id == *hand_name || d.name.to_lowercase() == hand_name.to_lowercase());

            match matching {
                Some(def) => {
                    let config = std::collections::HashMap::new();
                    match registry.activate(&def.id, config) {
                        Ok(instance) => {
                            println!(
                                "⚡ Hand '{}' activated! Instance: {}",
                                def.name, instance.instance_id
                            );
                            println!("   The hand's agent will process tasks autonomously.");
                        }
                        Err(e) => {
                            println!("❌ Failed to activate '{}': {}", hand_name, e);
                        }
                    }
                }
                None => {
                    println!("❌ Hand '{}' not found. Run `sovereign hands list` to see available hands.", hand_name);
                }
            }
        }
        "deactivate" => {
            if args.is_empty() {
                println!("Usage: sovereign hands deactivate <instance-id>");
                return Ok(());
            }
            let id_str = &args[0];
            match uuid::Uuid::parse_str(id_str) {
                Ok(id) => match registry.deactivate(id) {
                    Ok(instance) => {
                        println!("⚡ Hand instance '{}' deactivated.", instance.instance_id);
                    }
                    Err(e) => {
                        println!("❌ Failed to deactivate: {}", e);
                    }
                },
                Err(_) => {
                    println!("❌ Invalid instance ID. Use a valid UUID.");
                }
            }
        }
        "forge" => {
            if args.is_empty() {
                println!("Usage: sovereign hands forge \"description of the hand\"");
                return Ok(());
            }
            let description = args.join(" ");
            println!("⚡ Forging new Hand from description: \"{}\"...", description);

            // Initialize kernel to get the driver
            let kernel = SovereignKernel::init(config).await?;
            let driver = kernel.driver.clone();
            let model = kernel.model_name.clone();
            
            match SetupWizard::analyze_task_intent(driver, &model, &description).await {
                Ok(intent) => {
                    let hand_def = SetupWizard::intent_to_hand(&intent);
                    let toml_content = SetupWizard::export_hand(&hand_def)?;
                    
                    let filename = format!("{}.toml", hand_def.id);
                    let dest_path = custom_hands_dir.join(&filename);
                    
                    std::fs::write(&dest_path, toml_content)?;
                    
                    println!("✅ Hand forged successfully!");
                    println!("   Name: {}", hand_def.name);
                    println!("   ID:   {}", hand_def.id);
                    println!("   Path: {}", dest_path.display());
                    println!("\nRun `sovereign hands list` to see your new capability.");
                }
                Err(e) => {
                    println!("❌ Forge failed: {}", e);
                }
            }
        }
        "status" => {
            let instances = registry.list_instances();
            if instances.is_empty() {
                println!("⚡ No active hand instances.");
            } else {
                println!("⚡ Active Hand Instances:");
                println!("══════════════════════════════════════════");
                for inst in &instances {
                    println!(
                        "  {} — {} [{}]",
                        inst.instance_id, inst.hand_id, inst.status
                    );
                }
            }
        }
        _ => {
            println!("⚡ Sovereign Kernel — Hands");
            println!();
            println!("Usage:");
            println!("  sovereign hands list         — show available hands");
            println!("  sovereign hands forge <desc> — create a new hand from natural language");
            println!("  sovereign hands activate <n> — activate a hand");
            println!("  sovereign hands deactivate <id> — deactivate");
            println!("  sovereign hands status       — show active instances");
        }
    }

    Ok(())
}
