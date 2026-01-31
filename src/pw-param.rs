use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use pw_api::{PipeWireClient, get_all_params, set_param_from_string};

#[derive(Parser)]
#[command(name = "pw-param")]
#[command(about = "Get/set PipeWire node parameters")]
struct Cli {
    /// Node name or ID (default: speakereq2x2)
    #[arg(short, long, default_value = "speakereq2x2")]
    node: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get parameter value
    Get {
        /// Parameter name
        param: String,
    },
    /// Set parameter value
    Set {
        /// Parameter name
        param: String,
        /// Parameter value (true/false for bool, number for float/int)
        value: String,
    },
    /// List all parameters
    List {
        /// Optional filter string
        #[arg(short, long)]
        filter: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create PipeWire client
    let client = PipeWireClient::new()?;

    // Find and bind the node
    let (_info, node) = client.find_and_bind_node(&cli.node, 2)?;

    // Execute command
    match &cli.command {
        Commands::Set { param, value } => {
            set_param_from_string(&node, client.mainloop(), param, value)?;
            let full_name = if param.starts_with("speakereq") {
                param.clone()
            } else {
                format!("speakereq2x2:{}", param)
            };
            println!("Set {} = {}", full_name, value);
        }
        Commands::Get { param } => {
            get_param(&node, client.mainloop(), param)?;
        }
        Commands::List { filter } => {
            list_params(&node, client.mainloop(), filter.as_deref())?;
        }
    }

    Ok(())
}

fn list_params(
    node: &pipewire::node::Node,
    mainloop: &pipewire::main_loop::MainLoopRc,
    filter: Option<&str>,
) -> Result<()> {
    let params = get_all_params(node, mainloop)?;

    let mut keys: Vec<_> = params.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(f) = filter {
            if !key.contains(f) {
                continue;
            }
        }

        if let Some(value) = params.get(key.as_str()) {
            println!("{} = {}", key, value.to_string());
        }
    }

    Ok(())
}

fn get_param(
    node: &pipewire::node::Node,
    mainloop: &pipewire::main_loop::MainLoopRc,
    param_name: &str,
) -> Result<()> {
    let params = get_all_params(node, mainloop)?;
    let full_name = if param_name.starts_with("speakereq") {
        param_name.to_string()
    } else {
        format!("speakereq2x2:{}", param_name)
    };

    let value = params
        .get(&full_name)
        .ok_or_else(|| anyhow!("Parameter '{}' not found", param_name))?;

    println!("{}", value.to_string());
    Ok(())
}
