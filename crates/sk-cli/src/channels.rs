//! Channel bridge management (listing active channels, etc.)

use sk_types::KernelConfig;

pub async fn run(config: KernelConfig, action: &str, channel: Option<&str>) -> anyhow::Result<()> {
    match action {
        "list" => {
            println!("📡 Configured Channels Settings:\n");

            let mut any = false;

            if let Some(tg) = &config.channels.telegram {
                any = true;
                println!("- Telegram");
                println!("  Token Env Variable: {}", tg.bot_token_env);
                if !tg.allowed_users.is_empty() {
                    println!("  Allowed Users: {:?}", tg.allowed_users);
                }
            }
            if let Some(dc) = &config.channels.discord {
                any = true;
                println!("- Discord");
                println!("  Token Env Variable: {}", dc.bot_token_env);
                if !dc.allowed_guilds.is_empty() {
                    println!("  Allowed Guilds: {:?}", dc.allowed_guilds);
                }
            }

            if let Some(wa) = &config.channels.whatsapp {
                any = true;
                println!("- WhatsApp");
                println!("  Token Env Variable: {}", wa.access_token_env);
            }

            // Could expand later for Slack, Matrix, etc.

            if !any {
                println!("⚠️ No channels configured. Enable them in config.toml.");
            } else {
                println!("\nChannels configuration is currently loaded from KernelConfig.");
                println!("To activate them, make sure the respective environment variables are set and restart the daemon.");
            }
        }
        "info" => {
            if let Some(c) = channel {
                println!("Info for channel '{}' not fully implemented yet.", c);
            } else {
                println!("Please provide a channel name: sovereign channels info <name>");
            }
        }
        _ => {
            anyhow::bail!("Unknown action '{}'. Setup only: list, info", action);
        }
    }
    Ok(())
}
