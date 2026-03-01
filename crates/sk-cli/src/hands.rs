//! Hands management — list, activate, deactivate autonomous capability packages.

use sk_hands::registry::HandRegistry;

/// Run the hands subcommand.
pub async fn run(action: &str, args: &[String]) -> anyhow::Result<()> {
    let mut registry = HandRegistry::new();
    let loaded = registry.load_bundled();

    match action {
        "list" => {
            println!("⚡ Sovereign Kernel — Autonomous Hands");
            println!("══════════════════════════════════════════");
            println!();

            let defs = registry.list_definitions();
            if defs.is_empty() {
                println!("  No hands available.");
            } else {
                println!("  {} hand(s) loaded:\n", loaded);
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
            println!("  sovereign hands activate <n> — activate a hand");
            println!("  sovereign hands deactivate <id> — deactivate");
            println!("  sovereign hands status       — show active instances");
        }
    }

    Ok(())
}
