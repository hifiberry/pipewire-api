use anyhow::Result;
use clap::{Parser, Subcommand};
use pw_api::{PipeWireClient, default_link_rules, apply_link_rule};

#[derive(Parser, Debug)]
#[command(name = "pw-link")]
#[command(about = "PipeWire link management tool", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Apply the default link rules
    ApplyDefaults {
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ApplyDefaults { verbose } => {
            apply_default_rules(verbose)?;
        }
    }

    Ok(())
}

fn apply_default_rules(verbose: bool) -> Result<()> {
    // Get default rules
    let rules = default_link_rules::get_default_rules();
    
    if verbose {
        println!("Loaded {} default rule(s)", rules.len());
    }

    // Create PipeWire client
    let client = PipeWireClient::new()?;
    
    if verbose {
        println!("Connected to PipeWire");
    }

    let mut successful = 0;
    let mut failed = 0;

    // Apply each rule
    for (idx, rule) in rules.iter().enumerate() {
        if verbose {
            println!("\nApplying rule {}/{}:", idx + 1, rules.len());
            if let Some(ref name) = rule.source.node_name {
                println!("  Source (node.name): {}", name);
            }
            if let Some(ref nick) = rule.source.node_nick {
                println!("  Source (node.nick): {}", nick);
            }
            if let Some(ref path) = rule.source.object_path {
                println!("  Source (object.path): {}", path);
            }
            if let Some(ref name) = rule.destination.node_name {
                println!("  Destination (node.name): {}", name);
            }
            if let Some(ref nick) = rule.destination.node_nick {
                println!("  Destination (node.nick): {}", nick);
            }
            if let Some(ref path) = rule.destination.object_path {
                println!("  Destination (object.path): {}", path);
            }
            println!("  Action: {:?}", rule.link_type);
        }

        match apply_link_rule(client.registry(), client.core(), client.mainloop(), rule) {
            Ok(results) => {
                let rule_success = results.iter().all(|r| r.success);
                if rule_success {
                    successful += 1;
                } else {
                    failed += 1;
                }
                
                for result in results {
                    if verbose || !result.success {
                        let prefix = if result.success { "  ✓" } else { "  ✗" };
                        println!("{} {}", prefix, result.message);
                    } else {
                        println!("{}", result.message);
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("  ✗ Failed: {}", e);
            }
        }
    }

    // Print summary
    println!("\nSummary: {} successful, {} failed", successful, failed);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}
