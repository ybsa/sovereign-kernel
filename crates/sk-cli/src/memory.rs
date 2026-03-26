use anyhow::Result;
use clap::Subcommand;
use fs_err as fs;
use sk_memory::MemorySubstrate;
use sk_types::memory::{ExportFormat, Memory};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Export agent memory to a file
    Export {
        /// Format (json, markdown)
        #[clap(short, long, default_value = "markdown")]
        format: String,
        /// Output file path
        #[clap(short, long)]
        output: Option<PathBuf>,
    },
    /// Show memory statistics
    Stats,
    /// Import agent memory from a file
    Import {
        /// Format (json, markdown)
        #[clap(short, long)]
        format: Option<String>,
        /// Input file path
        #[clap(short, long)]
        input: PathBuf,
    },
}

pub async fn handle_memory_command(command: MemoryCommands) -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("sovereign");
    let db_path = config_dir.join("memory.db");

    let substrate = MemorySubstrate::open(&db_path, 0.1)?;

    match command {
        MemoryCommands::Export { format, output } => {
            let export_format = match format.to_lowercase().as_str() {
                "json" => ExportFormat::Json,
                "markdown" | "md" => ExportFormat::Markdown,
                _ => anyhow::bail!("Unsupported format: {}", format),
            };

            println!("Exporting memory in {} format...", format);

            let data = substrate.export(export_format).await?;

            let final_output = output.unwrap_or_else(|| {
                let ext = if matches!(export_format, ExportFormat::Json) {
                    "json"
                } else {
                    "md"
                };
                PathBuf::from(format!("sovereign_memory_export.{}", ext))
            });

            fs::write(&final_output, data)?;
            println!("Memory exported to {}", final_output.display());
        }
        MemoryCommands::Stats => {
            let agents = substrate.list_agents()?;
            println!("Sovereign Kernel Memory Statistics");
            println!("----------------------------------");
            println!("Agents registered: {}", agents.len());
            for agent in agents {
                println!("- {} ({})", agent.name, agent.id);
            }
            // More stats can be added by querying the DB directly if needed
        }
        MemoryCommands::Import { format, input } => {
            let data = fs::read(&input)?;
            let import_format = if let Some(fmt) = format {
                match fmt.to_lowercase().as_str() {
                    "json" => ExportFormat::Json,
                    "markdown" | "md" => ExportFormat::Markdown,
                    _ => anyhow::bail!("Unsupported format: {}", fmt),
                }
            } else {
                // Auto-detect by extension
                match input.extension().and_then(|e| e.to_str()) {
                    Some("json") => ExportFormat::Json,
                    Some("md") | Some("markdown") => ExportFormat::Markdown,
                    _ => anyhow::bail!(
                        "Could not auto-detect format from extension. Please specify --format."
                    ),
                }
            };

            println!("Importing memory from {}...", input.display());
            let report = substrate.import(&data, import_format).await?;

            println!("Memory Import Complete");
            println!("----------------------");
            println!("Memories imported: {}", report.memories_imported);
            println!("Entities imported: {}", report.entities_imported);
            println!("KV pairs imported: {}", report.kv_imported);

            if !report.errors.is_empty() {
                println!("\nWarnings/Errors ({}):", report.errors.len());
                for err in report.errors.iter().take(10) {
                    println!("- {}", err);
                }
                if report.errors.len() > 10 {
                    println!("- ... and {} more", report.errors.len() - 10);
                }
            }
        }
    }

    Ok(())
}
