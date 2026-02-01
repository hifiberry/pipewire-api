use anyhow::Result;
use pw_api::{PipeWireClient, AppState};
use std::sync::Arc;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "pipewire-api")]
#[command(about = "REST API server for SpeakerEQ 2x2 PipeWire plugin", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 2716)]
    port: u16,

    /// Bind to localhost only (127.0.0.1) instead of all interfaces (0.0.0.0)
    #[arg(long)]
    localhost: bool,

    /// Disable automatic link management
    #[arg(long)]
    no_auto_link: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Verify node exists
    let client = PipeWireClient::new()?;
    let (info, _node) = client.find_and_bind_node("speakereq2x2", 2)?;
    
    tracing::info!("Found speakereq2x2 node (id: {})", info.id);

    // Create shared state (just the node name, we'll reconnect per request)
    let state = Arc::new(AppState::new(info.name));

    // Load PipeWire object cache on startup
    if let Err(e) = state.refresh_object_cache() {
        tracing::warn!("Failed to load object cache on startup: {}", e);
    }

    // Load and apply volume rules on startup
    let volume_rules = pw_api::config::load_all_volume_rules();
    if !volume_rules.is_empty() {
        tracing::info!("Applying {} volume rule(s)", volume_rules.len());
        if let Err(e) = pw_api::volume::apply_volume_rules(volume_rules) {
            tracing::error!("Failed to apply volume rules: {}", e);
        }
    }

    // Load link rules unless disabled
    if !args.no_auto_link {
        // Load rules from config files (user config takes precedence over system config)
        let mut all_rules = pw_api::config::load_all_link_rules();
        
        // If no rules were loaded from config files, use hardcoded defaults
        if all_rules.is_empty() {
            tracing::info!("No config files found, using hardcoded default rules");
            let default_rules = pw_api::default_link_rules::get_default_rules();
            tracing::info!("Loaded {} hardcoded default rule(s)", default_rules.len());
            all_rules.extend(default_rules);
        }
        
        tracing::info!("Total {} link rule(s) configured", all_rules.len());
        state.set_link_rules(all_rules);

        // Apply startup rules
        pw_api::link_scheduler::apply_startup_rules(state.clone()).await;

        // Start the link scheduler for periodic relinking
        let _scheduler_handle = pw_api::link_scheduler::start_link_scheduler(state.clone());
    }

    // Create router with api, speakereq, and links endpoints
    let app = pw_api::api::create_router(state.clone())
        .merge(pw_api::speakereq::create_router(state.clone()))
        .merge(pw_api::links::create_router(state));

    // Bind to localhost or all interfaces
    let host = if args.localhost { "127.0.0.1" } else { "0.0.0.0" };
    let addr = format!("{}:{}", host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on http://{}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
