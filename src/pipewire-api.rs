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

    // Load default link rules unless disabled
    if !args.no_auto_link {
        let mut all_rules = Vec::new();
        
        // Load default rules
        let default_rules = pw_api::default_link_rules::get_default_rules();
        tracing::info!("Loaded {} default link rule(s)", default_rules.len());
        all_rules.extend(default_rules);
        
        // Load rules from config files
        let config_rules = pw_api::config::load_all_link_rules();
        all_rules.extend(config_rules);
        
        tracing::info!("Total {} link rule(s) configured", all_rules.len());
        state.set_link_rules(all_rules);

        // Apply startup rules
        pw_api::link_scheduler::apply_startup_rules(state.clone()).await;

        // Start the link scheduler for periodic relinking
        let _scheduler_handle = pw_api::link_scheduler::start_link_scheduler(state.clone());
    }

    // Create router with generic, speakereq, and links endpoints
    let app = pw_api::generic::create_router(state.clone())
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
